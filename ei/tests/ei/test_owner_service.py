"""The persistent owner service exposes bounded setup and fenced lifecycle."""

from __future__ import annotations

import hashlib
import json
import socket
import threading
import time
from pathlib import Path

from ei.owner_service import _Handler, _OwnerServer


def _exchange(path: Path, request: bytes) -> tuple[int, dict[str, object]]:
  with socket.socket(socket.AF_UNIX) as client:
    client.connect(str(path))
    client.sendall(request)
    client.shutdown(socket.SHUT_WR)
    response = bytearray()
    while part := client.recv(4096):
      response.extend(part)
  head, body = response.split(b"\r\n\r\n", 1)
  return int(head.split(b" ", 2)[1]), json.loads(body)


def _post(
  path: Path, route: str, body: dict[str, object]
) -> tuple[int, dict[str, object]]:
  encoded = json.dumps(body).encode()
  return _exchange(
    path,
    f"POST {route} HTTP/1.1\r\nHost: owner\r\nContent-Length: {len(encoded)}\r\n\r\n".encode()
    + encoded,
  )


def test_idle_owner_health_setup_and_closed_lifecycle(tmp_path: Path) -> None:
  path = tmp_path / "owner.sock"
  server = _OwnerServer(str(path), _Handler)
  worker = threading.Thread(target=server.serve_forever, daemon=True)
  worker.start()
  try:
    status, health = _exchange(path, b"GET /v1/health HTTP/1.1\r\nHost: owner\r\n\r\n")
    assert status == 200
    assert health == {
      "schema_version": 1,
      "core": "ei",
      "state": "idle",
      "setup_schema_version": 2,
    }
    status, identity = _post(
      path, "/v1/identity", {"schema_version": 1, "core": "ei", "op": "identity"}
    )
    assert status == 200
    assert identity["schema_version"] == 2
    assert identity["identity"]["core"] == "ei"
    status, unavailable = _post(
      path, "/v1/run", {"schema_version": 1, "core": "ei", "op": "run"}
    )
    assert status == 400
    assert unavailable["error"] == "invalid_run_identity"
    status, unavailable = _post(
      path,
      "/v1/run",
      {
        "schema_version": 1,
        "core": "ei",
        "op": "run",
        "run_id": "run_1",
        "revision": "a" * 64,
      },
    )
    assert status == 400
    assert unavailable["error"] == "invalid_launch"
    status, invalid = _post(
      path, "/v1/catalog", {"schema_version": 1, "core": "eos", "op": "catalog"}
    )
    assert status == 400
    assert invalid["error"] == "invalid_envelope"
  finally:
    server.shutdown()
    server.server_close()
    worker.join()


def test_catalog_uses_existing_setup_validation(tmp_path: Path) -> None:
  path = tmp_path / "owner.sock"
  server = _OwnerServer(str(path), _Handler)
  worker = threading.Thread(target=server.serve_forever, daemon=True)
  worker.start()
  try:
    status, reply = _post(
      path,
      "/v1/catalog",
      {
        "schema_version": 1,
        "core": "ei",
        "op": "catalog",
        "revision": "abc",
        "graph": {},
        "endpoints": {},
        "packages": [],
      },
    )
    assert status == 422
    assert reply["schema_version"] == 2
    assert reply["ok"] is False
    assert reply["files"] == []
    assert reply["diagnostics"] == [
      {
        "path": "/graph/version",
        "code": "unsupported_graph_version",
        "message": "EI requires graph 0.2.0",
      }
    ]
  finally:
    server.shutdown()
    server.server_close()
    worker.join()


def test_catalog_reports_node_mismatch_at_exact_graph_path(tmp_path: Path) -> None:
  path = tmp_path / "owner.sock"
  server = _OwnerServer(str(path), _Handler)
  worker = threading.Thread(target=server.serve_forever, daemon=True)
  worker.start()
  try:
    status, reply = _post(
      path,
      "/v1/catalog",
      {
        "schema_version": 1,
        "core": "ei",
        "op": "catalog",
        "revision": "abc",
        "graph": {
          "version": "0.2.0",
          "cores": {
            "ei": {
              "layers": {
                "cognition": {
                  "groups": {
                    "context": {
                      "nodes": {
                        "bad": {"node_type": "ei.cognition.context.retired@v0.2"}
                      }
                    }
                  }
                }
              }
            },
          },
        },
        "endpoints": {},
        "packages": [],
      },
    )
    assert status == 422
    assert reply["diagnostics"][0]["path"] == (
      "/graph/cores/ei/layers/cognition/groups/context/nodes/bad/node_type"
    )
    assert reply["diagnostics"][0]["code"] == "unsupported_node_type_version"
  finally:
    server.shutdown()
    server.server_close()
    worker.join()


def test_activation_follows_three_stages_and_fences_run(
  tmp_path: Path, monkeypatch
) -> None:
  from unittest.mock import Mock

  from ei import owner_service

  path = tmp_path / "owner.sock"
  server = _OwnerServer(str(path), _Handler)
  server.worker = Mock()
  server.worker.poll.return_value = None
  server.run_id = "run_1"
  server.revision = "a" * 64
  server.eos_owner_path = str(tmp_path / "eos.sock")
  source = "isos/ui/button/trigger"
  server.activation_graph = {
    "cores": {
      "ei": {
        "layers": {
          "io": {
            "groups": {
              "input": {
                "nodes": {
                  "input": {
                    "node_type": "ei.io.input.trigger@v0.2",
                    "inputs": {"activation": {"node": source, "output": "activation"}},
                  }
                }
              }
            }
          },
          "cognition": {
            "groups": {
              "intent": {
                "nodes": {
                  "forward": {
                    "node_type": "ei.cognition.intent.forwarder@v0.2",
                    "inputs": {
                      "activation": {
                        "node": "ei/io/input/input",
                        "output": "activation",
                      }
                    },
                  }
                }
              }
            }
          },
          "governance": {
            "groups": {
              "safety": {
                "nodes": {
                  "gate": {
                    "node_type": "ei.governance.safety.gate@v0.2",
                    "inputs": {
                      "activation": {
                        "node": "ei/cognition/intent/forward",
                        "output": "activation",
                      }
                    },
                  }
                }
              }
            }
          },
        }
      }
    }
  }
  calls: list[dict] = []

  def eos_reply(_path: str, _route: str, request: dict) -> tuple[int, dict]:
    calls.append(request)
    return 200, {
      "schema_version": 1,
      "core": "eos",
      "ok": True,
      "run_id": request["run_id"],
      "revision": request["revision"],
      "event_id": request["event_id"],
      "command_id": "cmd-1",
      "status": "accepted",
    }

  monkeypatch.setattr(owner_service, "_post_owner", eos_reply)
  worker = threading.Thread(target=server.serve_forever, daemon=True)
  worker.start()
  request = {
    "schema_version": 1,
    "core": "ei",
    "op": "activate",
    "run_id": "run_1",
    "revision": "a" * 64,
    "source_node": source,
    "event_id": "click-1",
    "kind": "activate",
  }
  try:
    status, reply = _post(path, "/v1/activate", request)
    assert status == 200
    assert reply["core"] == "ei"
    assert reply["status"] == "accepted"
    assert calls[0]["core"] == "eos"
    status, stale = _post(path, "/v1/activate", {**request, "revision": "b" * 64})
    assert status == 409 and stale["error"] == "run_identity_mismatch"
    status, invalid = _post(
      path, "/v1/activate", {**request, "source_node": "isos/ui/button/other"}
    )
    assert status == 422 and invalid["error"] == "activation_path_invalid"
    assert len(calls) == 1

    def eos_result(_path: str, route: str, forwarded: dict) -> tuple[int, dict]:
      assert route == "/v1/result" and forwarded["core"] == "eos"
      return 200, {
        "schema_version": 1,
        "core": "eos",
        "ok": True,
        "run_id": forwarded["run_id"],
        "revision": forwarded["revision"],
        "event_id": forwarded["event_id"],
        "command_id": "cmd-1",
        "status": "succeeded",
        "joint_positions_rad": [0.0] * 6,
      }

    monkeypatch.setattr(owner_service, "_post_owner", eos_result)
    result_request = {
      key: request[key]
      for key in ("schema_version", "core", "run_id", "revision", "event_id")
    }
    result_request["op"] = "result"
    status, result = _post(path, "/v1/result", result_request)
    assert status == 200 and result["status"] == "succeeded" and result["core"] == "ei"
    status, stale_result = _post(
      path, "/v1/result", {**result_request, "revision": "b" * 64}
    )
    assert status == 409 and stale_result["error"] == "run_identity_mismatch"

    def eos_unavailable(_path: str, _route: str, _request: dict) -> tuple[int, dict]:
      raise OSError("socket closed")

    monkeypatch.setattr(owner_service, "_post_owner", eos_unavailable)
    status, failed = _post(path, "/v1/activate", request)
    assert status == 503 and failed["error"] == "eos_outcome_unknown"
    server.worker.poll.return_value = 1
    status, unavailable = _post(path, "/v1/activate", request)
    assert status == 503 and unavailable["error"] == "owner_worker_unavailable"
    server.run_id = None  # Stop closes admission before worker teardown.
    status, stopped = _post(path, "/v1/activate", request)
    assert status == 409 and stopped["error"] == "run_identity_mismatch"
  finally:
    server.worker = None
    server.shutdown()
    server.server_close()
    worker.join()


def test_arm_monitor_passes_fenced_state_and_throttles_eos(
  tmp_path: Path, monkeypatch
) -> None:
  from unittest.mock import Mock

  from ei import owner_service

  path = tmp_path / "owner.sock"
  server = _OwnerServer(str(path), _Handler)
  server.worker = Mock()
  server.worker.poll.return_value = None
  server.run_id = "run_1"
  server.revision = "a" * 64
  server.eos_owner_path = str(tmp_path / "eos.sock")
  server.monitor_joint_names = [
    "base",
    "shoulder",
    "elbow",
    "wrist_roll",
    "wrist_pitch",
    "tool",
  ]
  server.monitor_forward_interval = 0.1
  server.activation_graph = {
    "cores": {
      "ei": {
        "layers": {
          "io": {
            "groups": {
              "input": {
                "nodes": {
                  "arm_status": {
                    "node_type": "ei.io.input.arm_status@v0.2",
                    "inputs": {
                      "observation": {
                        "node": "eos/interface/egress/arm_status",
                        "output": "observation",
                      }
                    },
                  }
                }
              }
            }
          },
          "cognition": {
            "groups": {
              "state": {
                "nodes": {
                  "arm_status": {
                    "node_type": "ei.cognition.state.arm_status@v0.2",
                    "inputs": {
                      "observation": {
                        "node": "ei/io/input/arm_status",
                        "output": "observation",
                      }
                    },
                  }
                }
              }
            }
          },
          "governance": {
            "groups": {
              "monitor": {
                "nodes": {
                  "arm_status": {
                    "node_type": "ei.governance.monitor.arm_status@v0.2",
                    "settings": {"forward_rate_hz": 10},
                    "inputs": {
                      "observation": {
                        "node": "ei/cognition/state/arm_status",
                        "output": "observation",
                      }
                    },
                  }
                }
              }
            }
          },
        }
      }
    }
  }
  assert owner_service._monitor_forward_rate(server.activation_graph) == 10
  calls: list[dict] = []
  observation = {
    "run_id": "run_1",
    "revision": "a" * 64,
    "source_node": "eos/hardware/mock/arm",
    "sequence": 1,
    "sampled_at_monotonic_ns": 100_000_000_000,
    "joint_names": ["base", "shoulder", "elbow", "wrist_roll", "wrist_pitch", "tool"],
    "joint_positions_rad": [0.0] * 6,
    "state": "ready",
    "reason_code": None,
  }
  assert owner_service._valid_monitor_observation(
    {**observation, "source_node": "eos/hardware/mock/arm_observer"},
    server.run_id,
    server.revision,
    server.monitor_joint_names,
  )
  assert not owner_service._valid_monitor_observation(
    {**observation, "source_node": "eos/hardware/mock/other"},
    server.run_id,
    server.revision,
    server.monitor_joint_names,
  )

  def eos_reply(_path: str, route: str, request: dict) -> tuple[int, dict]:
    assert route == "/v1/monitor"
    calls.append(request)
    next_observation = (
      observation
      if len(calls) == 1
      else {
        **observation,
        "sequence": 2,
        "state": "stale",
        "joint_positions_rad": None,
        "reason_code": "sample_stale",
      }
    )
    if len(calls) == 3:
      next_observation = {**next_observation, "sequence": 1}
    return 200, {
      "schema_version": 1,
      "core": "eos",
      "ok": True,
      "run_id": request["run_id"],
      "revision": request["revision"],
      "observation": next_observation,
    }

  monkeypatch.setattr(owner_service, "_post_owner", eos_reply)
  server.start_monitor()
  worker = threading.Thread(target=server.serve_forever, daemon=True)
  worker.start()
  request = {
    "schema_version": 1,
    "core": "ei",
    "op": "monitor",
    "run_id": "run_1",
    "revision": "a" * 64,
  }
  try:
    deadline = time.monotonic() + 1
    while server.monitor_latest is None and time.monotonic() < deadline:
      time.sleep(0.01)
    status, first = _post(path, "/v1/monitor", request)
    assert status == 200 and first["observation"] == observation
    status, cached = _post(path, "/v1/monitor", request)
    assert status == 200 and cached == first and len(calls) == 1
    deadline = time.monotonic() + 1
    while len(calls) < 2 and time.monotonic() < deadline:
      time.sleep(0.01)
    status, stale = _post(path, "/v1/monitor", request)
    assert status == 200 and stale["observation"]["state"] == "stale"
    assert stale["observation"]["joint_positions_rad"] is None
    assert len(calls) == 2
    deadline = time.monotonic() + 1
    while len(calls) < 3 and time.monotonic() < deadline:
      time.sleep(0.01)
    status, regressed = _post(path, "/v1/monitor", request)
    assert status == 503 and regressed["error"] == "eos_monitor_sequence_regressed"
    input_node = server.activation_graph["cores"]["ei"]["layers"]["io"]["groups"][
      "input"
    ]["nodes"]["arm_status"]
    input_node["inputs"]["observation"]["node"] = "eos/interface/egress/other"
    status, broken = _post(path, "/v1/monitor", request)
    assert status == 422 and broken["error"] == "monitor_path_invalid"
    input_node["inputs"]["observation"]["node"] = "eos/interface/egress/arm_status"
    status, mismatch = _post(path, "/v1/monitor", {**request, "revision": "b" * 64})
    assert status == 409 and mismatch["error"] == "run_identity_mismatch"
    server.run_id = None
    status, stopped = _post(path, "/v1/monitor", request)
    assert status == 409 and stopped["error"] == "run_identity_mismatch"
  finally:
    server.stop_monitor()
    server.worker = None
    server.shutdown()
    server.server_close()
    worker.join()


def test_run_is_fenced_and_stop_reaps_worker(tmp_path: Path, monkeypatch) -> None:
  program_dir = tmp_path / "bin"
  program_dir.mkdir()
  program = program_dir / "ei"
  program.write_text("#!/bin/sh\nexec sleep 30\n")
  program.chmod(0o700)
  monkeypatch.setenv("ISOS_EI_BIN_DIR", str(program_dir))
  path = tmp_path / "owner.sock"
  server = _OwnerServer(str(path), _Handler)
  worker = threading.Thread(target=server.serve_forever, daemon=True)
  worker.start()
  request = {
    "schema_version": 1,
    "core": "ei",
    "op": "run",
    "run_id": "run_1",
    "revision": "a" * 64,
    "graph": {"version": "0.2.0", "robot": {}, "cores": {"ei": {"layers": {}}}},
    "endpoints": {
      "eos_gateway": {"transport": "tcp", "host": "127.0.0.1", "port": 16200}
    },
    "packages": [],
    "launch": {
      "sha256": hashlib.sha256(program.read_bytes()).hexdigest(),
      "args": [],
      "env": {},
    },
  }
  try:
    monitor_graph = {
      "version": "0.2.0",
      "robot": {},
      "cores": {
        "ei": {
          "layers": {
            "governance": {
              "groups": {
                "monitor": {
                  "nodes": {
                    "arm_status": {
                      "node_type": "ei.governance.monitor.arm_status@v0.2",
                      "settings": {},
                      "inputs": {},
                    }
                  }
                }
              }
            },
          }
        }
      },
    }
    status, missing_names = _post(path, "/v1/run", {**request, "graph": monitor_graph})
    assert status == 400 and missing_names["error"] == "invalid_arm_monitor_names"
    status, started = _post(path, "/v1/run", request)
    assert status == 200
    assert started["run_id"] == "run_1"
    status, mismatch = _post(
      path,
      "/v1/stop",
      {
        "schema_version": 1,
        "core": "ei",
        "op": "stop",
        "run_id": "other",
        "revision": "a" * 64,
      },
    )
    assert status == 409
    assert mismatch["error"] == "run_identity_mismatch"
    status, stopped = _post(
      path,
      "/v1/stop",
      {
        "schema_version": 1,
        "core": "ei",
        "op": "stop",
        "run_id": "run_1",
        "revision": "a" * 64,
      },
    )
    assert status == 200
    assert stopped["terminal"]["outcome"] == "stopped"
    assert stopped["terminal"]["worker_exited"] is True
  finally:
    server.shutdown()
    server.server_close()
    worker.join()


def test_real_ei_worker_runs_health_only_without_configuration(
  tmp_path: Path, monkeypatch
) -> None:
  import time

  program_dir = Path(__file__).resolve().parents[2] / "bin"
  program = program_dir / "ei"
  monkeypatch.setenv("ISOS_EI_BIN_DIR", str(program_dir))
  monkeypatch.setenv("EI_LOG_DIR", str(tmp_path / "ei-log"))
  monkeypatch.setenv(
    "EI_PYTHON", str(Path(__file__).resolve().parents[2] / ".venv/bin/python")
  )
  path = tmp_path / "owner.sock"
  server = _OwnerServer(str(path), _Handler)
  thread = threading.Thread(target=server.serve_forever, daemon=True)
  thread.start()
  identity = {
    "schema_version": 1,
    "core": "ei",
    "run_id": "health_1",
    "revision": "a" * 64,
  }
  request = {
    **identity,
    "op": "run",
    "graph": {"version": "0.2.0", "robot": {}, "cores": {"ei": {"layers": {}}}},
    "endpoints": {},
    "packages": [],
    "launch": {
      "sha256": hashlib.sha256(program.read_bytes()).hexdigest(),
      "args": ["--state-root", str(tmp_path / "state")],
      "env": {"ISOS_RUN_ID": "health_1"},
    },
  }
  try:
    status, started = _post(path, "/v1/run", request)
    assert status == 200, started
    for _ in range(40):
      time.sleep(0.05)
      status, health = _exchange(
        path, b"GET /v1/health HTTP/1.1\r\nHost: owner\r\n\r\n"
      )
      if health["state"] != "running":
        break
    assert status == 200
    assert health["state"] == "running"
    status, stopped = _post(path, "/v1/stop", {**identity, "op": "stop"})
    assert status == 200, stopped
    assert stopped["terminal"]["worker_exited"] is True
    assert server.worker is None
    status, stale = _post(path, "/v1/status", {**identity, "op": "status"})
    assert status == 409
    assert stale["error"] == "run_identity_mismatch"
    second = {
      **request,
      "run_id": "health_2",
      "launch": {**request["launch"], "env": {"ISOS_RUN_ID": "health_2"}},
    }
    status, started_again = _post(path, "/v1/run", second)
    assert status == 200, started_again
    status, stopped_again = _post(
      path, "/v1/stop", {**identity, "run_id": "health_2", "op": "stop"}
    )
    assert status == 200, stopped_again
    assert stopped_again["terminal"]["worker_exited"] is True
    records = [
      json.loads(line)
      for line in (tmp_path / "ei-log/ei.jsonl").read_text().splitlines()
    ]
    assert [record["message"] for record in records] == [
      "EI health worker started",
      "EI health worker stopped",
      "EI health worker started",
      "EI health worker stopped",
    ]
    assert all(
      {"timestamp", "level", "source", "message"} <= record.keys()
      and record["source"] == "ei.owner_service.py@aether-v0.2.1"
      for record in records
    )
  finally:
    server.shutdown()
    server.server_close()
    thread.join()


def test_two_owner_health_workers_share_state_root_without_lock_collision(
  tmp_path: Path, monkeypatch
) -> None:
  import time

  program_dir = Path(__file__).resolve().parents[2] / "bin"
  program = program_dir / "ei"
  monkeypatch.setenv("ISOS_EI_BIN_DIR", str(program_dir))
  monkeypatch.setenv(
    "EI_PYTHON", str(Path(__file__).resolve().parents[2] / ".venv/bin/python")
  )
  monkeypatch.setenv("ISLAND_STATE", str(tmp_path / "shared-state"))
  monkeypatch.setenv("EI_LOG_DIR", str(tmp_path / "logs"))
  servers = [
    _OwnerServer(str(tmp_path / f"owner-{index}.sock"), _Handler) for index in (1, 2)
  ]
  threads = [
    threading.Thread(target=server.serve_forever, daemon=True) for server in servers
  ]
  for thread in threads:
    thread.start()
  requests = [
    {
      "schema_version": 1,
      "core": "ei",
      "op": "run",
      "run_id": f"parallel_{index}",
      "revision": "a" * 64,
      "graph": {"version": "0.2.0", "robot": {}, "cores": {"ei": {"layers": {}}}},
      "endpoints": {},
      "packages": [],
      "launch": {
        "sha256": hashlib.sha256(program.read_bytes()).hexdigest(),
        "args": [],
        "env": {},
      },
    }
    for index in (1, 2)
  ]
  try:
    for server, request in zip(servers, requests, strict=True):
      status, started = _post(Path(server.server_address), "/v1/run", request)
      assert status == 200, started
    time.sleep(0.2)
    for server, request in zip(servers, requests, strict=True):
      status, reply = _post(
        Path(server.server_address),
        "/v1/status",
        {
          "schema_version": 1,
          "core": "ei",
          "op": "status",
          "run_id": request["run_id"],
          "revision": request["revision"],
        },
      )
      assert status == 200 and reply["state"] == "running", reply
  finally:
    for server, request in zip(servers, requests, strict=True):
      _post(
        Path(server.server_address),
        "/v1/stop",
        {
          "schema_version": 1,
          "core": "ei",
          "op": "stop",
          "run_id": request["run_id"],
          "revision": request["revision"],
        },
      )
      server.shutdown()
      server.server_close()
    for thread in threads:
      thread.join()
