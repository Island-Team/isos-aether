"""EI main entry point.

Thin composition root: configures logging, resolves external EOS endpoints,
and starts the decision pipeline.
"""

from __future__ import annotations

import argparse
import asyncio
import contextlib
import json
import os
import re
import signal
import sys
import threading
from datetime import UTC, datetime
from pathlib import Path

_SRC = Path(__file__).resolve().parent.parent
if str(_SRC) not in sys.path:
  sys.path.insert(0, str(_SRC))

from ei.contracts.source import configure as configure_contracts
from ei.contracts.source import schema_registry
from ei.core import launch as launch_core
from ei.core import launch_empty
from ei.core.link import EosClient
from ei.internal.backend import BackendSessionHandoff
from ei.internal.recall import RecallIndex
from ei.io.northbound import NorthboundBindError
from ei.lib import shared_events
from ei.lib.config import (
  BackendEndpointConfig,
  GatewayConfig,
  LaunchConfig,
  load_launch_config,
  load_sandbox_config,
  resolve_etc_root,
)
from ei.lib.logger import (
  _safe_exception_summary,
  bind_correlation_id,
  configure_logging,
  get_logger,
)
from ei.lib.safety_state import (
  ResetAuthorization,
  SafetyState,
  SafetyStateError,
  runtime_lock,
)
from ei.stages import load_stage_runtime
from island.path import Layout
from island.share import configured_resources, require_eos_contracts


def _ei_log_dir(layout: Layout) -> Path:
  """Select the deployment's diagnostic root without changing other EI data."""
  override = os.environ.get("EI_LOG_DIR")
  if override:
    return Path(override).expanduser()
  deployment = os.environ.get("EI_DEPLOYMENT", "local")
  if not re.fullmatch(r"[a-zA-Z0-9][a-zA-Z0-9_-]{0,63}", deployment):
    raise ValueError("EI_DEPLOYMENT must be a simple deployment name")
  return layout.component_log("ei") / deployment


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
  """Parse launch arguments."""
  public_name = "ei"
  description = "EI decision runtime"
  parser = argparse.ArgumentParser(prog=public_name, description=description)
  parser.add_argument(
    "--etc",
    default=None,
    help="EI configuration root (default: EI_ETC, then root config/ei in source mode)",
  )
  parser.add_argument(
    "--island-config",
    default=None,
    help="product configuration selecting shared resources (default: island.toml)",
  )
  parser.add_argument(
    "--state-root",
    default=None,
    help="durable state root containing component namespaces",
  )
  parser.add_argument(
    "--cache-root",
    default=None,
    help="rebuildable cache root containing component namespaces",
  )
  parser.add_argument(
    "--log-root",
    default=None,
    help="diagnostic log root containing component namespaces",
  )
  parser.add_argument("--run-root", default=None, help="process-lifetime runtime root")
  parser.add_argument("--tmp-root", default=None, help="disposable scratch root")
  parser.add_argument(
    "--log-level",
    default=None,
    choices=["DEBUG", "INFO", "WARNING", "ERROR"],
    help="minimum log level (default: [logging] level in config/ei/ei.toml)",
  )
  parser.add_argument(
    "--mode",
    default=None,
    help="launch mode preset from config/ei/modes.toml (default: modes.toml default)",
  )
  parser.add_argument(
    "--no-tui",
    action="store_true",
    help="run without the operator console (accepted here for pass-through)",
  )
  parser.add_argument(
    "--health-only",
    action="store_true",
    help="run the managed EI process without pipeline or dataflow",
  )
  parser.add_argument(
    "--managed",
    action="store_true",
    help="run under IsOS ownership; reject an internally launched sandbox",
  )
  parser.add_argument(
    "--use-proxy",
    action="store_true",
    help=(
      "honor HTTP_PROXY/HTTPS_PROXY/ALL_PROXY from the environment "
      "(default: proxy vars are stripped — see strip_proxy_env)"
    ),
  )
  return parser.parse_args(argv)


def validate_managed_config(config: LaunchConfig, *, managed: bool) -> None:
  """Refuse hidden child backends when IsOS owns the deployment graph."""
  if managed and config.launch.sandbox:
    raise ValueError(
      "managed EI requires [launch] sandbox = false; configure the sandbox "
      "as a separate IsOS backend node"
    )


_PROXY_VARS = (
  "HTTP_PROXY",
  "HTTPS_PROXY",
  "ALL_PROXY",
  "http_proxy",
  "https_proxy",
  "all_proxy",
)


def strip_proxy_env() -> None:
  """Remove proxy env vars so loopback clients never route through them.

  Local EOS and model endpoints must not inherit a desktop HTTP proxy. EI
  therefore strips common proxy variables by default; pass ``--use-proxy``
  when remote model endpoints require them.
  """
  for name in _PROXY_VARS:
    os.environ.pop(name, None)


def _resolve_docs_corpus(config: LaunchConfig, layout: Layout) -> LaunchConfig:
  """Inject an absolute EI documentation corpus into launch config."""
  raw = config.memory.docs_corpus
  if not raw:
    return config
  corpus = Path(raw).expanduser()
  if not corpus.is_absolute():
    parts = corpus.parts
    if parts[:3] == ("share", "doc", "ei"):
      corpus = Path(*parts[3:])
    elif parts[:1] == ("docs",):
      corpus = Path(*parts[1:])
    corpus = layout.doc / "ei" / corpus
  memory = config.memory.model_copy(update={"docs_corpus": str(corpus.resolve())})
  return config.model_copy(update={"memory": memory})


def build_recall_index(config: LaunchConfig, cache_dir: Path) -> RecallIndex | None:
  """Build the §3.2 sqlite-vec recall index, or ``None`` when unavailable.

  The only layer permitted to import ``opt/`` by name. A missing package or
  extension — or ``vector_recall = false`` — degrades to keyword recall with
  one INFO log; startup never depends on the opt package being present.
  """
  log = get_logger("ei.main")
  if not config.launch.memory:
    # [launch] memory = false (§3.2): the master switch is off, so no vec.db
    # is even attempted — the composition root opens no memory stores at all.
    return None
  if not config.memory.vector_recall:
    return None
  try:
    from ei.opt.memory import SqliteVecIndex

    return SqliteVecIndex(cache_dir / "memory" / "vec.db")
  except Exception as exc:
    log.info("vector_recall_unavailable", error=str(exc))
    return None


async def start_sandbox(
  config: LaunchConfig,
  log_dir: Path,
  *,
  etc: str | None = None,
  defaults_root: Path | None = None,
  media_socket: Path | None = None,
  tmp_dir: Path | None = None,
) -> asyncio.subprocess.Process | None:
  """Start the optional playground without making it core-critical."""
  if not config.launch.sandbox:
    return None
  log = get_logger("ei.main")
  log_path = log_dir / "ei.jsonl"
  media_socket = media_socket or log_dir / "sandbox-media.sock"
  runtime = load_sandbox_config(etc, defaults=defaults_root)
  try:
    log_dir.mkdir(parents=True, exist_ok=True)
    command = [sys.executable, "-m", "ei.opt.sandbox"]
    if etc is not None:
      command.extend(["--etc", etc])
    if defaults_root is not None:
      command.extend(["--defaults", str(defaults_root)])
    command.extend(["--log-dir", str(log_dir)])
    command.extend(["--tmp-dir", str(tmp_dir or log_dir.parent / "tmp")])
    if media_socket is not None:
      command.extend(["--media-socket", str(media_socket)])
    process = await asyncio.create_subprocess_exec(
      *command,
      stdin=asyncio.subprocess.DEVNULL,
      stdout=asyncio.subprocess.DEVNULL,
      # Keep the structured console stream in the owning process group. In
      # TUI wrap mode this is the already-captured stderr pipe.
      stderr=None,
    )
  except Exception as exc:
    log.exception("sandbox_unavailable", error=str(exc), log=str(log_path))
    return None
  readiness = EosClient(
    GatewayConfig(host=runtime.gateway.host, grpc_port=runtime.gateway.grpc_port),
    backend="sandbox",
  )
  ready = False
  try:
    for _ in range(50):
      if process.returncode is not None:
        break
      try:
        await readiness.initialize(force=True)
      except Exception:
        await asyncio.sleep(0.1)
      else:
        ready = True
        break
  finally:
    await readiness.aclose()
  if not ready:
    log.error("sandbox_unavailable", error="readiness timeout", log=str(log_path))
    await stop_sandbox(process)
    return None
  log.info("sandbox_started", pid=process.pid, log=str(log_path))
  return process


async def stop_sandbox(process: asyncio.subprocess.Process | None) -> None:
  """Terminate and reap the optional sandbox child."""
  if process is None:
    return
  if process.returncode is not None:
    logger = get_logger("ei.main")
    if process.returncode == 0:
      logger.info("sandbox_stopped", pid=process.pid, returncode=process.returncode)
    else:
      logger.error("sandbox_exited", pid=process.pid, returncode=process.returncode)
    return
  with contextlib.suppress(ProcessLookupError):
    process.terminate()
  try:
    await asyncio.wait_for(process.wait(), timeout=5.0)
  except TimeoutError:
    process.kill()
    await process.wait()
  get_logger("ei.main").info(
    "sandbox_stopped", pid=process.pid, returncode=process.returncode
  )


async def _supervise_sandbox(
  config: LaunchConfig,
  log_dir: Path,
  holder: list[asyncio.subprocess.Process | None],
  stop: asyncio.Event,
  *,
  etc: str | None,
  defaults_root: Path | None,
  media_socket: Path,
  tmp_dir: Path,
) -> None:
  """Restart an enabled sandbox child without coupling failure to core."""
  backoff = (1.0, 2.0, 5.0, 10.0)
  failures = 0
  while not stop.is_set():
    process = holder[0]
    if process is not None:
      exited = asyncio.create_task(process.wait())
      stopping = asyncio.create_task(stop.wait())
      done, pending = await asyncio.wait(
        {exited, stopping}, return_when=asyncio.FIRST_COMPLETED
      )
      for task in pending:
        task.cancel()
      for task in pending:
        with contextlib.suppress(asyncio.CancelledError):
          await task
      if stopping in done:
        return
      get_logger("ei.main").error(
        "sandbox_exited", pid=process.pid, returncode=process.returncode
      )
    delay = backoff[min(failures, len(backoff) - 1)]
    failures += 1
    try:
      await asyncio.wait_for(stop.wait(), timeout=delay)
      return
    except TimeoutError:
      pass
    holder[0] = await start_sandbox(
      config,
      log_dir,
      etc=etc,
      defaults_root=defaults_root,
      media_socket=media_socket,
      tmp_dir=tmp_dir,
    )


async def _probe_real_backends(config: LaunchConfig) -> bool:
  """Return true only when every configured real EOS endpoint discovers."""
  clients = [
    EosClient(endpoint.gateway, backend=endpoint.id) for endpoint in config.endpoints()
  ]
  try:
    results = await asyncio.gather(
      *(client.initialize(force=True) for client in clients), return_exceptions=True
    )
    return all(not isinstance(result, BaseException) for result in results)
  finally:
    await asyncio.gather(*(client.aclose() for client in clients))


async def _wait_for_real_backends(
  config: LaunchConfig, *, timeout_s: float | None, retry_s: float
) -> bool:
  """Continuously probe the mandatory real fleet, optionally with a deadline."""
  deadline = (
    None if timeout_s is None else asyncio.get_running_loop().time() + timeout_s
  )
  attempts = 0
  while True:
    remaining = (
      None if deadline is None else deadline - asyncio.get_running_loop().time()
    )
    if remaining is not None and remaining <= 0:
      if attempts == 0:
        return False
      # The retry scheduled exactly at the grace boundary must still observe
      # a fleet that appeared during the window. Bound this final discovery
      # so an unreachable endpoint cannot materially extend fallback.
      try:
        return await asyncio.wait_for(
          _probe_real_backends(config), timeout=min(retry_s, 1.0)
        )
      except TimeoutError:
        return False
    try:
      probe = _probe_real_backends(config)
      ready = (
        await probe if remaining is None else await asyncio.wait_for(probe, remaining)
      )
    except TimeoutError:
      return False
    if ready:
      return True
    attempts += 1
    delay = retry_s if remaining is None else min(retry_s, max(remaining, 0.0))
    if delay <= 0:
      return False
    await asyncio.sleep(delay)


def _playground_config(config: LaunchConfig, host: str, port: int) -> LaunchConfig:
  """Replace the unavailable real fleet with the single sandbox alias."""
  gateway = GatewayConfig(host=host, grpc_port=port)
  return config.model_copy(
    update={
      "mode": "sandbox",
      "gateway": gateway,
      "backends": [BackendEndpointConfig(id="sandbox", gateway=gateway)],
    }
  )


async def _run_unbound(
  args: argparse.Namespace,
  config: LaunchConfig,
  *,
  layout: Layout,
  stage_run_id: str | None = None,
  reset_authorization: ResetAuthorization | None = None,
) -> int:
  """Run the decision pipeline with composition-root-resolved mutable paths."""
  log = get_logger("ei.main")
  stage_runtime = (
    load_stage_runtime(Path(args.etc), stage_run_id)
    if getattr(args, "managed", False) and stage_run_id is not None
    else None
  )
  state_dir = layout.component_state("ei")
  cache_dir = layout.component_cache("ei")
  log_dir = _ei_log_dir(layout)
  tmp_dir = layout.component_tmp("ei")
  with bind_correlation_id() as cid:
    log.info(
      "ei_launch",
      etc=args.etc,
      state=str(state_dir),
      cache=str(cache_dir),
      log=str(log_dir),
      run=str(layout.component_run("ei")),
      tmp=str(tmp_dir),
      log_level=config.logging.level,
      correlation_id=cid,
    )
    log.info(
      "config_resolved",
      mode=config.mode,
      server=f"{config.server.host}:{config.server.port}",
      gateway=config.gateway.endpoint,
      backend_enabled=config.launch.backend,
      sandbox_enabled=config.launch.sandbox,
      operator_primitives=config.primitives.expose_operator,
    )
    defaults_root = layout.ei_defaults
    sandbox_defaults = defaults_root if defaults_root.is_dir() else None
    sandbox_media_socket = layout.component_run("ei") / "ei-sandbox-media.sock"
    sandbox = await start_sandbox(
      config,
      log_dir,
      etc=args.etc,
      defaults_root=sandbox_defaults,
      media_socket=sandbox_media_socket,
      tmp_dir=layout.component_tmp("ei"),
    )
    sandbox_holder = [sandbox]
    sandbox_supervisor_stop = asyncio.Event()
    sandbox_supervisor = (
      asyncio.create_task(
        _supervise_sandbox(
          config,
          log_dir,
          sandbox_holder,
          sandbox_supervisor_stop,
          etc=args.etc,
          defaults_root=sandbox_defaults,
          media_socket=sandbox_media_socket,
          tmp_dir=layout.component_tmp("ei"),
        )
      )
      if config.launch.sandbox and (sandbox is None or hasattr(sandbox, "wait"))
      else None
    )
    try:
      if not config.launch.backend and not config.launch.sandbox:
        return await launch_empty(config, state_dir=state_dir)

      async def _launch(
        selected: LaunchConfig,
        *,
        stop: asyncio.Event | None = None,
        handoff: BackendSessionHandoff | None = None,
      ) -> int:
        nonlocal reset_authorization
        selected_reset = reset_authorization
        reset_authorization = None
        return await launch_core(
          selected,
          stage_runtime=stage_runtime,
          managed=bool(getattr(args, "managed", False)),
          reset_authorization=selected_reset,
          reset_checked=True,
          state_dir=state_dir,
          etc=args.etc,
          defaults_root=defaults_root,
          recall_index=build_recall_index(selected, cache_dir),
          external_stop=stop,
          session_handoff=handoff,
          sandbox_media_socket=(
            str(sandbox_media_socket) if config.launch.sandbox else None
          ),
        )

      if not config.launch.backend:
        if sandbox is None:
          log.warning("backend_fallback_to_empty", reason="sandbox unavailable")
          return await launch_empty(
            config,
            state_dir=state_dir,
            sandbox_media_socket=str(sandbox_media_socket),
          )
        sandbox_config = load_sandbox_config(args.etc, defaults=sandbox_defaults)
        return await _launch(
          _playground_config(
            config,
            sandbox_config.gateway.host,
            sandbox_config.gateway.grpc_port,
          )
        )
      if not config.launch.sandbox:
        return await _launch(config)

      sandbox_config = load_sandbox_config(args.etc, defaults=sandbox_defaults)
      if await _wait_for_real_backends(
        config,
        timeout_s=sandbox_config.fallback_timeout_s,
        retry_s=sandbox_config.backend_retry_interval_s,
      ):
        return await _launch(config)

      log.warning("backend_fallback_to_playground")
      playground_stop = asyncio.Event()
      handoff = BackendSessionHandoff()
      playground = asyncio.create_task(
        _launch(
          _playground_config(
            config,
            sandbox_config.gateway.host,
            sandbox_config.gateway.grpc_port,
          ),
          stop=playground_stop,
          handoff=handoff,
        )
      )
      promotion = asyncio.create_task(
        _wait_for_real_backends(
          config,
          timeout_s=None,
          retry_s=sandbox_config.backend_retry_interval_s,
        )
      )
      done, _ = await asyncio.wait(
        {playground, promotion}, return_when=asyncio.FIRST_COMPLETED
      )
      if playground in done:
        promotion.cancel()
        with contextlib.suppress(asyncio.CancelledError):
          await promotion
        return playground.result()
      log.info("backend_promoting_from_playground")
      playground_stop.set()
      await playground
      # Promotion is deliberately one-way for this process lifetime. The real
      # pipeline's availability gate handles later outages without failback.
      return await _launch(config, handoff=handoff)
    finally:
      sandbox_supervisor_stop.set()
      if sandbox_supervisor is not None:
        await sandbox_supervisor
      await stop_sandbox(sandbox_holder[0])


async def run(
  args: argparse.Namespace,
  config: LaunchConfig,
  *,
  layout: Layout,
  stage_run_id: str | None = None,
  reset_authorization: ResetAuthorization | None = None,
  stale_test_opt_in: bool = False,
) -> int:
  """Launch optional managed Vision owner service beside the stage pipeline."""
  from ei.vision_evidence import start_managed_vision

  vision = None
  if getattr(args, "managed", False) and stage_run_id is not None:
    vision = await start_managed_vision(
      run_id=stage_run_id, etc=Path(args.etc), stale_test_opt_in=stale_test_opt_in
    )
  try:
    return await _run_unbound(
      args,
      config,
      layout=layout,
      stage_run_id=stage_run_id,
      reset_authorization=reset_authorization,
    )
  finally:
    if vision is not None:
      evidence, inspection = vision
      await evidence.stop()
      await inspection.stop()


def load_dotenv(etc: str | None, layout: Layout) -> list[tuple[Path, list[str]]]:
  """Source the product ``.env`` before loading EI TOML configuration.

  The file is a fallback — variables already exported in the environment win
  (see :mod:`ei.lib.dotenv`). Returns only variable names, so the caller can
  report structured post-configuration diagnostics without printing secrets or
  raw stderr lines into the TUI.
  """
  from ei.lib.dotenv import load_env_file

  candidate = (
    layout.prefix.parent if layout.is_source_ei_checkout else layout.prefix
  ) / ".env"
  loaded = load_env_file(candidate)
  return [(candidate, sorted(loaded))] if loaded else []


def main(argv: list[str] | None = None) -> int:
  """Entry point: resolve config, configure logging, run the async pipeline."""
  args = parse_args(argv)
  layout = Layout.resolve(
    state=args.state_root,
    cache=args.cache_root,
    log=args.log_root,
    run=args.run_root,
    tmp=args.tmp_root,
  )
  if args.health_only:
    if not args.managed:
      raise ValueError("health-only mode requires managed ownership")
    # Owner-side graph data pass has no worker safety state. A deployment's
    # liveness child must not contend with another deployment's runtime lock.
    stopped = threading.Event()
    signal.signal(signal.SIGINT, lambda _signum, _frame: stopped.set())
    signal.signal(signal.SIGTERM, lambda _signum, _frame: stopped.set())
    stopped.wait()
    return 0
  try:
    with runtime_lock(layout.component_state("ei") / "safety"):
      return _run_locked(args, layout)
  except SafetyStateError:
    sys.stderr.write("EI safety state unavailable or runtime already active\n")
    return 1


def _run_locked(args: argparse.Namespace, layout: Layout) -> int:
  """Hold the durable owner lock from bootstrap through terminal event."""
  stale_test_opt_in = os.environ.get("EI_Q01_STALE_TEST") == "1"
  # Consume any authorization before config, discovery or checkpoint parsing.
  # An early startup failure cannot leave it armed for a later launch.
  reset_authorization = SafetyState(
    layout.component_state("ei") / "safety"
  ).consume_reset()
  try:
    run_id = shared_events.select_run_id(args.managed, os.environ.get("ISOS_RUN_ID"))
  except ValueError:
    # Before the event sink exists there is no authenticated deployment run.
    diagnostic = _ei_log_dir(layout) / "ei.jsonl"
    diagnostic.parent.mkdir(parents=True, exist_ok=True)
    with diagnostic.open("a", encoding="utf-8") as stream:
      stream.write(
        json.dumps(
          {
            "timestamp": datetime.now(UTC)
            .isoformat(timespec="milliseconds")
            .replace("+00:00", "Z"),
            "level": "error",
            "source": "ei.main.py@aether-v0.2.1",
            "message": "Invalid run binding",
          }
        )
        + "\n"
      )
    sys.stderr.write("EI run binding invalid\n")
    return 2
  log_dir = _ei_log_dir(layout)
  try:
    if args.island_config is not None:
      selected_product_config = Path(args.island_config).expanduser()
      if not selected_product_config.is_absolute():
        selected_product_config = layout.prefix / selected_product_config
      os.environ["ISLAND_CONFIG"] = str(selected_product_config.resolve())
    shared = configured_resources(layout, args.island_config)
    require_eos_contracts(shared)
    configure_contracts(shared)
    schema_registry()
    dotenv_files = load_dotenv(args.etc, layout)
    args.etc = str(resolve_etc_root(args.etc, layout=layout))
    if not args.use_proxy:
      strip_proxy_env()
    config = _resolve_docs_corpus(
      load_launch_config(
        etc=args.etc,
        mode=args.mode,
        defaults=layout.ei_defaults,
      ),
      layout,
    )
    validate_managed_config(config, managed=args.managed)
    level = args.log_level or config.logging.level
    configure_logging(
      log_dir=log_dir,
      config=config.logging,
      replay_dir=layout.component_state("ei"),
      level=level,
      run_id=run_id,
      managed=args.managed,
    )
  except Exception:
    _write_bootstrap_error(log_dir / "ei.jsonl")
    if args.managed:
      return 1
    raise
  log = get_logger("ei.main")
  shared_events.emit("ei_starting")
  for path, variables in dotenv_files:
    log.info("dotenv_loaded", path=str(path), variables=variables)

  try:
    result = asyncio.run(
      run(
        args,
        config,
        layout=layout,
        stage_run_id=run_id,
        reset_authorization=reset_authorization,
        stale_test_opt_in=stale_test_opt_in,
      )
    )
    shared_events.flush()
    if result == 0:
      shared_events.emit(
        "ei_stopped",
        fields={
          "state": "stop_unconfirmed" if shared_events.stop_unconfirmed() else "stopped"
        },
      )
    elif not shared_events.ready_emitted():
      shared_events.emit("ei_start_failed", fields={"reason_code": "internal"})
    return result
  except NorthboundBindError as exc:
    shared_events.flush()
    if not shared_events.ready_emitted():
      shared_events.emit(
        "ei_start_failed", fields={"reason_code": "listener_unavailable"}
      )
    log.error(
      "northbound_listener_unavailable",
      host=exc.host,
      port=exc.port,
      reason="address_in_use",
    )
    return 1
  except Exception:
    shared_events.flush()
    if not shared_events.ready_emitted():
      shared_events.emit("ei_start_failed", fields={"reason_code": "internal"})
    log.exception("ei_unhandled_error")
    if args.managed:
      return 1
    raise


def _write_bootstrap_error(path: Path) -> None:
  """Persist an exception raised before structured logging is available."""
  path.parent.mkdir(parents=True, exist_ok=True)
  with path.open("a", encoding="utf-8") as stream:
    cause = _safe_exception_summary(sys.exc_info()[1])
    stream.write(
      json.dumps(
        {
          "timestamp": datetime.now(UTC)
          .isoformat(timespec="milliseconds")
          .replace("+00:00", "Z"),
          "level": "error",
          "source": "ei.main.py@aether-v0.2.1",
          "message": "EI startup failed",
          "cause": cause,
        },
        ensure_ascii=False,
      )
      + "\n"
    )


if __name__ == "__main__":
  raise SystemExit(main())
