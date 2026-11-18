"""Run-owned EI native node lifecycle for the Linux v1 ABI."""

from __future__ import annotations

import asyncio
import ctypes
import hashlib
import json
import threading
import time
from collections.abc import Callable
from datetime import UTC, datetime, timedelta
from pathlib import Path
from typing import Any

from ei.core.cognition.context.builder import ContextBuilder
from ei.core.evidence.perception.admission import Admission
from ei.core.system.node_registry import _NodeApi, _safe_file, close_library
from ei.lib.config import load_prompt_config

_Emit = ctypes.CFUNCTYPE(
  ctypes.c_int32, ctypes.c_void_p, ctypes.POINTER(ctypes.c_uint8), ctypes.c_size_t
)
_Log = ctypes.CFUNCTYPE(
  None,
  ctypes.c_void_p,
  ctypes.c_uint32,
  ctypes.POINTER(ctypes.c_uint8),
  ctypes.c_size_t,
)


class _Host(ctypes.Structure):
  _fields_ = [
    ("abi_version", ctypes.c_uint32),
    ("struct_size", ctypes.c_uint32),
    ("context", ctypes.c_void_p),
    ("emit", _Emit),
    ("log", _Log),
  ]


_Create = ctypes.CFUNCTYPE(
  ctypes.c_int32,
  ctypes.POINTER(ctypes.c_uint8),
  ctypes.c_size_t,
  ctypes.POINTER(_Host),
  ctypes.POINTER(ctypes.c_void_p),
)
_One = ctypes.CFUNCTYPE(ctypes.c_int32, ctypes.c_void_p)
_Input = ctypes.CFUNCTYPE(
  ctypes.c_int32, ctypes.c_void_p, ctypes.POINTER(ctypes.c_uint8), ctypes.c_size_t
)
_Destroy = ctypes.CFUNCTYPE(None, ctypes.c_void_p)


class NativeNode:
  """Keep the library, host callbacks and opaque handle alive for one run."""

  def __init__(
    self,
    root: str,
    implementation: dict[str, Any],
    config: dict[str, Any],
    emit: Callable[[dict[str, Any]], None],
  ) -> None:
    self.library = ctypes.CDLL(
      str(_safe_file(__import__("pathlib").Path(root), implementation["library"])),
      mode=ctypes.RTLD_LOCAL,
    )
    entry = self.library.isos_node_v1
    entry.argtypes = []
    entry.restype = ctypes.POINTER(_NodeApi)
    api = entry().contents
    if api.abi_version != 1 or api.struct_size < ctypes.sizeof(_NodeApi):
      raise ValueError("unsupported EI native node ABI")
    if any(
      not getattr(api, name)
      for name in ("create", "start", "input", "health", "stop", "destroy")
    ):
      raise ValueError("incomplete EI native node ABI")
    self._create = _Create(api.create)
    self._start = _One(api.start)
    self._input = _Input(api.input)
    self._health = _One(api.health)
    self._stop = _One(api.stop)
    self._destroy = _Destroy(api.destroy)
    self._handle = ctypes.c_void_p()

    def _emit(_context: int, data: Any, length: int) -> int:
      if length > 1_048_576:
        return -1
      try:
        value = json.loads(ctypes.string_at(data, length))
        if not isinstance(value, dict):
          return -1
        emit(value)
        return 0
      except (ValueError, TypeError, OSError):
        return -1

    def _log(_context: int, _level: int, _message: Any, _length: int) -> None:
      return

    self._emit_callback = _Emit(_emit)
    self._log_callback = _Log(_log)
    self._host = _Host(
      1, ctypes.sizeof(_Host), None, self._emit_callback, self._log_callback
    )
    payload = json.dumps(config, separators=(",", ":")).encode()
    if len(payload) > 1_048_576:
      raise ValueError("EI node configuration too large")
    buffer = (ctypes.c_uint8 * len(payload)).from_buffer_copy(payload)
    if (
      self._create(
        buffer, len(payload), ctypes.byref(self._host), ctypes.byref(self._handle)
      )
      != 0
      or not self._handle
    ):
      raise ValueError("EI node create failed")
    if self._start(self._handle) != 0:
      self._destroy(self._handle)
      self._handle = ctypes.c_void_p()
      close_library(self.library)
      raise ValueError("EI node start failed")

  def input(self, value: dict[str, Any]) -> None:
    payload = json.dumps(value, separators=(",", ":")).encode()
    if len(payload) > 1_048_576:
      raise ValueError("EI node input too large")
    buffer = (ctypes.c_uint8 * len(payload)).from_buffer_copy(payload)
    if self._input(self._handle, buffer, len(payload)) != 0:
      raise ValueError("EI node input failed")

  def ready(self) -> bool:
    return self._health(self._handle) == 1

  def stop(self) -> None:
    if self._handle:
      if self._stop(self._handle) != 0:
        raise ValueError("EI node stop did not prove quiescence")
      self._destroy(self._handle)
      self._handle = ctypes.c_void_p()
      close_library(self.library)


class ExternalNode:
  """Run one package process over the owner-private external-node v1 socket."""

  def __init__(
    self,
    root: str,
    entrypoint: str,
    config: dict[str, Any],
    emit: Callable[[dict[str, Any]], None],
  ) -> None:
    import os
    import subprocess
    import tempfile
    import time
    from pathlib import Path

    self._directory = tempfile.TemporaryDirectory(prefix="ei-node-")
    self._path = str(Path(self._directory.name) / "node.sock")
    self._emit = emit
    self._process = subprocess.Popen(
      [str(_safe_file(Path(root), entrypoint))],
      env={
        **os.environ,
        "ISOS_NODE_SOCKET": self._path,
        "ISOS_RUN_ID": config["run_id"],
        "ISOS_REVISION": config["revision"],
      },
      stdin=subprocess.DEVNULL,
      stdout=subprocess.DEVNULL,
      stderr=subprocess.DEVNULL,
      start_new_session=True,
    )
    deadline = time.monotonic() + 5
    while not Path(self._path).is_socket():
      if self._process.poll() is not None or time.monotonic() >= deadline:
        self._process.kill()
        self._process.wait()
        self._directory.cleanup()
        raise ValueError("EI external node failed to open socket")
      time.sleep(0.01)
    try:
      reply = self._exchange({"version": 1, "op": "start", "config": config})
      if reply.get("ok") is not True:
        raise ValueError("EI external node start failed")
    except (OSError, TimeoutError, ValueError):
      self._process.kill()
      self._process.wait()
      self._directory.cleanup()
      raise

  def _exchange(self, request: dict[str, Any]) -> dict[str, Any]:
    import socket

    body = json.dumps(request, separators=(",", ":")).encode() + b"\n"
    if len(body) > 1_048_576:
      raise ValueError("EI external request too large")
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as connection:
      connection.settimeout(5)
      connection.connect(self._path)
      connection.sendall(body)
      response = bytearray()
      while not response.endswith(b"\n"):
        chunk = connection.recv(4096)
        if not chunk or len(response) + len(chunk) > 1_048_576:
          raise ValueError("EI external response is invalid")
        response.extend(chunk)
    value = json.loads(response)
    if not isinstance(value, dict) or value.get("version") != 1:
      raise ValueError("EI external response version is invalid")
    return value

  def input(self, value: dict[str, Any]) -> None:
    reply = self._exchange({"version": 1, "op": "input", "data": value})
    if reply.get("ok") is not True or not isinstance(reply.get("outputs"), list):
      raise ValueError("EI external node input failed")
    for output in reply["outputs"]:
      if not isinstance(output, dict):
        raise TypeError("EI external node output is invalid")
      self._emit(output)

  def ready(self) -> bool:
    return (
      self._process.poll() is None
      and self._exchange({"version": 1, "op": "health"}).get("ready") is True
    )

  def stop(self) -> None:
    if self._process.poll() is not None:
      raise ValueError("EI external node exited without stop proof")
    reply = self._exchange({"version": 1, "op": "stop"})
    if reply.get("ok") is not True or reply.get("quiesced") is not True:
      raise ValueError("EI external node stop did not prove quiescence")
    try:
      self._process.wait(timeout=2)
    except __import__("subprocess").TimeoutExpired:
      self._process.terminate()
      self._process.wait(timeout=2)
    self._directory.cleanup()


class OwnerStage:
  """Run-owned external owner interface for a built-in boundary stage."""

  def __init__(self, emit: Callable[[dict[str, Any]], None]) -> None:
    self._emit = emit
    self._ready = True

  def input(self, value: dict[str, Any]) -> None:
    if not self._ready:
      raise ValueError("EI owner stage is stopped")
    self._emit(value)

  def ready(self) -> bool:
    return self._ready

  def stop(self) -> None:
    self._ready = False


class GatewayStage:
  """Admit attributed, bounded EOS records without opening a graph link."""

  def __init__(self, emit, run_id: str | None = None):
    self._emit = emit
    self._ready = True
    self._run_id = run_id
    self._latest_health = None

  def input(self, value):
    if not self._ready:
      raise ValueError("Gateway stage is stopped")
    port = value.get("port")
    data = value.get("payload", value.get("data"))
    if port in {"resource", "health", "result"}:
      if not isinstance(data, dict):
        raise ValueError("Gateway record must be an object")
      if data.get("owner") != "eos" or (
        self._run_id is not None and data.get("run_id") != self._run_id
      ):
        raise ValueError("Gateway record owner or run mismatch")
      source_time = data.get("source_time")
      if not isinstance(source_time, str):
        raise ValueError("Gateway record requires source time")
      try:
        observed = datetime.fromisoformat(source_time)
      except ValueError as exc:
        raise ValueError("Gateway source time is invalid") from exc
      age = datetime.now(UTC) - observed if observed.tzinfo else None
      if age is None or not timedelta(seconds=-1) <= age <= timedelta(seconds=300):
        raise ValueError("Gateway source time is stale or in the future")
      if len(json.dumps(data, allow_nan=False).encode()) > 65_536:
        raise ValueError("Gateway record exceeds byte bound")
    if port == "resource":
      self._emit({"port": "resource", "payload": data})
    elif port == "health":
      self._latest_health = data
      self._emit({"port": "health", "payload": data})
    elif port in ("gateway", "admitted", "emergency_stop"):
      pass  # fixed inputs, owner-internal
    elif port == "result":
      self._emit({"port": "result", "payload": data})
    else:
      raise ValueError(f"Gateway stage received unsupported port: {port!r}")

  def ready(self):
    return self._ready

  def stop(self):
    self._ready = False


class AdmissionStage:
  """Accept resource/hearing inputs and emit observations."""

  def __init__(self, emit):
    self._emit = emit
    self._ready = True

  def input(self, value):
    if not self._ready:
      raise ValueError("Admission stage is stopped")
    port = value.get("port")
    data = value.get("payload", value.get("data"))
    if port in ("resource", "hearing", "result"):
      if not isinstance(data, dict):
        raise ValueError("Admission requires attributed evidence")
      kind = data.get("kind")
      source = data.get("source")
      text = data.get("text")
      if (
        not isinstance(kind, str)
        or not kind
        or not isinstance(source, str)
        or not source
        or not isinstance(text, str)
        or not text
      ):
        raise ValueError("Admission requires kind, source and text")
      if len(text.encode()) > 4096:
        raise ValueError("Admission evidence is too large")
      observation = {"kind": kind, "source": source, "text": text, "raw": data}
      self._emit({"port": "observations", "payload": observation})
    else:
      raise ValueError(f"Admission stage received unsupported port: {port!r}")

  def ready(self):
    return self._ready

  def stop(self):
    self._ready = False


class BuilderStage:
  """Accept observations and emit a context snapshot."""

  def __init__(self, emit):
    self._emit = emit
    self._ready = True
    self._loop = asyncio.new_event_loop()
    try:
      from ei.core.evidence.perception.admission import Admission

      self._queue = Admission(capacity=64, history_per_kind=8)
      from ei.core.cognition.context.builder import ContextBuilder
      from ei.lib.config import load_prompt_config

      self._builder = ContextBuilder(self._queue, load_prompt_config())
      self._has_queue = True
    except (ImportError, AttributeError, TypeError):
      self._has_queue = False

  def input(self, value):
    if not self._ready:
      raise ValueError("Builder stage is stopped")
    port = value.get("port")
    data = value.get("payload", value.get("data"))
    if port == "observations":
      if self._has_queue:
        try:
          if isinstance(data, dict):
            self._loop.run_until_complete(
              self._queue.enqueue_latest(
                kind=data.get("kind", "hardware"),
                source=data.get("source", "admission"),
                text=data.get("text", str(data)),
              )
            )
          snapshot = self._builder.build().model_dump(mode="json")
          self._emit({"port": "snapshot", "payload": snapshot})
          return
        except (
          TimeoutError,
          ImportError,
          AttributeError,
          TypeError,
          ValueError,
          OSError,
        ):
          pass
      raise ValueError("Context builder could not render admitted evidence")
    elif port in ("body", "memory"):
      pass  # accepted but not used in stub
    else:
      raise ValueError(f"Builder stage received unsupported port: {port!r}")

  def ready(self):
    return self._ready

  def stop(self):
    self._ready = False
    if not self._loop.is_closed():
      self._loop.close()


class QueueStage:
  """Select unique missions from a bounded local priority queue."""

  def __init__(self, emit, capacity: int = 8):
    if type(capacity) is not int or not 1 <= capacity <= 256:
      raise ValueError("mission capacity must be 1–256")
    self._emit = emit
    self._ready = True
    self._latest_selected = None
    self._capacity = capacity
    self._missions: dict[str, dict[str, Any]] = {}

  def input(self, value):
    if not self._ready:
      raise ValueError("Queue stage is stopped")
    port = value.get("port")
    data = value.get("payload", value.get("data"))
    if port == "intent":
      if not isinstance(data, dict):
        raise ValueError("Mission intent must be an object")
      mission_id = data.get("mission_id")
      mission_text = data.get("text")
      priority = data.get("priority", 0)
      if (
        not isinstance(mission_id, str)
        or not 1 <= len(mission_id) <= 128
        or not isinstance(mission_text, str)
        or not 1 <= len(mission_text) <= 4096
        or type(priority) is not int
        or not -100 <= priority <= 100
      ):
        raise ValueError("Mission intent ID, text or priority is invalid")
      if mission_id in self._missions or len(self._missions) >= self._capacity:
        raise ValueError("Duplicate mission ID or queue capacity exhausted")
      self._missions[mission_id] = {
        "mission_id": mission_id,
        "text": mission_text,
        "priority": priority,
      }
      selected = max(self._missions.values(), key=lambda item: item["priority"])
      if selected != self._latest_selected:
        self._latest_selected = selected.copy()
        self._emit({"port": "selected", "payload": selected.copy()})
    elif port == "result":
      if (
        not isinstance(data, dict)
        or data.get("mission_id") not in self._missions
        or data.get("state") not in {"completed", "failed", "cancelled"}
      ):
        raise ValueError("Mission result does not match a queued mission")
      del self._missions[data["mission_id"]]
      self._latest_selected = None
      self._emit({"port": "lifecycle", "payload": data})
      if self._missions:
        selected = max(self._missions.values(), key=lambda item: item["priority"])
        self._latest_selected = selected.copy()
        self._emit({"port": "selected", "payload": selected.copy()})
    else:
      raise ValueError(f"Queue stage received unsupported port: {port!r}")

  def ready(self):
    return self._ready

  def stop(self):
    self._ready = False
    self._missions.clear()


class PlannerStage:
  """Produce a bounded plan from a provider or explicitly abstain."""

  def __init__(
    self,
    emit,
    provider: Callable[[dict[str, Any], dict[str, Any]], dict[str, Any]] | None = None,
    verified_model: tuple[str, str] | None = None,
    max_steps: int = 8,
    deadline_ms: int = 1000,
  ):
    if (
      type(max_steps) is not int
      or not 1 <= max_steps <= 64
      or type(deadline_ms) is not int
      or not 1 <= deadline_ms <= 30_000
    ):
      raise ValueError("Planner max steps or deadline is invalid")
    if provider is not None:
      if verified_model is None or len(verified_model) != 2:
        raise ValueError("Planner provider requires exact verified model")
      path, digest = verified_model
      if (
        not Path(path).is_file()
        or hashlib.sha256(Path(path).read_bytes()).hexdigest() != digest
      ):
        raise ValueError("Planner verified model bytes changed or missing")
    self._emit = emit
    self._ready = True
    self._provider = provider
    self._verified_model = verified_model
    self._max_steps = max_steps
    self._deadline_ms = deadline_ms
    self._latest_selected = None
    self._latest_snapshot = None

  def input(self, value):
    if not self._ready:
      raise ValueError("Planner stage is stopped")
    port = value.get("port")
    data = value.get("payload", value.get("data"))
    if port == "selected":
      if (
        not isinstance(data, dict)
        or not isinstance(data.get("mission_id"), str)
        or not data["mission_id"]
      ):
        raise ValueError("Planner requires selected mission identity")
      self._latest_selected = data
    elif port == "snapshot":
      if not isinstance(data, dict) or not isinstance(data.get("text"), str):
        raise ValueError("Planner requires context text")
      if len(data["text"].encode()) > 65_536:
        raise ValueError("Planner context exceeds byte bound")
      self._latest_snapshot = data
    else:
      raise ValueError(f"Planner stage received unsupported port: {port!r}")

    if self._latest_selected is not None and self._latest_snapshot is not None:
      if self._provider is None:
        self._emit(
          {
            "port": "plan_state",
            "payload": {
              "state": "abstained",
              "reason": "missing_model",
            },
          }
        )
        return
      if self._verified_model is not None:
        path, digest = self._verified_model
        if (
          not Path(path).is_file()
          or hashlib.sha256(Path(path).read_bytes()).hexdigest() != digest
        ):
          raise ValueError("Planner verified model bytes changed or missing")
      started = time.monotonic()
      result = self._provider(self._latest_selected, self._latest_snapshot)
      if (time.monotonic() - started) * 1000 > self._deadline_ms:
        raise ValueError("Planner deadline exceeded")
      if (
        not isinstance(result, dict)
        or set(result) != {"plan_id", "steps", "proposal"}
        or not isinstance(result["plan_id"], str)
        or not result["plan_id"]
        or not isinstance(result["steps"], list)
        or not 1 <= len(result["steps"]) <= self._max_steps
        or not all(
          isinstance(step, str) and 1 <= len(step) <= 256 for step in result["steps"]
        )
      ):
        raise ValueError("Planner result is invalid or exceeds step bound")
      proposal = result["proposal"]
      if (
        not isinstance(proposal, dict)
        or not all(
          isinstance(proposal.get(key), str) and proposal[key]
          for key in ("backend", "device", "primitive", "version", "target_id")
        )
        or not isinstance(proposal.get("arguments"), dict)
        or not isinstance(proposal.get("expires_at"), str)
      ):
        raise ValueError("Planner proposal lacks target or expiry")
      try:
        expiry = datetime.fromisoformat(proposal["expires_at"])
      except ValueError as exc:
        raise ValueError("Planner proposal expiry is invalid") from exc
      if expiry.tzinfo is None or expiry <= datetime.now(UTC):
        raise ValueError("Planner proposal has expired")
      if len(json.dumps(result, allow_nan=False).encode()) > 65_536:
        raise ValueError("Planner output exceeds byte bound")
      self._emit({"port": "proposal", "payload": proposal})
      self._emit(
        {
          "port": "plan_state",
          "payload": {
            "state": "planned",
            "plan_id": result["plan_id"],
            "steps": len(result["steps"]),
          },
        }
      )

  def ready(self):
    return self._ready

  def stop(self):
    self._ready = False


class SafetyGateStage:
  """Forward activation unchanged and observe planner proposals without emitting."""

  def __init__(self, emit):
    self._emit = emit
    self._ready = True
    self._latest_proposal = None

  def input(self, value):
    if not self._ready:
      raise ValueError("Safety gate stage is stopped")
    port = value.get("port")
    data = value.get("payload", value.get("data"))
    if port == "activation":
      self._emit({"port": "activation", "payload": data})
    elif port == "proposal":
      self._latest_proposal = data
    else:
      raise ValueError(f"Safety gate stage received unsupported port: {port!r}")

  def ready(self):
    return self._ready

  def stop(self):
    self._ready = False


class HealthMonitorStage:
  """Accept health input and emit periodic status."""

  def __init__(self, emit):
    self._emit = emit
    self._ready = True
    self._latest_health = None

  def input(self, value):
    if not self._ready:
      raise ValueError("Health monitor stage is stopped")
    port = value.get("port")
    data = value.get("payload", value.get("data"))
    if port == "health":
      self._latest_health = data
      self._emit({"port": "status", "payload": {"health": "ok", "detail": data}})
    elif port in ("progress", "bounds"):
      pass  # accepted, not emitted in stub
    else:
      raise ValueError(f"Health monitor stage received unsupported port: {port!r}")

  def ready(self):
    return self._ready

  def stop(self):
    self._ready = False


class ForwarderStage:
  """Cognition intent forwarder that consumes the context snapshot.

  Caches the latest context snapshot while forwarding the activation event
  unchanged so the safety-gate and EOS contracts remain intact.  The
  snapshot is held for downstream consumers (planner, logging, or other
  stages) without altering the activation payload.
  """

  def __init__(self, emit: Callable[[dict[str, Any]], None]) -> None:
    self._emit = emit
    self._ready = True
    self._lock = threading.Lock()
    self._latest_snapshot: dict[str, Any] | None = None

  def input(self, value: dict[str, Any]) -> None:
    if not self._ready:
      raise ValueError("EI forwarder stage is stopped")
    port = value.get("port")
    if port == "snapshot":
      with self._lock:
        self._latest_snapshot = value.get("payload", value.get("data"))
      return
    if port == "activation":
      self._emit(value)
      return
    raise ValueError(f"EI forwarder stage received unsupported port: {port!r}")

  def ready(self) -> bool:
    return self._ready

  def stop(self) -> None:
    self._ready = False


class ContextStage:
  """Organize a bounded model context snapshot from admitted observations.

  Observations are admitted into the canonical EI perception queue and the
  snapshot is rendered by the shared ``ContextBuilder``, so provenance,
  section ordering and token budgeting match the product context pipeline.
  The activation contract stays unchanged: activations are emitted verbatim
  on the ``activation`` port.
  """

  def __init__(self, emit: Callable[[dict[str, Any]], None]) -> None:
    self._emit = emit
    self._ready = True
    self._lock = threading.Lock()
    self._loop = asyncio.new_event_loop()
    self._queue = Admission(capacity=64, history_per_kind=8)
    self._builder = ContextBuilder(self._queue, load_prompt_config())

  def input(self, value: dict[str, Any]) -> None:
    if not self._ready:
      raise ValueError("EI context stage is stopped")
    port = value.get("port")
    data = value.get("payload", value.get("data"))
    if port == "observation":
      if not isinstance(data, dict):
        raise ValueError("EI context stage requires an observation object")
      source = str(data.get("source_node") or "arm_observer")
      rendered = _render_arm_text(data)
      with self._lock:
        self._loop.run_until_complete(
          self._queue.enqueue_latest(
            kind="hardware",
            source=source,
            text=rendered,
          )
        )
      return
    if port != "activation":
      raise ValueError("EI context stage received an unsupported input port")
    with self._lock:
      snapshot = self._builder.build().model_dump(mode="json")
    # Emit the activation first: it is the path safety and EOS rely on.
    self._emit({"port": "activation", "payload": data})
    self._emit({"port": "snapshot", "payload": snapshot})

  def ready(self) -> bool:
    return self._ready

  def stop(self) -> None:
    self._ready = False
    if not self._loop.is_closed():
      self._loop.close()


def _render_arm_text(observation: dict[str, Any]) -> str:
  """Render one arm observation as the hardware prompt data line."""
  names = observation.get("joint_names")
  positions = observation.get("joint_positions_rad")
  pairs: list[str] = []
  if isinstance(names, list) and isinstance(positions, list):
    for name, position in zip(names, positions):
      if (
        isinstance(name, str)
        and isinstance(position, (int, float))
        and not isinstance(position, bool)
      ):
        pairs.append(f"{name}={float(position):.3f}")
  if pairs:
    return ", ".join(pairs)
  state = observation.get("state")
  return (
    f"joint positions unavailable (state={state})"
    if isinstance(state, str)
    else "joint positions unavailable"
  )


def builtin_library() -> tuple[str, dict[str, str]]:
  """Resolve the packaged ABI library, compiling only in a source checkout."""
  import subprocess
  from pathlib import Path

  package_root = Path(__file__).parents[2]
  packaged = package_root / "native" / "libei_demo_stage.so"
  if packaged.is_file():
    return str(packaged.parent), {
      "kind": "native_plugin",
      "abi": "isos_node_v1",
      "library": packaged.name,
    }
  repository = package_root.parent
  source = repository / "ei/native/demo_stage.c"
  header = repository / "contract/node/isos_node_v1.h"
  if not source.is_file() or not header.is_file():
    raise ValueError("EI demo native library is not installed")
  cache = repository / "data/cache/tools/ei-native"
  cache.mkdir(parents=True, exist_ok=True)
  library = cache / "libei_demo_stage.so"
  if not library.is_file() or library.stat().st_mtime < max(
    source.stat().st_mtime, header.stat().st_mtime
  ):
    subprocess.run(
      [
        "cc",
        "-std=c11",
        "-O2",
        "-fPIC",
        "-shared",
        "-I",
        str(header.parent),
        str(source),
        "-o",
        str(library),
      ],
      check=True,
      capture_output=True,
      timeout=30,
    )
  return str(cache), {
    "kind": "native_plugin",
    "abi": "isos_node_v1",
    "library": library.name,
  }
