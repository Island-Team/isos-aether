"""Real v0.2 IsOS→EOS supervisor replay with the published MuJoCo package.

Usage: python3 crates/isos/tests/unified_mujoco_supervisor_replay.py OUT RELEASE BUNDLE MUJOCO_ROOT [pick-place|navigation]
"""

import base64
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from urllib.error import HTTPError
from urllib.request import Request, urlopen

from generated_config_preflight_cli import ISOS, CleanHome, digest
from real_mujoco_first_use import ASSET_SHA256, MANIFEST_SHA256
from real_mujoco_first_use import digest as file_digest
from unified_owner_replay_cli import BIN as DEFAULT_BIN, EI as DEFAULT_EI, unified_graph

BIN = Path(os.environ.get("ISOS_AETHER_BIN_DIR", DEFAULT_BIN))
EI = Path(os.environ.get("ISOS_EI_ROOT", DEFAULT_EI))


def children(pid):
    path = Path(f"/proc/{pid}/task/{pid}/children")
    return [int(value) for value in path.read_text().split()] if path.exists() else []


def bridge_request(route, payload=None):
    request = Request(f"http://127.0.0.1:16020/api/eos/v2/{route}",
                      data=None if payload is None else json.dumps(payload).encode(),
                      headers={"Content-Type": "application/json"})
    try:
        response = urlopen(request, timeout=15)
    except HTTPError as error:
        response = error
    with response:
        return response.status, json.loads(response.read())


def main():
    out, manifest, bundle, runtime = (Path(value).resolve() for value in sys.argv[1:5])
    scene = sys.argv[5] if len(sys.argv) > 5 else "pick-place"
    assert scene in ("pick-place", "navigation"), scene
    assert not out.exists(), "use a fresh evidence directory"
    out.mkdir(parents=True)
    assert file_digest(manifest) == MANIFEST_SHA256
    assert file_digest(bundle.parent / "mujoco-sim-v0.1.1-linux-x86_64.tar.gz") == ASSET_SHA256
    assert (runtime / "lib/libmujoco.so.3.16.0").is_file()
    short_root = Path(tempfile.mkdtemp(prefix="isos-v2-", dir="/tmp"))
    (short_root / "xdg").mkdir()
    case = CleanHome(str(short_root))
    case.env.update({
        "XDG_CONFIG_HOME": str(short_root / "xdg"),
        "ISOS_AETHER_BIN_DIR": str(BIN),
        "ISOSUI_DIST_DIR": str(Path(os.environ.get("ISOSUI_DIST_DIR", DEFAULT_BIN.parents[1] / "ui/dist/local"))),
        "ISOS_EI_BIN_DIR": str(EI / "bin"),
        "ISOS_EI_DEFAULTS_DIR": str(EI.parent / "config/ei"),
        "ISOS_EI_FACTORY_DEFAULTS_DIR": str(EI.parent / "share/ei/defaults/ei"),
        "EI_PYTHON": str(EI / ".venv/bin/python"),
        "ISLAND_PREFIX": str(EI),
        "MUJOCO_ROOT": str(runtime),
        "LD_LIBRARY_PATH": str(runtime / "lib"),
        "MUJOCO_GL": "egl",
    })
    try:
        case.init()
        case.command("pkg", "link", "island/mujoco-sim", "0.1.1", bundle, "--manifest", manifest)
        case.command("pkg", "verify", manifest,
                     bundle.parent / "mujoco-sim-v0.1.1-linux-x86_64.tar.gz")
        template = Path(__file__).parent / "fixtures/unified_mujoco/current_draft.json"
        graph = json.loads(template.read_text().replace("{ISOS_HOME}", str(case.home)))
        graph = unified_graph(graph)
        graph["cores"]["isos"]["layers"]["eos_process"]["groups"]["mujoco"]["nodes"]["simulator"]["settings"]["scene"] = scene
        candidate = out / "candidate.nodes.json"
        candidate.write_text(json.dumps(graph, indent=2, sort_keys=True) + "\n")
        case.command("config", "validate", candidate)
        saved_revision = digest(case.graph_path.read_bytes())
        case.command("config", "apply", candidate, "--revision", saved_revision)
        revision = digest(case.graph_path.read_bytes())
        applied = json.loads(case.graph_path.read_text())
        assert applied["cores"]["isos"]["layers"]["eos_process"]["groups"]["mujoco"]["nodes"]["simulator"]["settings"]["scene"] == scene
        case.output = (out / "serve.log").open("w+")
        case.server = subprocess.Popen([str(ISOS), "serve"], env=case.env,
                                       stdout=case.output, stderr=subprocess.STDOUT, text=True)
        for _ in range(200):
            case.output.flush()
            log = (out / "serve.log").read_text()
            match = re.search(r"IsOS API: http://127\.0\.0\.1:(\d+)", log)
            ui = re.search(r"http://127\.0\.0\.1:(\d+)/en/isos", log)
            if match and ui:
                case.port = int(match.group(1))
                ui_origin = f"http://127.0.0.1:{ui.group(1)}"
                def api(route, data=None, method=None, timeout=30, origin=ui_origin):
                    payload = None if data is None else json.dumps(data).encode()
                    request = Request(f"http://127.0.0.1:{case.port}/api/isos/v1/{route}",
                                      data=payload, method=method or ("POST" if payload else "GET"),
                                      headers={"Origin": origin,
                                               "Content-Type": "application/json"})
                    try:
                        response = urlopen(request, timeout=timeout)
                    except HTTPError as error:
                        response = error
                    with response:
                        body = response.read()
                        try:
                            return response.status, json.loads(body)
                        except ValueError as error:
                            raise AssertionError((response.status, body[:300])) from error
                case.api = api
                case.control = lambda operation, timeout=30: case.api(
                    "runtime/control", {"schema_version": "0.1.0", "operation": operation,
                                        "revision": digest(case.graph_path.read_bytes())}, timeout=timeout)
                try:
                    case.api("graph")
                    break
                except OSError:
                    pass
            if case.server.poll() is not None:
                raise AssertionError("IsOS server exited before readiness")
            time.sleep(.05)
        else:
            raise AssertionError("IsOS server did not become ready")
        status, reply = case.control("run", timeout=90)
        (out / "run.json").write_text(json.dumps(reply, indent=2))
        assert status == 200, reply
        status, state = case.api("runtime/process-status")
        assert status == 200, state
        (out / "running.json").write_text(json.dumps(state, indent=2))
        status, readiness = case.api("cores/readiness")
        (out / "readiness.json").write_text(json.dumps(readiness, indent=2))
        assert status == 200, readiness
        ready_owners = {entry["core"]: entry for entry in readiness["data"]["cores"]}
        assert all(ready_owners[owner]["ready"] for owner in ("eos", "ei")), ready_owners
        processes = {entry["id"]: entry for entry in state["data"]["processes"]}
        assert "eos" in processes, processes
        assert state["data"]["state"] == "running", state
        assert {"eos", "bridge", "config", "ei"} <= set(processes), processes
        assert state["data"]["log_ingest_rejected"] == 0, state
        eos_pid = processes["eos"]["pid"]
        simulator_children = children(eos_pid)
        assert simulator_children, "EOS did not directly supervise MuJoCo"
        status, gates = bridge_request("vision/gates")
        (out / "camera-gates.json").write_text(json.dumps(gates, indent=2))
        assert status == 200 and gates["gates"], gates
        camera = "isos/eos_hardware/camera/front"
        status, captured = bridge_request("control", {"op": "capture", "node_path": camera})
        assert status == 200 and captured["ok"] and captured["frame"]["rgb8_base64"], captured
        frame = captured["frame"]
        pixels = base64.b64decode(frame["rgb8_base64"], validate=True)
        (out / "camera-capture.json").write_text(json.dumps({
            "node_path": camera, "width_px": frame["width_px"],
            "height_px": frame["height_px"], "rgb8_sha256": digest(pixels),
            "rgb8_bytes": len(pixels)}, indent=2))
        status, clock = bridge_request("control", {"op": "time"})
        assert status == 200 and clock["ok"], clock
        payload = {"type": "body_velocity", "x_m_s": 0.1, "y_m_s": 0,
                   "yaw_rad_s": 0, "valid_for_ms": 100}
        control = {"op": "primitive_typed", "header": {
            "data_type_id": "base_movement", "data_type_version": "1.0",
            "primitive_name": "base.velocity", "primitive_version": "1.0.0",
            "source_path": "isos/eos_dispatcher/base/base-velocity",
            "target_path": "isos/eos_hardware/base/drive", "active_revision": revision,
            "command_id": "supervisor-first-use", "sequence": 1,
            "deadline_monotonic_ns": clock["monotonic_ns"] + 2_000_000_000,
            "payload_length_bytes": len(json.dumps(payload, separators=(",", ":")).encode()),
            "codec": "json_v1"}, "payload": payload}
        status, admitted = bridge_request("control", control)
        (out / "admitted-control.json").write_text(json.dumps(admitted, indent=2))
        assert status == 200 and admitted["ok"], admitted
        deadline = time.monotonic() + 5
        while True:
            status, result = bridge_request("control", {
                "op": "result", "command_id": "supervisor-first-use",
                "active_revision": revision})
            assert status == 200 and result["ok"], result
            if result["result"]["status"] != "InProgress":
                break
            assert time.monotonic() < deadline, result
            time.sleep(.05)
        (out / "completed-control.json").write_text(json.dumps(result, indent=2))
        assert result["result"]["status"] == "Completed", result
        status, logs = case.api("runtime/logs?source=all&limit=200")
        (out / "control-logs.json").write_text(json.dumps(logs, indent=2))
        assert status == 200, logs
        run_id = reply["data"]["run_id"]
        lines = logs["data"]["lines"]
        assert any(line["run_id"] == run_id and line["event_code"] == "deployment_ready"
                   for line in lines), lines
        result_events = [(line["fields"]["phase"], line["fields"]["status"])
                         for line in lines if line["run_id"] == run_id and
                         line["event_code"] == "bridge_primitive_result" and
                         line["correlation_id"] == "supervisor-first-use"]
        assert ("submit", "InProgress") in result_events, result_events
        assert ("followup", "Completed") in result_events, result_events
        (out / "first-simulator-pid.txt").write_text(str(simulator_children[0]) + "\n")
        status, restarted = case.control("restart", timeout=90)
        (out / "restart.json").write_text(json.dumps(restarted, indent=2))
        assert status == 200, restarted
        assert not Path(f"/proc/{simulator_children[0]}").exists(), "old simulator survived Restart"
        status, state = case.api("runtime/process-status")
        assert status == 200, state
        eos_pid = {entry["id"]: entry for entry in state["data"]["processes"]}["eos"]["pid"]
        simulator_children = children(eos_pid)
        assert simulator_children, "restarted EOS did not supervise MuJoCo"
        status, stopped = case.control("stop", timeout=90)
        (out / "stop.json").write_text(json.dumps(stopped, indent=2))
        assert status == 200, stopped
        assert stopped["data"]["terminal"]["outcome"] == "confirmed", stopped
        assert not Path(f"/proc/{simulator_children[0]}").exists(), "simulator child survived Stop"
        status, stop_logs = case.api("runtime/logs?source=all&limit=200")
        (out / "stop-logs.json").write_text(json.dumps(stop_logs, indent=2))
        assert status == 200, stop_logs
        stop_run = stopped["data"]["terminal"]["run_id"]
        stop_events = [line["event_code"] for line in stop_logs["data"]["lines"]
                       if line["run_id"] == stop_run]
        assert "eos_stopped" in stop_events and "deployment_stopped" in stop_events, stop_events
        status, reply = case.control("run", timeout=90)
        (out / "failure-run.json").write_text(json.dumps(reply, indent=2))
        assert status == 200, reply
        status, state = case.api("runtime/process-status")
        assert status == 200, state
        eos_pid = {entry["id"]: entry for entry in state["data"]["processes"]}["eos"]["pid"]
        simulator_children = children(eos_pid)
        assert simulator_children, "failure run did not supervise MuJoCo"
        (out / "failed-simulator-pid.txt").write_text(str(simulator_children[0]) + "\n")
        os.kill(simulator_children[0], signal.SIGKILL)
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline and Path(f"/proc/{simulator_children[0]}").exists():
            time.sleep(.05)
        status, failed = case.api("runtime/process-status")
        (out / "after-child-failure.json").write_text(json.dumps(failed, indent=2))
        assert status == 200
        assert failed["data"]["state"] == "degraded", failed
        assert failed["data"]["terminal"]["outcome"] == "unknown", failed
        status, rejected = case.control("restart", timeout=90)
        (out / "rejected-restart.json").write_text(json.dumps(rejected, indent=2))
        assert status != 200 and rejected["error"]["code"] == "deployment_stop_unconfirmed", rejected
        assert not Path(f"/proc/{simulator_children[0]}").exists(), "failed child remains"
        assert digest(case.graph_path.read_bytes()) == revision
        print(f"v0.2 {scene} real owner Apply → Run → Restart → Stop; failure rejects unsafe Restart")
    finally:
        case.close()
        if case.home.exists():
            shutil.copytree(case.home, out / "home", dirs_exist_ok=True, symlinks=True)
        shutil.rmtree(short_root)


if __name__ == "__main__":
    main()
