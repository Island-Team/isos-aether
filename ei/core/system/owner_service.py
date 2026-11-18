"""Persistent EI owner control endpoint over a private Unix socket."""

from __future__ import annotations

import hashlib
import json
import math
import os
import re
import signal
import socket
import socketserver
import subprocess
import threading
import time
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

from pydantic import ValidationError

from ei.core.system.node_registry import BUILTIN_PLACEMENTS, load_registry
from ei.core.system.node_runtime import (
  AdmissionStage,
  BuilderStage,
  ContextStage,
  ExternalNode,
  ForwarderStage,
  GatewayStage,
  HealthMonitorStage,
  NativeNode,
  OwnerStage,
  PlannerStage,
  QueueStage,
  SafetyGateStage,
  builtin_library,
)
from ei.core.system.setup import (
  MAX_SETUP_INPUT_BYTES,
  SETUP_SCHEMA_VERSION,
  SetupDiagnostic,
  _setup_reply,
  _validate_setup_request,
)
from ei.lib.layout import Layout

MAX_HEADER_BYTES = 8192
MAX_REPLY_BYTES = 4 * 1024 * 1024
MAX_LOG_PAGE_BYTES = 64 * 1024


def _core_log(level: str, message: str, **fields: object) -> None:
  """Append owner diagnostics locally without carrying logs in owner replies."""
  try:
    _write_core_log(level, message, **fields)
  except OSError:
    # A failed diagnostic write must not turn a completed owner action into
    # an ambiguous transport error.
    pass


def _write_core_log(level: str, message: str, **fields: object) -> None:
  directory = Path(
    os.environ.get("EI_LOG_DIR", Layout.resolve().component_log("ei") / "local")
  )
  directory.mkdir(parents=True, exist_ok=True)
  record = {
    "timestamp": datetime.now(UTC)
    .isoformat(timespec="milliseconds")
    .replace("+00:00", "Z"),
    "level": level,
    "source": "ei.owner_service.py@aether-v0.2.3",
    "message": message,
    **fields,
  }
  data = (json.dumps(record, ensure_ascii=False, separators=(",", ":")) + "\n").encode()
  fd = os.open(directory / "ei.jsonl", os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
  try:
    os.write(fd, data)
  finally:
    os.close(fd)
  safe_message = message.replace("\r", "\\n").replace("\n", "\\n")
  readable = (
    f'IsOS-v0.2.3@{record["source"]}: '
    f'[{level.upper()}][{record["timestamp"]}]'
    f'{safe_message}\n'
  ).encode()
  fd = os.open(directory / "ei.log", os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
  try:
    os.write(fd, readable)
  finally:
    os.close(fd)


def _read_core_logs(after: int) -> dict[str, Any]:
  directory = Path(
    os.environ.get("EI_LOG_DIR", Layout.resolve().component_log("ei") / "local")
  )
  path = directory / "ei.jsonl"
  if not path.is_file():
    return {"schema_version": 1, "core": "ei", "next_cursor": 0, "entries": []}
  with path.open("rb") as stream:
    length = stream.seek(0, os.SEEK_END)
    start = after if after <= length else 0
    stream.seek(start)
    page = stream.read(MAX_LOG_PAGE_BYTES)
  complete = page.rfind(b"\n") + 1
  entries: list[dict[str, Any]] = []
  for line in page[:complete].splitlines():
    try:
      record = json.loads(line)
    except (UnicodeError, json.JSONDecodeError):
      continue
    if not isinstance(record, dict):
      continue
    level = record.get("level")
    message = record.get("message")
    if level in {"warning", "warn", "error", "critical"} or (
      level == "info"
      and isinstance(message, str)
      and message.startswith(("EI owner service", "EI health worker"))
    ):
      entries.append(record)
  return {
    "schema_version": 1,
    "core": "ei",
    "next_cursor": start + complete,
    "entries": entries,
  }


def _signal_group(pid: int, signum: int) -> None:
  try:
    os.killpg(pid, signum)
  except ProcessLookupError:
    pass


class _OwnerServer(socketserver.UnixStreamServer):
  allow_reuse_address = False

  def __init__(self, socket_path: str, handler: type[_Handler]) -> None:
    super().__init__(socket_path, handler)
    self.worker: subprocess.Popen[bytes] | None = None
    self.run_id: str | None = None
    self.revision: str | None = None
    self.terminal: dict[str, Any] | None = None
    self.previous_runs: set[tuple[str, str]] = set()
    self.eos_owner_path: str | None = None
    self.activation_graph: dict[str, Any] | None = None
    self.monitor_latest: dict[str, Any] | None = None
    self.monitor_polled_at: float = 0.0
    self.monitor_joint_names: list[str] | None = None
    self.monitor_forward_interval: float = 0.05
    self.monitor_lock = threading.Lock()
    self.monitor_stop = threading.Event()
    self.monitor_thread: threading.Thread | None = None
    self.monitor_error: str | None = None
    self.custom_nodes: dict[
      str,
      NativeNode
      | ExternalNode
      | OwnerStage
      | ContextStage
      | ForwarderStage
      | GatewayStage
      | AdmissionStage
      | BuilderStage
      | QueueStage
      | PlannerStage
      | HealthMonitorStage
      | SafetyGateStage,
    ] = {}
    self.node_outputs: dict[str, list[dict[str, Any]]] = {}
    self.node_edges: dict[str, list[tuple[str, str, str]]] = {}
    self.cross_inputs: dict[tuple[str, str], tuple[str, str]] = {}
    self.cross_outputs: dict[str, list[tuple[str, str, str]]] = {}
    self.node_depth = threading.local()

  def emit_node(self, path: str, value: dict[str, Any]) -> None:
    depth = getattr(self.node_depth, "value", 0)
    if depth >= 64:
      raise ValueError("EI node route depth exceeded")
    self.node_outputs[path].append(value)
    output = value.get("port")
    data = value.get("payload", value.get("data"))
    if not isinstance(output, str) or not isinstance(data, dict):
      return
    self.node_depth.value = depth + 1
    try:
      for target, input_name, output_name in self.node_edges.get(path, []):
        if output == output_name:
          self.custom_nodes[target].input({"port": input_name, "payload": data})
      for target, input_name, output_name in self.cross_outputs.get(path, []):
        if output != output_name:
          continue
        if self.eos_owner_path is None or self.run_id is None or self.revision is None:
          raise ValueError("EOS owner route is unavailable")
        status, reply = _post_owner(
          self.eos_owner_path,
          "/v1/input",
          {
            "schema_version": 1,
            "core": "eos",
            "op": "input",
            "run_id": self.run_id,
            "revision": self.revision,
            "node": target,
            "port": input_name,
            "source_node": path,
            "source_port": output,
            "payload": data,
          },
        )
        if (
          status != 200
          or reply.get("ok") is not True
          or reply.get("run_id") != self.run_id
          or reply.get("revision") != self.revision
        ):
          raise ValueError("EOS owner rejected node route")
    finally:
      self.node_depth.value = depth

  def start_nodes(
    self,
    graph: dict[str, Any],
    packages: list[dict[str, Any]],
    run_id: str,
    revision: str,
    cross_edges: list[dict[str, Any]] | None = None,
  ) -> None:
    registry = load_registry(packages)
    for layer_id, layer in graph["cores"]["ei"]["layers"].items():
      for group_id, group in layer["groups"].items():
        for node_id, instance in group["nodes"].items():
          spec = registry[instance["node_type"]]
          built_in = instance["node_type"] in BUILTIN_PLACEMENTS
          implementation = spec.get("implementation", {})
          path = f"ei/{layer_id}/{group_id}/{node_id}"
          self.node_outputs[path] = []
          config = {"instance": instance, "run_id": run_id, "revision": revision}
          emit = lambda value, path=path: self.emit_node(path, value)
          if built_in and instance["node_type"] in {
            "ei.cognition.state.arm_status@v0.2",
          }:
            root, builtin_implementation = builtin_library()
            self.custom_nodes[path] = NativeNode(
              root, builtin_implementation, config, emit
            )
          elif built_in and (
            instance["node_type"] == "ei.cognition.intent.context@v0.2"
          ):
            self.custom_nodes[path] = ContextStage(emit)
          elif built_in and (
            instance["node_type"] == "ei.cognition.intent.forwarder@v0.2"
          ):
            self.custom_nodes[path] = ForwarderStage(emit)
          elif built_in and instance["node_type"] == "ei.io.link.gateway@v0.2":
            self.custom_nodes[path] = GatewayStage(emit, run_id)
          elif (
            built_in
            and instance["node_type"] == "ei.evidence.perception.admission@v0.2"
          ):
            self.custom_nodes[path] = AdmissionStage(emit)
          elif (
            built_in and instance["node_type"] == "ei.cognition.context.builder@v0.2"
          ):
            self.custom_nodes[path] = BuilderStage(emit)
          elif built_in and instance["node_type"] == "ei.cognition.missions.queue@v0.2":
            self.custom_nodes[path] = QueueStage(emit)
          elif (
            built_in and instance["node_type"] == "ei.cognition.decision.planner@v0.2"
          ):
            self.custom_nodes[path] = PlannerStage(emit)
          elif (
            built_in and instance["node_type"] == "ei.governance.monitor.health@v0.2"
          ):
            self.custom_nodes[path] = HealthMonitorStage(emit)
          elif built_in and instance["node_type"] == "ei.governance.safety.gate@v0.2":
            self.custom_nodes[path] = SafetyGateStage(emit)
          elif built_in:
            self.custom_nodes[path] = OwnerStage(emit)
          elif implementation["kind"] == "native_plugin":
            self.custom_nodes[path] = NativeNode(
              spec["_package_root"], implementation, config, emit
            )
          else:
            self.custom_nodes[path] = ExternalNode(
              spec["_package_root"], spec["_entrypoint"], config, emit
            )
          if not self.custom_nodes[path].ready():
            raise ValueError("EI node is not ready")
    for layer_id, layer in graph["cores"]["ei"]["layers"].items():
      for group_id, group in layer["groups"].items():
        for node_id, instance in group["nodes"].items():
          target = f"ei/{layer_id}/{group_id}/{node_id}"
          if target not in self.custom_nodes:
            continue
          for input_name, link in instance.get("inputs", {}).items():
            source = link.get("node")
            if source in self.custom_nodes:
              self.node_edges.setdefault(source, []).append(
                (target, input_name, link["output"])
              )
            elif (
              isinstance(source, str)
              and source.startswith("eos/")
              and instance["node_type"] not in BUILTIN_PLACEMENTS
            ):
              self.cross_inputs[(target, input_name)] = (source, link["output"])

    for edge in cross_edges or []:
      if not isinstance(edge, dict) or set(edge) != {
        "source",
        "output",
        "target",
        "input",
      }:
        raise ValueError("invalid EI to EOS edge declaration")
      source, output, target, input_name = (
        edge[key] for key in ("source", "output", "target", "input")
      )
      if (
        not all(
          isinstance(value, str) for value in (source, output, target, input_name)
        )
        or source not in self.custom_nodes
        or source not in self.node_outputs
        or not target.startswith("eos/")
      ):
        raise ValueError("EI to EOS edge is not bound to a live node")
      source_type = _type_at_path(graph, source)
      if source_type in BUILTIN_PLACEMENTS:
        continue  # Demo safety-to-actuator delivery remains on /v1/activate.
      self.cross_outputs.setdefault(source, []).append((target, input_name, output))

  def server_close(self) -> None:
    self.stop_monitor()
    for node in reversed(list(self.custom_nodes.values())):
      try:
        node.stop()
      except (OSError, TimeoutError, ValueError):
        pass
    self.custom_nodes.clear()
    self.node_edges.clear()
    self.cross_inputs.clear()
    self.cross_outputs.clear()
    if self.worker is not None and self.worker.poll() is None:
      # A local owner signal takes the same graceful worker stop lane as the
      # normal Stop request, regardless of the saved run identity.
      _signal_group(self.worker.pid, signal.SIGINT)
      try:
        self.worker.wait(timeout=5)
      except subprocess.TimeoutExpired:
        _signal_group(self.worker.pid, signal.SIGTERM)
        try:
          self.worker.wait(timeout=2)
        except subprocess.TimeoutExpired:
          _signal_group(self.worker.pid, signal.SIGKILL)
          self.worker.wait(timeout=2)
    super().server_close()

  def stop_monitor(self) -> None:
    self.monitor_stop.set()
    if self.monitor_thread is not None:
      self.monitor_thread.join(timeout=6)
      self.monitor_thread = None
    with self.monitor_lock:
      self.monitor_latest = None
      self.monitor_polled_at = 0.0
      self.monitor_error = None

  def start_monitor(self) -> None:
    if not _monitor_path(self.activation_graph) or self.eos_owner_path is None:
      return
    self.monitor_stop.clear()
    self.monitor_thread = threading.Thread(
      target=_refresh_monitor, args=(self,), daemon=True
    )
    self.monitor_thread.start()


class _Handler(socketserver.StreamRequestHandler):
  def setup(self) -> None:
    super().setup()
    self.connection.settimeout(10)

  def _reply(self, status: int, value: dict[str, Any]) -> None:
    body = json.dumps(value, ensure_ascii=False, separators=(",", ":")).encode()
    if len(body) > MAX_REPLY_BYTES:
      status, body = 500, b'{"error":"response_too_large"}'
    reason = {
      200: "OK",
      400: "Bad Request",
      409: "Conflict",
      404: "Not Found",
      405: "Method Not Allowed",
      411: "Length Required",
      413: "Payload Too Large",
      422: "Unprocessable Content",
      431: "Request Header Fields Too Large",
      501: "Not Implemented",
      503: "Service Unavailable",
    }.get(status, "Internal Server Error")
    self.wfile.write(
      f"HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\n"
      f"Content-Length: {len(body)}\r\nConnection: close\r\n\r\n".encode()
    )
    self.wfile.write(body)
    self.wfile.flush()

  def handle(self) -> None:
    try:
      self._handle()
    except (OSError, TimeoutError, UnicodeError):
      return

  def _handle(self) -> None:
    head = bytearray()
    while not head.endswith(b"\r\n\r\n"):
      if len(head) >= MAX_HEADER_BYTES:
        return self._reply(431, {"error": "headers_too_large"})
      octet = self.rfile.read(1)
      if not octet:
        return
      head.extend(octet)
    lines = head.decode("ascii").split("\r\n")
    parts = lines[0].split()
    if len(parts) != 3 or parts[2] != "HTTP/1.1":
      return self._reply(400, {"error": "invalid_http"})
    method, path, _ = parts
    headers: dict[str, str] = {}
    for line in lines[1:]:
      if not line:
        continue
      if ":" not in line:
        return self._reply(400, {"error": "invalid_header"})
      name, value = line.split(":", 1)
      key = name.lower()
      if key in headers:
        return self._reply(400, {"error": "duplicate_header"})
      headers[key] = value.strip()
    if "transfer-encoding" in headers:
      return self._reply(400, {"error": "transfer_encoding_unsupported"})
    if method == "GET" and path == "/v1/health":
      server: _OwnerServer = self.server  # type: ignore[assignment]
      worker = server.worker
      state = (
        "idle" if worker is None else ("running" if worker.poll() is None else "exited")
      )
      return self._reply(
        200,
        {
          "schema_version": 1,
          "core": "ei",
          "state": state,
          "run_id": server.run_id,
          "revision": server.revision,
          "setup_schema_version": SETUP_SCHEMA_VERSION,
        },
      )
    if method == "GET" and path.startswith("/v1/logs"):
      prefix = "/v1/logs?after="
      cursor = path.removeprefix(prefix) if path.startswith(prefix) else ""
      if not cursor.isascii() or not cursor.isdecimal() or len(cursor) > 20:
        return self._reply(400, {"error": "invalid_log_cursor"})
      try:
        return self._reply(200, _read_core_logs(int(cursor)))
      except OSError:
        return self._reply(503, {"error": "owner_logs_unavailable"})
    if method != "POST":
      return self._reply(405, {"error": "method_not_allowed"})
    try:
      length = int(headers["content-length"])
    except KeyError:
      return self._reply(411, {"error": "length_required"})
    except ValueError:
      return self._reply(400, {"error": "invalid_length"})
    if length < 0:
      return self._reply(400, {"error": "invalid_length"})
    if length > MAX_SETUP_INPUT_BYTES:
      return self._reply(413, {"error": "request_too_large"})
    body = self.rfile.read(length)
    if len(body) != length:
      return self._reply(400, {"error": "truncated_body"})
    if path == "/v1/apply":
      return self._reply(
        501,
        {
          "schema_version": 1,
          "core": "ei",
          "ok": False,
          "error": "lifecycle_handoff_unavailable",
        },
      )
    op = path.removeprefix("/v1/")
    if path not in {
      "/v1/identity",
      "/v1/catalog",
      "/v1/options",
      "/v1/validate",
      "/v1/render",
      "/v1/run",
      "/v1/stop",
      "/v1/status",
      "/v1/activate",
      "/v1/result",
      "/v1/monitor",
      "/v1/node-input",
      "/v1/input",
    }:
      return self._reply(404, {"error": "not_found"})
    try:
      request = json.loads(body)
    except (ValueError, UnicodeDecodeError):
      return self._reply(400, {"error": "invalid_json"})
    if (
      not isinstance(request, dict)
      or request.get("schema_version") != 1
      or request.get("core") != "ei"
      or request.get("op") != op
    ):
      return self._reply(400, {"error": "invalid_envelope"})
    if op in {"run", "stop", "status"}:
      status, reply = self._lifecycle(op, request)
      return self._reply(status, reply)
    if op == "activate":
      status, reply = self._activate(request)
      return self._reply(status, reply)
    if op == "result":
      status, reply = self._result(request)
      return self._reply(status, reply)
    if op == "monitor":
      status, reply = self._monitor(request)
      return self._reply(status, reply)
    if op == "node-input":
      status, reply = self._node_input(request)
      return self._reply(status, reply)
    if op == "input":
      status, reply = self._input(request)
      return self._reply(status, reply)
    request["schema_version"] = SETUP_SCHEMA_VERSION
    try:
      reply = _setup_reply(_validate_setup_request(request))
    except SetupDiagnostic as exc:
      reply = {
        "schema_version": SETUP_SCHEMA_VERSION,
        "ok": False,
        "diagnostics": [
          {"path": exc.path, "code": exc.code, "message": str(exc)[:512]}
        ],
        "files": [],
      }
    except (ValidationError, TypeError, ValueError) as exc:
      reply = {
        "schema_version": SETUP_SCHEMA_VERSION,
        "ok": False,
        "diagnostics": [
          {"path": "$", "code": "invalid_input", "message": str(exc)[:512]}
        ],
        "files": [],
      }
    return self._reply(200 if reply["ok"] else 422, reply)

  def _lifecycle(self, op: str, request: dict[str, Any]) -> tuple[int, dict[str, Any]]:
    run_id = request.get("run_id")
    revision = request.get("revision")
    if (
      not isinstance(run_id, str)
      or not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]{0,127}", run_id)
      or not isinstance(revision, str)
      or not re.fullmatch(r"[0-9a-f]{64}", revision)
    ):
      return 400, {"error": "invalid_run_identity"}
    server: _OwnerServer = self.server  # type: ignore[assignment]
    if op == "run":
      if server.run_id is not None:
        _core_log(
          "warn", "Run rejected: owner already has a selected run", run_id=run_id
        )
        return 409, {"error": "owner_run_already_selected"}
      if (run_id, revision) in server.previous_runs:
        return 409, {"error": "stale_run_identity"}
      launch = request.get("launch")
      if not isinstance(launch, dict) or set(launch) != {"sha256", "args", "env"}:
        return 400, {"error": "invalid_launch"}
      args, env = launch["args"], launch["env"]
      if (
        not isinstance(args, list)
        or len(args) > 64
        or any(
          not isinstance(arg, str) or len(arg) > 1024 or "\x00" in arg for arg in args
        )
        or not isinstance(env, dict)
        or len(env) > 64
        or any(
          not isinstance(key, str)
          or not re.fullmatch(r"[A-Z][A-Z0-9_]{0,63}", key)
          or not isinstance(value, str)
          or len(value) > 4096
          or "\x00" in value
          for key, value in env.items()
        )
      ):
        return 400, {"error": "invalid_launch"}
      setup = {
        key: request.get(key) for key in ("revision", "graph", "endpoints", "packages")
      }
      setup.update(
        {"schema_version": SETUP_SCHEMA_VERSION, "core": "ei", "op": "validate"}
      )
      try:
        verified = _setup_reply(_validate_setup_request(setup))
      except (SetupDiagnostic, ValidationError, TypeError, ValueError):
        return 422, {"error": "owner_graph_rejected"}
      if not verified["ok"]:
        return 422, {"error": "owner_graph_rejected"}
      endpoints = request.get("endpoints")
      eos_owner = endpoints.get("eos_owner") if isinstance(endpoints, dict) else None
      eos_path = eos_owner.get("path") if isinstance(eos_owner, dict) else None
      if eos_owner is not None and (
        eos_owner.get("transport") != "uds"
        or not isinstance(eos_path, str)
        or not Path(eos_path).is_absolute()
      ):
        return 400, {"error": "invalid_eos_owner_endpoint"}
      monitor_joint_names = None
      if _has_monitor_nodes(request["graph"]):
        monitor = endpoints.get("arm_monitor") if isinstance(endpoints, dict) else None
        names = monitor.get("joint_names") if isinstance(monitor, dict) else None
        if (
          not isinstance(monitor, dict)
          or set(monitor) != {"joint_names"}
          or not isinstance(names, list)
          or len(names) != 6
          or any(
            not isinstance(name, str)
            or not 1 <= len(name) <= 128
            or any(ord(ch) < 32 for ch in name)
            for name in names
          )
          or len(set(names)) != 6
        ):
          return 400, {"error": "invalid_arm_monitor_names"}
        monitor_joint_names = names
      directory = Path(
        os.environ.get("ISOS_EI_BIN_DIR", Path(__file__).parents[2] / "bin")
      )
      program = directory / "ei"
      try:
        digest = hashlib.sha256(program.read_bytes()).hexdigest()
      except OSError:
        return 503, {"error": "owner_worker_unavailable"}
      if launch["sha256"] != digest:
        return 422, {"error": "worker_digest_mismatch"}
      if env.get("ISOS_RUN_ID", run_id) != run_id:
        return 422, {"error": "worker_run_identity_mismatch"}
      try:
        worker = subprocess.Popen(
          [str(program), *args, "--managed", "--health-only"],
          env={**os.environ, **env, "ISOS_RUN_ID": run_id},
          stdin=subprocess.DEVNULL,
          stdout=subprocess.DEVNULL,
          stderr=subprocess.DEVNULL,
          start_new_session=True,
        )
      except OSError:
        _core_log("error", "Run failed: EI worker unavailable", run_id=run_id)
        return 503, {"error": "owner_worker_unavailable"}
      server.worker, server.run_id, server.revision, server.terminal = (
        worker,
        run_id,
        revision,
        None,
      )
      server.eos_owner_path = eos_path
      server.activation_graph = request["graph"]
      server.monitor_latest = None
      server.monitor_polled_at = 0.0
      server.monitor_joint_names = monitor_joint_names
      server.monitor_forward_interval = 1.0 / _monitor_forward_rate(request["graph"])
      try:
        cross_edges = (
          endpoints.get("ei_to_eos_edges", []) if isinstance(endpoints, dict) else []
        )
        if not isinstance(cross_edges, list):
          raise TypeError("invalid EI to EOS edge projection")
        server.start_nodes(
          request["graph"], request["packages"], run_id, revision, cross_edges
        )
      except (OSError, TimeoutError, ValueError, KeyError, TypeError):
        for node in reversed(list(server.custom_nodes.values())):
          try:
            node.stop()
          except (OSError, TimeoutError, ValueError):
            pass
        server.custom_nodes.clear()
        server.node_outputs.clear()
        server.node_edges.clear()
        server.cross_inputs.clear()
        server.cross_outputs.clear()
        _signal_group(worker.pid, signal.SIGINT)
        try:
          worker.wait(timeout=5)
        except subprocess.TimeoutExpired:
          _signal_group(worker.pid, signal.SIGKILL)
          worker.wait(timeout=2)
        server.worker = None
        server.run_id = None
        server.revision = None
        return 422, {"error": "owner_node_start_failed"}
      server.start_monitor()
      _core_log("info", "EI health worker started", run_id=run_id, pid=worker.pid)
      return 200, {
        "schema_version": 1,
        "core": "ei",
        "ok": True,
        "run_id": run_id,
        "revision": revision,
        "state": "starting",
        "pid": worker.pid,
      }
    if run_id != server.run_id or revision != server.revision:
      return 409, {"error": "run_identity_mismatch"}
    if op == "status":
      worker = server.worker
      state = "running" if worker is not None and worker.poll() is None else "exited"
      if state == "exited" and server.terminal is None:
        server.terminal = {
          "outcome": "unknown",
          "reason_code": "ei_worker_exited_unexpectedly",
          "run_id": run_id,
          "revision": revision,
          "worker_exited": True,
        }
        _core_log("error", "EI health worker exited unexpectedly", run_id=run_id)
      return 200, {
        "schema_version": 1,
        "core": "ei",
        "ok": True,
        "run_id": run_id,
        "revision": revision,
        "state": state,
        "terminal": server.terminal,
      }
    worker = server.worker
    server.stop_monitor()
    try:
      for node in reversed(list(server.custom_nodes.values())):
        node.stop()
    except (OSError, TimeoutError, ValueError):
      return 503, {"error": "ei_node_stop_unproven"}
    server.custom_nodes.clear()
    server.node_outputs.clear()
    server.node_edges.clear()
    server.cross_inputs.clear()
    server.cross_outputs.clear()
    if worker is not None and worker.poll() is None:
      _signal_group(worker.pid, signal.SIGINT)
      try:
        worker.wait(timeout=5)
      except subprocess.TimeoutExpired:
        _signal_group(worker.pid, signal.SIGTERM)
        try:
          worker.wait(timeout=2)
        except subprocess.TimeoutExpired:
          _signal_group(worker.pid, signal.SIGKILL)
          worker.wait(timeout=2)
    server.terminal = {
      "outcome": "stopped",
      "reason_code": "ei_worker_stopped",
      "run_id": run_id,
      "revision": revision,
      "worker_exited": worker is None or worker.poll() is not None,
    }
    server.previous_runs.add((run_id, revision))
    server.run_id = None
    server.revision = None
    server.worker = None
    server.eos_owner_path = None
    server.activation_graph = None
    server.monitor_latest = None
    server.monitor_polled_at = 0.0
    server.monitor_joint_names = None
    server.monitor_forward_interval = 0.05
    _core_log("info", "EI health worker stopped", run_id=run_id)
    return 200, {
      "schema_version": 1,
      "core": "ei",
      "ok": True,
      "terminal": server.terminal,
    }

  def _input(self, request: dict[str, Any]) -> tuple[int, dict[str, Any]]:
    """Admit one EOS→EI datum only on an authored, run-fenced EI input."""
    if set(request) != {
      "schema_version",
      "core",
      "op",
      "run_id",
      "revision",
      "node",
      "port",
      "payload",
      "source_node",
      "source_port",
    }:
      return 400, {"error": "invalid_node_input"}
    server: _OwnerServer = self.server  # type: ignore[assignment]
    if (
      request.get("run_id") != server.run_id
      or request.get("revision") != server.revision
    ):
      return 409, {"error": "run_identity_mismatch"}
    path, port, payload = (
      request.get("node"),
      request.get("port"),
      request.get("payload"),
    )
    if (
      not isinstance(path, str)
      or not isinstance(port, str)
      or not isinstance(payload, dict)
    ):
      return 400, {"error": "invalid_node_input"}
    if (
      (path, port) not in server.cross_inputs
      or path not in server.custom_nodes
      or server.cross_inputs[(path, port)]
      != (request.get("source_node"), request.get("source_port"))
    ):
      return 422, {"error": "node_input_route_unavailable"}
    node = server.custom_nodes[path]
    if not node.ready():
      return 503, {"error": "node_unready"}
    server.node_outputs[path].clear()
    try:
      node.input({"port": port, "payload": payload})
    except (OSError, TimeoutError, TypeError, ValueError):
      return 503, {"error": "node_input_failed"}
    outputs = []
    for output in server.node_outputs[path]:
      out_port = output.get("port")
      data = output.get("payload", output.get("data"))
      if isinstance(out_port, str) and isinstance(data, dict):
        outputs.append({"node": path, "output": {"port": out_port, "payload": data}})
    return 200, {
      "schema_version": 1,
      "core": "ei",
      "ok": True,
      "run_id": server.run_id,
      "revision": server.revision,
      "outputs": outputs,
    }

  def _node_input(self, request: dict[str, Any]) -> tuple[int, dict[str, Any]]:
    if set(request) != {
      "schema_version",
      "core",
      "op",
      "run_id",
      "revision",
      "node",
      "data",
    }:
      return 400, {"error": "invalid_node_input"}
    server: _OwnerServer = self.server  # type: ignore[assignment]
    if (
      request.get("run_id") != server.run_id
      or request.get("revision") != server.revision
    ):
      return 409, {"error": "run_identity_mismatch"}
    path = request.get("node")
    if (
      not isinstance(path, str)
      or path not in server.custom_nodes
      or not isinstance(request.get("data"), dict)
    ):
      return 422, {"error": "node_input_unavailable"}
    node = server.custom_nodes[path]
    if not node.ready():
      return 503, {"error": "node_unready"}
    server.node_outputs[path].clear()
    try:
      node.input(request["data"])
    except (OSError, TimeoutError, TypeError, ValueError):
      return 503, {"error": "node_input_failed"}
    return 200, {
      "schema_version": 1,
      "core": "ei",
      "ok": True,
      "run_id": server.run_id,
      "revision": server.revision,
      "outputs": list(server.node_outputs[path]),
    }

  def _seed_cognition(self, event_id: str, source: str) -> None:
    """Drive the planner chain from the same button activation (best effort)."""
    server: _OwnerServer = self.server  # type: ignore[assignment]
    seeds = (
      (
        "ei.io.link.gateway@v0.2",
        "resource",
        {"kind": "activation", "event_id": event_id},
      ),
      (
        "ei.cognition.missions.queue@v0.2",
        "intent",
        {"mission_id": event_id, "text": f"move arm from {source}"},
      ),
    )
    for node_type, port, data in seeds:
      path = _node_path(server.activation_graph, node_type)
      if path is None or path not in server.custom_nodes:
        continue
      try:
        server.custom_nodes[path].input({"port": port, "data": data})
      except (OSError, TimeoutError, TypeError, ValueError):
        _core_log(
          "info", "EI cognition seed skipped", event_id=event_id, node=node_type
        )

  def _activate(self, request: dict[str, Any]) -> tuple[int, dict[str, Any]]:
    """Pass one fenced activation through the declared EI stages to EOS."""
    run_id = request.get("run_id")
    revision = request.get("revision")
    event_id = request.get("event_id")
    source = request.get("source_node")
    if (
      set(request)
      != {
        "schema_version",
        "core",
        "op",
        "run_id",
        "revision",
        "source_node",
        "event_id",
        "kind",
      }
      or not isinstance(run_id, str)
      or re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]{0,127}", run_id) is None
      or not isinstance(revision, str)
      or re.fullmatch(r"[0-9a-f]{64}", revision) is None
      or not isinstance(event_id, str)
      or re.fullmatch(r"[A-Za-z0-9_.-]{1,64}", event_id) is None
      or not isinstance(source, str)
      or re.fullmatch(r"[A-Za-z0-9_.-]{1,48}(?:/[A-Za-z0-9_.-]{1,48}){3}", source)
      is None
      or request.get("kind") != "activate"
    ):
      return 400, {"error": "invalid_activation"}
    server: _OwnerServer = self.server  # type: ignore[assignment]
    if run_id != server.run_id or revision != server.revision:
      return 409, {"error": "run_identity_mismatch"}
    if server.worker is None or server.worker.poll() is not None:
      return 503, {"error": "owner_worker_unavailable"}
    if not _activation_path(server.activation_graph, source):
      return 422, {"error": "activation_path_invalid"}
    if server.eos_owner_path is None:
      return 503, {"error": "eos_owner_unavailable"}
    ingress = _node_path(server.activation_graph, "ei.io.input.trigger@v0.2")
    egress = _node_path(server.activation_graph, "ei.governance.safety.gate@v0.2")
    if (
      ingress is None
      or egress is None
      or ingress not in server.custom_nodes
      or egress not in server.custom_nodes
    ):
      return 503, {"error": "activation_node_unavailable"}
    event = {"event_id": event_id, "kind": "activate"}
    server.node_outputs[egress].clear()
    self._seed_cognition(event_id, source)
    try:
      server.custom_nodes[ingress].input({"port": "activation", "data": event})
    except (OSError, TimeoutError, TypeError, ValueError):
      return 503, {"error": "activation_node_failed"}
    if not any(
      output.get("port") == "activation"
      and output.get("payload", output.get("data")) == event
      for output in server.node_outputs[egress]
    ):
      return 503, {"error": "activation_node_output_missing"}
    proposal = getattr(server.custom_nodes.get(egress), "_latest_proposal", None)
    if proposal is not None:
      _core_log(
        "info",
        "EI cognition proposal observed",
        run_id=run_id,
        event_id=event_id,
        proposal=proposal,
      )
    forwarded = dict(request)
    forwarded["core"] = "eos"
    try:
      status, result = _post_owner(server.eos_owner_path, "/v1/activate", forwarded)
    except (OSError, TimeoutError, ValueError, TypeError):
      _core_log(
        "error", "EOS activation outcome unknown", run_id=run_id, event_id=event_id
      )
      return 503, {"error": "eos_outcome_unknown"}
    if status != 200 or not isinstance(result, dict):
      return status, result if isinstance(result, dict) else {
        "error": "eos_invalid_result"
      }
    if (
      result.get("run_id") != run_id
      or result.get("revision") != revision
      or result.get("event_id") != event_id
      or result.get("status") != "accepted"
      or result.get("ok") is not True
      or not isinstance(result.get("command_id"), str)
    ):
      return 503, {"error": "eos_result_fence_mismatch"}
    _core_log("info", "EI activation forwarded", run_id=run_id, event_id=event_id)
    return 200, {
      **result,
      "schema_version": 1,
      "core": "ei",
      "run_id": run_id,
      "revision": revision,
      "event_id": event_id,
    }

  def _result(self, request: dict[str, Any]) -> tuple[int, dict[str, Any]]:
    if set(request) != {
      "schema_version",
      "core",
      "op",
      "run_id",
      "revision",
      "event_id",
    }:
      return 400, {"error": "invalid_result_request"}
    run_id, revision, event_id = (
      request.get("run_id"),
      request.get("revision"),
      request.get("event_id"),
    )
    if (
      not isinstance(run_id, str)
      or re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]{0,127}", run_id) is None
      or not isinstance(revision, str)
      or re.fullmatch(r"[0-9a-f]{64}", revision) is None
      or not isinstance(event_id, str)
      or re.fullmatch(r"[A-Za-z0-9_.-]{1,64}", event_id) is None
    ):
      return 400, {"error": "invalid_result_request"}
    server: _OwnerServer = self.server  # type: ignore[assignment]
    if run_id != server.run_id or revision != server.revision:
      return 409, {"error": "run_identity_mismatch"}
    if server.worker is None or server.worker.poll() is not None:
      return 503, {"error": "owner_worker_unavailable"}
    if server.eos_owner_path is None:
      return 503, {"error": "eos_owner_unavailable"}
    try:
      status, result = _post_owner(
        server.eos_owner_path, "/v1/result", {**request, "core": "eos"}
      )
    except (OSError, TimeoutError, ValueError, TypeError):
      return 503, {"error": "eos_result_unavailable"}
    if status != 200 or not isinstance(result, dict):
      return status, result if isinstance(result, dict) else {
        "error": "eos_invalid_result"
      }
    if (
      result.get("ok") is not True
      or result.get("run_id") != run_id
      or result.get("revision") != revision
      or result.get("event_id") != event_id
      or not isinstance(result.get("command_id"), str)
      or result.get("status")
      not in {
        "accepted",
        "running",
        "succeeded",
        "failed",
        "cancelled",
        "outcome_unknown",
      }
    ):
      return 503, {"error": "eos_result_fence_mismatch"}
    return 200, {**result, "core": "ei"}

  def _monitor(self, request: dict[str, Any]) -> tuple[int, dict[str, Any]]:
    """Read one EOS arm state through the declared EI observation path."""
    if set(request) != {"schema_version", "core", "op", "run_id", "revision"}:
      return 400, {"error": "invalid_monitor_request"}
    run_id, revision = request.get("run_id"), request.get("revision")
    if (
      not isinstance(run_id, str)
      or re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]{0,127}", run_id) is None
      or not isinstance(revision, str)
      or re.fullmatch(r"[0-9a-f]{64}", revision) is None
    ):
      return 400, {"error": "invalid_monitor_request"}
    server: _OwnerServer = self.server  # type: ignore[assignment]
    if run_id != server.run_id or revision != server.revision:
      return 409, {"error": "run_identity_mismatch"}
    if server.worker is None or server.worker.poll() is not None:
      return 503, {"error": "owner_worker_unavailable"}
    if not _monitor_path(server.activation_graph):
      return 422, {"error": "monitor_path_invalid"}
    with server.monitor_lock:
      observation = server.monitor_latest
      polled_at = server.monitor_polled_at
      error = server.monitor_error
    if error is not None:
      return 503, {"error": error}
    if observation is None or time.monotonic() - polled_at > max(
      1.0, 3 * server.monitor_forward_interval
    ):
      return 503, {"error": "eos_monitor_unavailable"}
    return 200, {
      "schema_version": 1,
      "core": "ei",
      "ok": True,
      "run_id": run_id,
      "revision": revision,
      "observation": observation,
    }


def _refresh_monitor(server: _OwnerServer) -> None:
  """Poll EOS once per interval, independently of any IsOS reader."""
  run_id, revision, path = server.run_id, server.revision, server.eos_owner_path
  if run_id is None or revision is None or path is None:
    return
  while not server.monitor_stop.is_set():
    started = time.monotonic()
    error: str | None = None
    observation: dict[str, Any] | None = None
    try:
      status, result = _post_owner(
        path,
        "/v1/monitor",
        {
          "schema_version": 1,
          "core": "eos",
          "op": "monitor",
          "run_id": run_id,
          "revision": revision,
        },
      )
      if status != 200 or result.get("ok") is not True:
        error = "eos_monitor_rejected"
      else:
        candidate = result.get("observation")
        if (
          result.get("run_id") != run_id
          or result.get("revision") != revision
          or not _valid_monitor_observation(
            candidate, run_id, revision, server.monitor_joint_names
          )
        ):
          error = "eos_monitor_invalid"
        elif isinstance(candidate, dict):
          ingress = _node_path(server.activation_graph, "ei.io.input.arm_status@v0.2")
          egress = _node_path(
            server.activation_graph, "ei.governance.monitor.arm_status@v0.2"
          )
          if (
            ingress is None
            or egress is None
            or ingress not in server.custom_nodes
            or egress not in server.custom_nodes
          ):
            error = "ei_monitor_node_unavailable"
          else:
            server.node_outputs[egress].clear()
            server.custom_nodes[ingress].input(
              {"port": "observation", "data": candidate}
            )
            if any(
              output.get("port") == "observation"
              and output.get("payload", output.get("data")) == candidate
              for output in server.node_outputs[egress]
            ):
              observation = candidate
            else:
              error = "ei_monitor_node_output_missing"
    except (OSError, TimeoutError, ValueError, TypeError):
      error = "eos_monitor_unavailable"
    with server.monitor_lock:
      if (
        server.monitor_stop.is_set()
        or server.run_id != run_id
        or server.revision != revision
      ):
        break
      previous = server.monitor_latest
      if (
        observation is not None
        and previous is not None
        and observation["sequence"] < previous["sequence"]
      ):
        error = "eos_monitor_sequence_regressed"
      if error is None:
        server.monitor_latest = observation
        server.monitor_polled_at = time.monotonic()
      server.monitor_error = error
    server.monitor_stop.wait(
      max(0.0, server.monitor_forward_interval - (time.monotonic() - started))
    )


def _type_at_path(graph: dict[str, Any], path: str) -> str | None:
  parts = path.split("/")
  if len(parts) != 4 or parts[0] != "ei":
    return None
  node = (
    graph.get("cores", {})
    .get("ei", {})
    .get("layers", {})
    .get(parts[1], {})
    .get("groups", {})
    .get(parts[2], {})
    .get("nodes", {})
    .get(parts[3], {})
  )
  return node.get("node_type") if isinstance(node, dict) else None


def _node_path(graph: dict[str, Any] | None, type_id: str) -> str | None:
  if not isinstance(graph, dict):
    return None
  layers = graph.get("cores", {}).get("ei", {}).get("layers", {})
  for layer_id, layer in layers.items():
    for group_id, group in layer.get("groups", {}).items():
      for node_id, node in group.get("nodes", {}).items():
        if isinstance(node, dict) and node.get("node_type") == type_id:
          return f"ei/{layer_id}/{group_id}/{node_id}"
  return None


def _activation_path(graph: dict[str, Any] | None, source: str) -> bool:
  if not isinstance(graph, dict):
    return False
  core = graph.get("cores", {}).get("ei", {})
  layers = core.get("layers", {}) if isinstance(core, dict) else {}
  stages = (
    ("io", "input", "trigger"),
    ("cognition", "intent", "context"),
    ("cognition", "intent", "forwarder"),
    ("governance", "safety", "gate"),
  )
  previous = source
  for layer, group, name in stages:
    nodes = layers.get(layer, {}).get("groups", {}).get(group, {}).get("nodes", {})
    matching = (
      [
        (node_id, node)
        for node_id, node in nodes.items()
        if isinstance(node, dict)
        and node.get("node_type") == f"ei.{layer}.{group}.{name}@v0.2"
      ]
      if isinstance(nodes, dict)
      else []
    )
    if len(matching) != 1:
      return False
    node_id, node = matching[0]
    if node.get("inputs", {}).get("activation") != {
      "node": previous,
      "output": "activation",
    }:
      return False
    previous = f"ei/{layer}/{group}/{node_id}"
  return True


def _monitor_path(graph: dict[str, Any] | None) -> bool:
  if not isinstance(graph, dict):
    return False
  core = graph.get("cores", {}).get("ei", {})
  layers = core.get("layers", {}) if isinstance(core, dict) else {}
  stages = (("io", "input"), ("cognition", "state"), ("governance", "monitor"))
  previous = "eos/interface/egress/arm_status"
  for layer, group in stages:
    nodes = layers.get(layer, {}).get("groups", {}).get(group, {}).get("nodes", {})
    matching = (
      [
        (node_id, node)
        for node_id, node in nodes.items()
        if isinstance(node, dict)
        and node.get("node_type") == f"ei.{layer}.{group}.arm_status@v0.2"
      ]
      if isinstance(nodes, dict)
      else []
    )
    if len(matching) != 1:
      return False
    node_id, node = matching[0]
    if node.get("inputs", {}).get("observation") != {
      "node": previous,
      "output": "observation",
    }:
      return False
    previous = f"ei/{layer}/{group}/{node_id}"
  return True


def _has_monitor_nodes(graph: dict[str, Any]) -> bool:
  layers = graph.get("cores", {}).get("ei", {}).get("layers", {})
  return any(
    isinstance(node, dict) and node.get("node_type", "").endswith(".arm_status@v0.2")
    for layer in layers.values()
    for group in layer.get("groups", {}).values()
    for node in group.get("nodes", {}).values()
  )


def _monitor_forward_rate(graph: dict[str, Any]) -> float:
  nodes = (
    graph.get("cores", {})
    .get("ei", {})
    .get("layers", {})
    .get("governance", {})
    .get("groups", {})
    .get("monitor", {})
    .get("nodes", {})
  )
  for node in nodes.values():
    if (
      isinstance(node, dict)
      and node.get("node_type") == "ei.governance.monitor.arm_status@v0.2"
    ):
      return float(node.get("settings", {}).get("forward_rate_hz", 20))
  return 20.0


def _valid_monitor_observation(
  value: Any, run_id: str, revision: str, joint_names: list[str] | None
) -> bool:
  if not isinstance(value, dict) or set(value) != {
    "run_id",
    "revision",
    "source_node",
    "sequence",
    "sampled_at_monotonic_ns",
    "joint_names",
    "joint_positions_rad",
    "state",
    "reason_code",
  }:
    return False
  state = value.get("state")
  positions = value.get("joint_positions_rad")
  return (
    value.get("run_id") == run_id
    and value.get("revision") == revision
    and value.get("source_node")
    in {"eos/hardware/mock/arm", "eos/hardware/mock/arm_observer"}
    and type(value.get("sequence")) is int
    and value["sequence"] >= 0
    and type(value.get("sampled_at_monotonic_ns")) is int
    and value["sampled_at_monotonic_ns"] >= 0
    and joint_names is not None
    and value.get("joint_names") == joint_names
    and state in {"ready", "degraded", "stale", "stopped"}
    and (
      value.get("reason_code") is None
      if state == "ready"
      else isinstance(value.get("reason_code"), str)
    )
    and (
      isinstance(positions, list)
      and len(positions) == 6
      and all(
        type(position) in (int, float) and math.isfinite(position)
        for position in positions
      )
      if state == "ready"
      else positions is None
    )
  )


def _post_owner(
  path: str, route: str, payload: dict[str, Any]
) -> tuple[int, dict[str, Any]]:
  body = json.dumps(payload, separators=(",", ":")).encode()
  with socket.socket(socket.AF_UNIX) as client:
    client.settimeout(5)
    client.connect(path)
    client.sendall(
      f"POST {route} HTTP/1.1\r\nHost: owner\r\nContent-Length: {len(body)}\r\nConnection: close\r\n\r\n".encode()
      + body
    )
    response = bytearray()
    while chunk := client.recv(4096):
      response.extend(chunk)
      if len(response) > MAX_REPLY_BYTES + MAX_HEADER_BYTES:
        raise ValueError("EOS owner response oversized")
  head, raw = response.split(b"\r\n\r\n", 1)
  status = int(head.split(b" ", 2)[1])
  result = json.loads(raw)
  if not isinstance(result, dict):
    raise TypeError("EOS owner response is not an object")
  return status, result


def serve(socket_path: str) -> int:
  """Serve setup and a fenced worker lifecycle independently of IsOS."""
  path = Path(socket_path)
  if not path.is_absolute() or not path.parent.is_dir() or path.exists():
    raise ValueError(
      "owner socket must be a new absolute path in an existing directory"
    )
  old_umask = os.umask(0o177)
  try:
    server = _OwnerServer(socket_path, _Handler)
  finally:
    os.umask(old_umask)
  try:
    os.chmod(socket_path, 0o600)
    _core_log("info", "EI owner service listening")
    previous_handlers: dict[int, Any] = {}

    def request_shutdown(_signum: int, _frame: Any) -> None:
      # shutdown() must run outside the serve_forever thread. The signal
      # handler itself never waits for a worker or performs socket I/O.
      threading.Thread(target=server.shutdown, daemon=True).start()

    for signum in (signal.SIGINT, signal.SIGTERM):
      previous_handlers[signum] = signal.signal(signum, request_shutdown)
    server.serve_forever(poll_interval=0.2)
  finally:
    _core_log("info", "EI owner service stopping")
    for signum, handler in locals().get("previous_handlers", {}).items():
      signal.signal(signum, handler)
    server.server_close()
    path.unlink(missing_ok=True)
  return 0
