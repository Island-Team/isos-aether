"""Sanitized EI logging into one local core JSONL file.

The local ``data/ei/log/<deployment>/ei.jsonl`` file is the diagnostic source
read by IsOS. Standalone EI may additionally show lifecycle output on stderr.
Every JSONL line shares timestamp, level, source, and message fields; optional
context is bounded and redacted before it reaches either sink.
"""

from __future__ import annotations

import json
import logging
import os
import re
import sys
import threading
import time
import uuid
from collections import OrderedDict
from collections.abc import Iterator, Mapping, Sequence
from contextlib import contextmanager
from contextvars import ContextVar
from copy import copy
from pathlib import Path

import structlog
from structlog.types import EventDict, WrappedLogger

from ei.lib.config import LoggingConfig

__all__ = [
  "bind_correlation_id",
  "bind_log_context",
  "configure_logging",
  "get_logger",
  "log_once",
  "restore_console_level",
  "show_lifecycle_logs",
]

_configured = False
_console_handler: logging.Handler | None = None
# Configured ``[logging.console] level`` — the runtime target that
# :func:`restore_console_level` returns to once the pipeline is ready.
_console_config_level: str = "INFO"
_logged_once: set[str] = set()

_correlation_id: ContextVar[str | None] = ContextVar("ei_correlation_id", default=None)
_log_context: ContextVar[dict[str, str] | None] = ContextVar(
  "ei_log_context", default=None
)

_CONTEXT_FIELDS = frozenset(
  {
    "run_id",
    "correlation_id",
    "mission_id",
    "step_id",
    "backend",
    "command_id",
    "execution_id",
  }
)
_SECRET_KEYS = frozenset(
  {
    "api_key",
    "apikey",
    "authorization",
    "cookie",
    "credential",
    "credentials",
    "passwd",
    "password",
    "secret",
    "token",
    "access_token",
    "refresh_token",
  }
)
_EXCLUDED_KEYS = frozenset(
  {
    "arguments",
    "audio",
    "body",
    "completion",
    "image",
    "images",
    "media",
    "payload",
    "prompt",
    "prompts",
    "provider_payload",
    "raw_provider_payload",
    "request",
    "response",
    "result",
    "transcript",
  }
)
_EXCLUDED_KEY_PARTS = frozenset(
  {"arguments", "audio", "image", "media", "payload", "prompt", "transcript"}
)
_MAX_VALUE_CHARS = 1024
_MAX_COLLECTION_ITEMS = 32
_MAX_VALUE_DEPTH = 6
_EVENT_CODE = re.compile(r"[a-z][a-z0-9_]{0,63}\Z")


def _inject_log_context(
  logger: WrappedLogger, method: str, event_dict: EventDict
) -> EventDict:
  event_dict.update(_log_context.get() or {})
  cid = _correlation_id.get()
  if cid is not None:
    event_dict["correlation_id"] = cid
  return event_dict


def _safe_value(value: object, *, key: str = "", depth: int = 0) -> object:
  """Return a bounded, JSON-safe diagnostic representation of ``value``."""
  normalized_key = key.lower().replace("-", "_")
  key_parts = set(normalized_key.split("_"))
  if normalized_key in _EXCLUDED_KEYS or key_parts & _EXCLUDED_KEY_PARTS:
    return "<excluded>"
  if normalized_key in _SECRET_KEYS or key_parts & _SECRET_KEYS:
    return "<redacted>"
  if depth >= _MAX_VALUE_DEPTH:
    return "<max-depth>"
  if isinstance(value, bytes | bytearray | memoryview):
    return {"type": "bytes", "bytes": len(value)}
  if isinstance(value, str):
    lowered = value.lower()
    if lowered.startswith(("http://", "https://", "ws://", "wss://", "data:")):
      return "<redacted-url>"
    if len(value) > _MAX_VALUE_CHARS:
      return value[:_MAX_VALUE_CHARS] + "…<truncated>"
    return value
  if isinstance(value, Mapping):
    items = list(value.items())
    safe = {
      str(item_key): _safe_value(item, key=str(item_key), depth=depth + 1)
      for item_key, item in items[:_MAX_COLLECTION_ITEMS]
    }
    if len(items) > _MAX_COLLECTION_ITEMS:
      safe["_truncated_fields"] = len(items) - _MAX_COLLECTION_ITEMS
    return safe
  if isinstance(value, Sequence) and not isinstance(value, str):
    items = list(value)
    safe_items = [
      _safe_value(item, depth=depth + 1) for item in items[:_MAX_COLLECTION_ITEMS]
    ]
    if len(items) > _MAX_COLLECTION_ITEMS:
      safe_items.append(f"<truncated:{len(items) - _MAX_COLLECTION_ITEMS}>")
    return safe_items
  if value is None or isinstance(value, bool | int | float):
    return value
  return _safe_value(str(value), key=key, depth=depth)


def _safe_exception_summary(error: BaseException | None) -> str:
  """Bound and redact an early error before structured logging exists."""
  if error is None:
    return "unknown error"
  message = str(error)
  message = re.sub(
    r"(?i)\b(api[_-]?key|authorization|cookie|credential|password|passwd|secret|token)\s*[:=]\s*([^\s,;]+)",
    lambda match: f"{match.group(1)}=<redacted>",
    message,
  )
  message = re.sub(r"(?i)\b(?:https?|wss?)://\S+", "<redacted-url>", message)
  safe = _safe_value(message, key="cause")
  return f"{type(error).__name__}: {safe}".replace("\r", "\\r").replace("\n", "\\n")


def _sanitize_event(
  logger: WrappedLogger, method: str, event_dict: EventDict
) -> EventDict:
  """Redact secrets and payloads before any configured sink sees an event."""
  return {
    str(key): value
    if key in ("_record", "_from_structlog")
    else _safe_value(value, key=str(key))
    for key, value in event_dict.items()
  }


_SHARED_PROCESSORS = [
  structlog.contextvars.merge_contextvars,
  _inject_log_context,
  structlog.processors.add_log_level,
  structlog.stdlib.add_logger_name,
  # Foreign stdlib records must retain _record until add_logger_name and the
  # formatter's remove_processors_meta read it; only user fields are sanitized.
  _sanitize_event,
  structlog.processors.TimeStamper(fmt="iso", utc=True),
  structlog.processors.StackInfoRenderer(),
  structlog.processors.format_exc_info,
]

_PREFIX = "aether-ei"

# Console colorization switch, resolved once at import: on when stderr is a
# TTY, ``FORCE_COLOR`` forces it on, ``NO_COLOR`` forces it off (the latter
# wins per https://no-color.org).
_CONSOLE_COLOR = ("FORCE_COLOR" in os.environ or sys.stderr.isatty()) and (
  "NO_COLOR" not in os.environ
)


def _fmt_extra(value: object) -> str:
  """Render one already-sanitized extra value for a human-readable sink."""
  return str(value).replace("\r\n", "\\n").replace("\r", "\\n").replace("\n", "\\n")


# --- ANSI color -----------------------------------------------------------
# Console output is colorized only when stderr is a TTY. ``NO_COLOR`` disables
# color unconditionally (https://no-color.org); ``FORCE_COLOR`` enables it
# (e.g. for captured output that a TTY-aware pager renders). Log files always
# use the plain renderer regardless of these settings.


def _render_file(logger: WrappedLogger, method: str, event_dict: EventDict) -> str:
  """Render one readable diagnostic line with its useful identity first."""
  event = event_dict.pop("event", "")
  level = event_dict.pop("level", "?").upper()
  name = event_dict.pop("logger", "-")
  timestamp = event_dict.pop("timestamp", "-")
  identities = ("run_id", "correlation_id", "stage", "cause")
  ordered = [(key, event_dict.pop(key)) for key in identities if key in event_dict]
  ordered.extend(sorted(event_dict.items()))
  extras = " ".join(f"{k}={_fmt_extra(v)}" for k, v in ordered)
  suffix = f" {extras}" if extras else ""
  code = _fmt_extra(event)
  return (
    f"{timestamp} {level} ei/{_fmt_extra(name)} {code} {code.replace('_', ' ')}{suffix}"
  )


# SGR parameter codes by short name; ``None`` renders plain text.
_SGR_CODES: dict[str, str | None] = {
  "bold": "1",
  "dim": "2",
  "red": "31",
  "green": "32",
  "yellow": "33",
  "blue": "34",
  "magenta": "35",
  "cyan": "36",
}


def _sgr(text: str, style: str) -> str:
  """Wrap ``text`` in an ANSI SGR sequence for ``style`` (no-op off-TTY)."""
  code = _SGR_CODES.get(style)
  return f"\x1b[{code}m{text}\x1b[0m" if code else text


_LEVEL_SGR: dict[str, str] = {
  "DEBUG": "dim",
  "INFO": "green",
  "WARNING": "yellow",
  "ERROR": "red",
  "CRITICAL": "red",
}


def _render_console(logger: WrappedLogger, method: str, event_dict: EventDict) -> str:
  """Render a colorized line for the terminal; falls back to plain off-TTY."""
  if not _CONSOLE_COLOR:
    return _render_file(logger, method, event_dict)
  event = event_dict.pop("event", "")
  level = event_dict.pop("level", "?").upper()
  name = event_dict.pop("logger", "-")
  timestamp = event_dict.pop("timestamp", "-")
  event_dict.pop("correlation_id", None)
  extras = " ".join(
    f"{_sgr(f'{k}=', 'dim')}{_fmt_extra(v)}" for k, v in sorted(event_dict.items())
  )
  suffix = f" {extras}" if extras else ""
  level_code = _LEVEL_SGR.get(level, "dim")
  return (
    f"{_sgr(f'[{_PREFIX}]', 'cyan')}"
    f"{_sgr(level, level_code)}"
    f"{_sgr('@', 'dim')}{_sgr(name, 'blue')}"
    f"{_sgr(f'[{timestamp}]', 'dim')}{_sgr(':', 'dim')} "
    f"{_sgr(event, 'bold')}{suffix}"
  )


_FILE_FORMATTER = structlog.stdlib.ProcessorFormatter(
  foreign_pre_chain=_SHARED_PROCESSORS, processor=_render_file
)
_CONSOLE_FORMATTER = structlog.stdlib.ProcessorFormatter(
  foreign_pre_chain=_SHARED_PROCESSORS, processor=_render_console
)


def _render_diagnostic_json(
  logger: WrappedLogger, method: str, event_dict: EventDict
) -> str:
  """Render one bounded local core log record."""
  event = event_dict.get("event")
  code = (
    event if isinstance(event, str) and _EVENT_CODE.fullmatch(event) else "ei_event"
  )
  fields = {
    key: value
    for key, value in list(event_dict.items())[:_MAX_COLLECTION_ITEMS]
    if key
    not in {
      "event",
      "level",
      "logger",
      "timestamp",
      "run_id",
      "correlation_id",
      "exc_info",
      "exception",
      "stack",
      "stack_info",
      "_record",
      "_from_structlog",
    }
  }
  fields["logger"] = _safe_value(event_dict.get("logger", ""), key="logger")
  logger_name = str(event_dict.get("logger", "ei.main"))
  source_path = logger_name.removeprefix("ei.")
  if not source_path or source_path == "ei":
    source_path = "main"
  envelope = {
    "timestamp": event_dict.get("timestamp"),
    "level": "warn"
    if event_dict.get("level") == "warning"
    else event_dict.get("level"),
    "source": f"ei.{source_path}.py@aether-v0.2.1",
    "event_code": code,
    "message": code.replace("_", " "),
    "run_id": event_dict.get("run_id"),
    "correlation_id": event_dict.get("correlation_id"),
    "fields": fields,
  }
  return json.dumps(envelope, ensure_ascii=False, separators=(",", ":"))


def _level_value(level: str) -> int:
  return logging.getLevelName(level.upper())


def _most_verbose(*levels: str) -> str:
  return min(levels, key=_level_value)


class _RepeatState:
  """One bounded dedup-index entry."""

  def __init__(self, emitted_at: float, record: logging.LogRecord) -> None:
    self.emitted_at = emitted_at
    self.record = copy(record)
    self.suppressed = 0


class _RepeatSuppressor(logging.Filter):
  """Collapse identical repeated events in file sinks into counted lines.

  The full event payload participates in the key, excluding only volatile
  timestamp/correlation fields. Two model completions or primitive calls with the
  same event name but different tokens, arguments, or outcomes therefore stay
  visible. Truly identical polling chatter inside ``window_s`` is suppressed;
  the first occurrence after the window carries ``repeats=N``. The console
  sink is untouched.
  """

  def __init__(self, window_s: float, capacity: int = 4096) -> None:
    super().__init__()
    self._window_s = window_s
    self._capacity = capacity
    self._states: OrderedDict[tuple[str, str, str], _RepeatState] = OrderedDict()
    self._lock = threading.Lock()

  @staticmethod
  def _must_emit(record: logging.LogRecord) -> bool:
    msg = record.msg
    if not isinstance(msg, dict):
      return False
    event = str(msg.get("event", ""))
    status = str(msg.get("status", ""))
    return bool(
      msg.get("audit")
      or msg.get("terminal")
      or status
      in {
        "succeeded",
        "rejected",
        "failed",
        "cancelled",
        "timed_out",
        "outcome_unknown",
      }
      or event.endswith(
        ("_terminal", "_succeeded", "_rejected", "_failed", "_cancelled", "_timed_out")
      )
      or event in {"outcome_unknown", "command_terminal", "primitive_answered"}
    )

  def _event_signature(self, record: logging.LogRecord) -> str:
    """Stable signature for content, not merely the broad event label."""
    msg = record.msg
    if not isinstance(msg, dict):
      return str(msg)
    ignored = {"timestamp", "repeats", "dedup_summary"}
    if msg.get("event") in {"resource_read", "stream_frame"}:
      # Ingress payloads carry their own timestamp and stream sequence. Those
      # values make every poll unique even when nothing meaningful changed,
      # defeating rate limiting. Size and payload sequence still break the
      # suppression window when the resource changes.
      allowed = {
        "event",
        "backend",
        "uri",
        "kind",
        "bytes",
        "payload_sequence",
        "truncated",
        "run_id",
        "correlation_id",
        "mission_id",
        "step_id",
        "command_id",
        "execution_id",
      }
      content = {key: value for key, value in msg.items() if key in allowed}
    else:
      content = {key: value for key, value in msg.items() if key not in ignored}
    return json.dumps(content, sort_keys=True, ensure_ascii=False, default=str)

  def filter(self, record: logging.LogRecord) -> bool:
    if self._must_emit(record):
      return True
    key = (record.name, record.levelname, self._event_signature(record))
    now = time.monotonic()
    with self._lock:
      state = self._states.get(key)
      if state is not None and now - state.emitted_at < self._window_s:
        state.suppressed += 1
        self._states.move_to_end(key)
        return False
      suppressed = state.suppressed if state is not None else 0
      self._states[key] = _RepeatState(now, record)
      self._states.move_to_end(key)
      while len(self._states) > self._capacity:
        self._states.popitem(last=False)
    if suppressed and isinstance(record.msg, dict):
      record.msg["repeats"] = suppressed + 1
    return True

  def drain(self) -> list[logging.LogRecord]:
    """Return final counted records for repeats still suppressed at shutdown."""
    summaries: list[logging.LogRecord] = []
    with self._lock:
      for state in self._states.values():
        if not state.suppressed or not isinstance(state.record.msg, dict):
          continue
        record = copy(state.record)
        record.msg = dict(state.record.msg)
        record.msg["repeats"] = state.suppressed + 1
        record.msg["dedup_summary"] = True
        summaries.append(record)
      self._states.clear()
    return summaries


def _flush_dedup_summaries(handler: logging.Handler) -> None:
  """Write final repeat summaries without passing them through dedup again."""
  suppressors = [
    item for item in handler.filters if isinstance(item, _RepeatSuppressor)
  ]
  for suppressor in suppressors:
    handler.removeFilter(suppressor)
    for record in suppressor.drain():
      handler.emit(record)


class _CoreFileHandler(logging.FileHandler):
  """Append JSONL and preserve pending dedup summaries on close."""

  def close(self) -> None:
    _flush_dedup_summaries(self)
    super().close()


def configure_logging(
  log_dir: Path | str = "data/ei/log/local",
  config: LoggingConfig | None = None,
  *,
  replay_dir: Path | str,
  level: str | None = None,
  files_dir: str | None = None,
  run_id: str | None = None,
  managed: bool = False,
) -> None:
  """Configure structlog and stdlib logging once for the whole process.

  Idempotent: later calls are no-ops so library code can never clobber the
  launch-time configuration.

  Args:
    log_dir: Component diagnostic-log root; created on demand.
    config: Resolved ``[logging]`` section from ``etc/ei/ei.toml``.
    replay_dir: Deprecated caller argument; no replay file is created.
    level: Optional override for ``config.level`` (``--log-level``).
    files_dir: Optional subdirectory under ``log_dir`` for the human file
      sink. Spawned children pass their own name (e.g. ``"sim"``) so two
      processes sharing one component log root never write the same file.
  """
  global _configured, _console_handler, _console_config_level
  if _configured:
    return
  from ei.lib import shared_events

  selected_run = run_id or shared_events.select_run_id(False, None)
  if managed and run_id is None:
    raise ValueError("managed logging requires a validated run ID")

  cfg = config or LoggingConfig()
  if level is not None:
    cfg = cfg.model_copy(update={"level": level})  # type: ignore[arg-type]
  # Diagnostic files are the durable fallback for every console event and
  # error. Production config validation rejects disabling them; keep this
  # defensive promotion for programmatically constructed configs too.
  if not cfg.files.enabled:
    cfg = cfg.model_copy(
      update={"files": cfg.files.model_copy(update={"enabled": True})}
    )
  if cfg.console.enabled and not managed:
    file_level = _most_verbose(cfg.files.level, cfg.console.level)
    if file_level != cfg.files.level:
      cfg = cfg.model_copy(
        update={"files": cfg.files.model_copy(update={"level": file_level})}
      )

  # Remember the configured console level: it is the runtime target that
  # restore_console_level() returns to after launch/shutdown.
  _console_config_level = cfg.console.level

  # All EI diagnostics use one local JSONL file for IsOS to read.
  log_path = Path(log_dir)
  log_path.mkdir(parents=True, exist_ok=True)

  process_level = _most_verbose(
    cfg.level,
    *(sink.level for sink in (cfg.console, cfg.replay, cfg.files) if sink.enabled),
  )

  root = logging.getLogger()
  root.setLevel(process_level)

  # Third-party transports log their frames at DEBUG; the core file's DEBUG
  # level would otherwise fill ei.jsonl with transport chatter.
  # Raise just those loggers to INFO — the floor the frames are worth.
  for noisy in (
    "httpx",
    "httpcore",
    "websockets",
    "grpc",
    "asyncio",
    "httpx2",
    "httpcore2",
  ):
    logging.getLogger(noisy).setLevel(logging.INFO)

  def _dedup_filter() -> logging.Filter | None:
    # One fresh suppressor per sink: a shared instance would run once per
    # handler per record and double-count the suppressed repeats.
    return (
      _RepeatSuppressor(cfg.dedup_window_s, capacity=cfg.dedup_capacity)
      if cfg.dedup
      else None
    )

  if cfg.console.enabled and not managed:
    console_handler = logging.StreamHandler(sys.stderr)
    console_handler.setFormatter(_CONSOLE_FORMATTER)
    root.addHandler(console_handler)
    _console_handler = console_handler
    # Start at the launch/shutdown floor (at least INFO) so setup messages are
    # visible regardless of the configured console level; restore_console_level
    # drops to the configured level once the pipeline reports ready.
    show_lifecycle_logs()

  core_handler = _CoreFileHandler(
    log_path / "ei.jsonl",
    mode="a",
    encoding="utf-8",
  )
  core_handler.setLevel(cfg.files.level)
  core_handler.setFormatter(
    structlog.stdlib.ProcessorFormatter(
      foreign_pre_chain=_SHARED_PROCESSORS,
      processor=_render_diagnostic_json,
    )
  )
  dedup = _dedup_filter()
  if dedup is not None:
    core_handler.addFilter(dedup)
  root.addHandler(core_handler)

  structlog.configure(
    processors=[
      *_SHARED_PROCESSORS,
      structlog.stdlib.ProcessorFormatter.wrap_for_formatter,
    ],
    # The stdlib root and handlers own level filtering. Keeping the wrapper
    # unfiltered also lets temporary processor scopes (tests and diagnostics)
    # observe DEBUG events without inheriting an earlier configured floor.
    wrapper_class=structlog.stdlib.BoundLogger,
    logger_factory=structlog.stdlib.LoggerFactory(),
    # Tests and extension processes may replace processors while preserving
    # module-level logger proxies. Keeping those proxies dynamic prevents a
    # previous sink configuration from bypassing capture/redaction processors.
    cache_logger_on_first_use=False,
  )
  _log_context.set({"run_id": selected_run})
  # Closed lifecycle events share the local file. Managed stderr is discarded
  # because native writes, warnings, and tracebacks may contain secrets.
  event_stream = (log_path / "ei.jsonl").open("a", encoding="utf-8", buffering=1)
  shared_events.configure(selected_run, stream=event_stream)
  if managed:
    null_fd = os.open(os.devnull, os.O_WRONLY)
    try:
      os.dup2(null_fd, 2)
    finally:
      os.close(null_fd)
  _configured = True


def get_logger(name: str) -> structlog.stdlib.BoundLogger:
  """Return a bound logger for ``name``; call after ``configure_logging``."""
  return structlog.get_logger(name)


def log_once(
  logger: structlog.stdlib.BoundLogger,
  level: str,
  event: str,
  *,
  key: str | None = None,
  **kwargs: object,
) -> None:
  """Emit ``event`` at most once per process for a dedupe key.

  The key defaults to ``event`` scoped to the logger name. Pass ``key`` to
  dedupe on a finer grain (e.g. once per HTTP path).
  """
  name = (
    logger._logger_factory_args[0]  # type: ignore[attr-defined]
    if getattr(logger, "_logger_factory_args", None)
    else "?"
  )
  token = f"{name}:{key or event}"
  if token in _logged_once:
    return
  _logged_once.add(token)
  getattr(logger, level)(event, **kwargs)


def _lifecycle_floor_level() -> str:
  """Console level for launch/shutdown: at least INFO, never less verbose.

  A ``[logging.console] level`` of WARNING (the default) would hide setup and
  teardown messages; this floors the console to INFO in that case. A DEBUG
  config is left untouched so nothing is raised above what was configured.
  """
  floor = min(_level_value(_console_config_level), _level_value("INFO"))
  return logging.getLevelName(floor)


def restore_console_level() -> None:
  """Drop the stderr threshold back to the configured ``[logging.console] level``.

  Called once the pipeline reports ready, so only the configured verbosity
  reaches the terminal during runtime. Replay and file sinks are unaffected —
  they keep every event at their own configured levels.
  """
  if _console_handler is not None:
    _console_handler.setLevel(_console_config_level.upper())


def show_lifecycle_logs() -> None:
  """Lower stderr to the launch/shutdown floor (at least INFO).

  Ensures setup and teardown messages reach the terminal even when the
  configured ``[logging.console] level`` would hide them; never raises the
  threshold (a DEBUG config stays DEBUG).
  """
  if _console_handler is not None:
    _console_handler.setLevel(_lifecycle_floor_level())


@contextmanager
def bind_correlation_id(correlation_id: str | None = None) -> Iterator[str]:
  """Bind a correlation ID to every event emitted inside the context.

  Args:
    correlation_id: Existing ID to propagate, or ``None`` to mint a new UUID4.

  Yields:
    The effective correlation ID.
  """
  cid = correlation_id or uuid.uuid4().hex
  token = _correlation_id.set(cid)
  try:
    yield cid
  finally:
    _correlation_id.reset(token)


@contextmanager
def bind_log_context(**fields: str | None) -> Iterator[dict[str, str]]:
  """Bind supported lifecycle identities to every event in the current context.

  Supported identities are ``run_id``, ``correlation_id``, ``mission_id``,
  ``step_id``, ``backend``, ``command_id``, and ``execution_id``. ``None``
  removes a field for the nested context; unknown names fail closed.
  """
  unknown = set(fields) - _CONTEXT_FIELDS
  if unknown:
    raise ValueError(f"unknown logging context fields: {sorted(unknown)}")
  context = dict(_log_context.get() or {})
  for key, value in fields.items():
    if value is None:
      context.pop(key, None)
    else:
      context[key] = str(value)
  token = _log_context.set(context)
  try:
    yield dict(context)
  finally:
    _log_context.reset(token)
