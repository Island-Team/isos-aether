"""P04 clean-home generated configuration and launch-preflight replay.

Build the Rust workspace first. Set ISOS_BIN and ISOS_AETHER_BIN_DIR when using
another build output directory.
"""

import hashlib
import json
import os
import socket
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path
from urllib.error import HTTPError
from urllib.request import Request, urlopen

ROOT = Path(__file__).resolve().parents[1]
WORKSPACE = ROOT.parents[1]
ISOS = Path(os.environ.get("ISOS_BIN", WORKSPACE / "target/debug/isos"))
EOS_BIN_DIR = Path(os.environ.get(
    "ISOS_AETHER_BIN_DIR", WORKSPACE / "target/debug"
))
CASES = json.loads((ROOT / "tests/fixtures/generated_config_preflight/cases.json").read_text())


def free_port():
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        return listener.getsockname()[1]


def digest(data):
    return hashlib.sha256(data).hexdigest()


class CleanHome:
    def __init__(self, root):
        self.root = Path(root)
        self.home = self.root / "home"
        self.graph_path = self.home / "config/deployments/default.nodes.json"
        self.generated = self.home / "data/isos/state/generated/default"
        self.port = free_port()
        self.ui_port = free_port()
        self.server = None
        self.output = None
        self.env = dict(os.environ, ISOS_HOME=str(self.home),
                        XDG_CONFIG_HOME=str(self.root / "xdg-config"),
                        ISOS_AETHER_BIN_DIR=str(EOS_BIN_DIR),
                        ISOS_HTTP_PORT=str(self.port), ISOSUI_PORT=str(self.ui_port))
        self.env.pop("P04_MISSING_SECRET", None)

    def command(self, *args, success=True):
        result = subprocess.run([str(ISOS), *map(str, args)], env=self.env,
                                text=True, capture_output=True, timeout=45, check=False)
        print("$ isos", *args, "=>", result.returncode, flush=True)
        if result.stdout:
            print(result.stdout.rstrip(), flush=True)
        if result.stderr:
            print(result.stderr.rstrip(), flush=True)
        if success and result.returncode:
            raise AssertionError(f"isos {' '.join(map(str, args))} failed")
        return result

    def init(self):
        self.command("init", "--mode", "dev")
        graph = json.loads(self.graph_path.read_text())
        assert graph["version"] == "0.2.0", graph["version"]
        return graph

    def apply(self, graph=None):
        if graph is None:
            graph = json.loads(self.graph_path.read_text())
        candidate = self.root / "candidate.nodes.json"
        candidate.write_text(json.dumps(graph, indent=2))
        saved_revision = digest(self.graph_path.read_bytes())
        self.command("config", "validate", candidate)
        self.command("config", "apply", candidate, "--revision", saved_revision)
        return digest(self.graph_path.read_bytes())

    def selected(self):
        revision = digest(self.graph_path.read_bytes())
        selection = json.loads((self.generated / "selection.json").read_text())
        assert selection["schema_version"] == 2
        entry = selection["entries"][revision]
        assert entry["kind"] == "v2"
        generation = entry["generation"]
        leaf = self.generated / "v2" / revision / generation
        return revision, generation, leaf

    def seed_ei_distribution(self):
        prefix = self.root / "ei-install"
        binary_dir = prefix / "bin"
        component = prefix / "etc/ei"
        factory = prefix / "share/defaults/ei"
        binary_dir.mkdir(parents=True)
        component.mkdir(parents=True)
        factory.mkdir(parents=True)
        (prefix / "pyproject.toml").write_text("[project]\nname='ei-fixture'\nversion='0.2.1'\n")
        module = prefix / "src/ei"
        module.mkdir(parents=True)
        (module / "__init__.py").write_text("# verified fixture module\n")
        for relative in (
            "ei.toml", "modes.toml", "profiles/isos.toml", "models.toml",
            "prompts.toml", "missions.toml", "safety/gate.toml", "opt/sandbox.toml",
            "extra/nested.toml",
        ):
            path = component / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("enabled = true\n")
        (factory / "models.toml").write_text("enabled = true\n")
        adapter = binary_dir / "ei-config"
        adapter.write_text("""#!/usr/bin/env python3
import json
import os
import sys
identity = {
        "core": "ei", "graph_version": "0.2.0", "node_type_version": "0.2",
        "product_version": "0.2.1", "setup_schema_version": 2,
        "supported_graph_versions": ["0.2.0"],
        "supported_node_type_versions": ["0.2"],
        "supported_node_types": ["isos.ei_cognition.builder@v0.2", "isos.ei_cognition.planner@v0.2",
                                 "isos.ei_cognition.queue@v0.2", "isos.ei_evidence.admission@v0.2",
                                 "isos.ei_governance.gate@v0.2", "isos.ei_governance.health@v0.2",
                                 "isos.ei_io.gateway@v0.2"],
        "features": ["setup-v2", "catalog", "graph-0.2-ei-projection", "ei-evidence-routing-v1"],
    }
if sys.argv[1:] == ["--capabilities"]:
    print(json.dumps({"binary": "ei-config", **{k: v for k, v in identity.items() if k != "core"}}))
    sys.exit(0)
request = json.load(sys.stdin)
reply = {"schema_version": 2, "ok": True, "diagnostics": [], "files": []}
if request["op"] == "identity":
    reply["identity"] = identity
elif request["op"] == "render":
    reply["files"] = [
        {"path": "ei/ei.toml", "content": "mode = 'isos'\\n"},
        {"path": "ei/modes.toml", "content": "enabled = true\\n"},
        {"path": "ei/profiles/isos.toml", "content": "managed = true\\n"},
    ]
    if os.getenv("P04_REJECT_RENDER") == "1":
        reply["ok"] = False
        reply["diagnostics"] = [{"path": "/cores/isos/layers/ei_io", "code": "stub_rejected",
                                 "message": "rejected"}]
print(json.dumps(reply))
""")
        adapter.chmod(0o700)
        executable = binary_dir / "ei"
        executable.write_text("#!/bin/sh\nif [ \"$1\" = --capabilities ]; then printf '%s\\n' '{\"binary\":\"ei\",\"product_version\":\"0.2.1\",\"features\":[\"managed-config-v1\",\"eos-gateway-client-v2\"]}'; exit 0; fi\nexit 0\n")
        executable.chmod(0o700)
        self.env["ISOS_EI_BIN_DIR"] = str(binary_dir)
        self.env["EI_PYTHON"] = str(Path(sys.executable).resolve())
        self.env["ISOS_EI_DEFAULTS_DIR"] = str(component)
        self.env["ISOS_EI_FACTORY_DEFAULTS_DIR"] = str(factory)
        self.env.pop("ISLAND_PREFIX", None)
        return component, factory

    def start_server(self):
        self.output = (self.root / "serve.log").open("w+")
        self.server = subprocess.Popen([str(ISOS), "serve"], env=self.env,
                                       stdout=self.output, stderr=subprocess.STDOUT, text=True)
        for _ in range(160):
            if self.server.poll() is not None:
                raise AssertionError(f"IsOS serve exited {self.server.returncode}")
            try:
                self.api("graph")
                return
            except (OSError, ValueError):
                time.sleep(0.05)
        raise AssertionError("IsOS API did not become ready: " + (self.root / "serve.log").read_text()[-3000:])

    def api(self, route, data=None, method=None, timeout=30):
        payload = None if data is None else json.dumps(data).encode()
        request = Request(f"http://127.0.0.1:{self.port}/api/isos/v1/{route}",
                          data=payload, method=method or ("POST" if data is not None else "GET"),
                          headers={"Origin": f"http://127.0.0.1:{self.port}",
                                   "Content-Type": "application/json"})
        try:
            with urlopen(request, timeout=timeout) as response:
                return response.status, json.load(response)
        except HTTPError as error:
            return error.code, json.load(error)

    def control(self, operation, timeout=30):
        return self.api("runtime/control", {
            "schema_version": "0.1.0", "operation": operation,
            "revision": digest(self.graph_path.read_bytes()),
        }, timeout=timeout)

    def http_apply(self, graph):
        return self.api("graph", {
            "schema_version": "0.1.0",
            "revision": digest(self.graph_path.read_bytes()),
            "graph": graph,
        }, method="PUT")

    def close(self):
        if self.server is not None:
            self.server.terminate()
            try:
                self.server.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.server.kill()
                self.server.wait(timeout=5)
        if self.output is not None:
            self.output.close()


class GeneratedConfigReplay(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        for binary in (ISOS, EOS_BIN_DIR / "aether-profile", EOS_BIN_DIR / "aether-runtime"):
            if not binary.is_file():
                raise AssertionError(f"missing test binary: {binary}")
        assert CASES["slice"] == "P04"

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="isos-p04-")
        self.case = CleanHome(self.temp.name)
        self.case.init()

    def tearDown(self):
        self.case.close()
        self.temp.cleanup()

    def assert_failure(self, expected_case):
        expected = next(case for case in CASES["cases"] if case["id"] == expected_case)
        status, reply = self.case.control("run")
        self.assertEqual(status, 422, reply)
        self.assertFalse(reply["ok"])
        self.assertIsInstance(reply["error"]["details"], dict, reply)
        first = reply["error"]["details"]["diagnostics"][0]
        self.assertEqual(
            {key: first[key] for key in ("code", "path", "stage")},
            {"code": expected["expected_code"], "path": expected["expected_path"],
             "stage": expected["expected_stage"]},
        )
        self.assertEqual(set(first), {"code", "path", "message", "stage"})

    def assert_apply_failure(self, candidate, revision, expected_case):
        expected = next(case for case in CASES["cases"] if case["id"] == expected_case)
        graph_before = self.case.graph_path.read_bytes()
        selection_before = (self.case.generated / "selection.json").read_bytes()
        result = self.case.command("config", "apply", candidate,
                                   "--revision", revision, success=False)
        self.assertNotEqual(result.returncode, 0)
        output = result.stdout + result.stderr
        self.assertIn('"code":"' + expected["expected_code"] + '"', output)
        self.assertIn('"path":"' + expected["expected_path"] + '"', output)
        self.assertIn('"stage":"' + expected["expected_stage"] + '"', output)
        self.assertEqual(self.case.graph_path.read_bytes(), graph_before)
        self.assertEqual((self.case.generated / "selection.json").read_bytes(),
                         selection_before)

    def test_immutable_selection_and_repeat_apply(self):
        graph_before = self.case.graph_path.read_bytes()
        self.case.apply()
        revision, generation, leaf = self.case.selected()
        self.assertEqual(revision, digest(self.case.graph_path.read_bytes()))
        marker = self.case.generated / "v2/required" / f"{revision}.json"
        self.assertEqual(json.loads(marker.read_text()),
                         {"schema_version": 1, "revision": revision})
        manifest = json.loads((leaf / "manifest.json").read_text())
        self.assertEqual(manifest["schema_version"], 2)
        self.assertEqual((manifest["revision"], manifest["generation"]),
                         (revision, generation))
        inventory = {key: value for key, value in manifest.items() if key != "generation"}
        canonical = json.dumps(inventory, sort_keys=True, separators=(",", ":"),
                               ensure_ascii=False).encode()
        self.assertEqual(digest(canonical), generation)
        self.assertEqual([item["path"] for item in manifest["files"]],
                         sorted(item["path"] for item in manifest["files"]))
        listed = {item["path"] for item in manifest["files"]}
        present = {str(path.relative_to(leaf)) for path in leaf.rglob("*") if path.is_file()}
        self.assertEqual(present, listed | {"manifest.json"})
        for item in manifest["files"]:
            path = leaf / item["path"]
            self.assertEqual(digest(path.read_bytes()), item["sha256"])
            self.assertEqual(path.stat().st_mode & 0o777, 0o600)
        self.assertEqual(leaf.stat().st_mode & 0o777, 0o700)
        self.assertEqual(marker.stat().st_mode & 0o777, 0o600)
        self.assertEqual((leaf / "manifest.json").stat().st_mode & 0o777, 0o600)
        manifest_before = (leaf / "manifest.json").read_bytes()
        self.case.apply()
        self.assertEqual(self.case.selected()[:2], (revision, generation))
        self.assertEqual((leaf / "manifest.json").read_bytes(), manifest_before)
        self.assertEqual(self.case.graph_path.read_bytes(), graph_before)

    def test_deleted_selection_rejects_launch(self):
        self.case.apply()
        before = self.case.graph_path.read_bytes()
        (self.case.generated / "selection.json").unlink()
        self.case.start_server()
        self.assert_failure("deleted_selection_after_v2_apply")
        self.assertEqual(self.case.graph_path.read_bytes(), before)

    def test_changed_graph_selection_loss_keeps_marked_revision_inert(self):
        self.case.apply()
        old_revision, old_generation, old_leaf = self.case.selected()
        graph = json.loads(self.case.graph_path.read_text())
        graph["cores"]["isos"]["layers"]["process"]["groups"]["component"]["nodes"]["eos"][
            "description"] = "Changed EOS process"
        self.case.apply(graph)
        new_revision, new_generation, new_leaf = self.case.selected()
        self.assertNotEqual(new_revision, old_revision)
        self.assertTrue((old_leaf / "manifest.json").is_file())
        self.assertTrue((new_leaf / "manifest.json").is_file())
        selection = json.loads((self.case.generated / "selection.json").read_text())
        self.assertEqual(selection["entries"][old_revision]["generation"], old_generation)
        self.assertEqual(selection["entries"][new_revision]["generation"], new_generation)
        (self.case.generated / "selection.json").unlink()
        before = self.case.graph_path.read_bytes()
        self.case.start_server()
        self.assert_failure("deleted_selection_after_changed_graph_apply")
        self.assertEqual(self.case.graph_path.read_bytes(), before)

    def test_changed_defaults_and_binary_require_explicit_generation_selection(self):
        component, _ = self.case.seed_ei_distribution()
        graph = json.loads(self.case.graph_path.read_text())
        nodes = graph["cores"]["isos"]["layers"]["process"]["groups"]["component"]["nodes"]
        nodes["ei"] = {
            "id": "ei", "node_type": "isos.process.component@v0.2",
            "label": "EI agent", "description": "Managed agent for generation selection",
            "capabilities": ["process"],
            "settings": {"role": "ei", "args": [],
                         "depends_on": ["cores/isos/layers/process/groups/component/nodes/eos"]},
        }
        self.case.apply(graph)
        revision, first_generation, first_leaf = self.case.selected()
        first_manifest = (first_leaf / "manifest.json").read_bytes()
        graph_bytes = self.case.graph_path.read_bytes()
        self.case.start_server()

        models = component / "models.toml"
        models.write_text("enabled = false\n")
        status, reply = self.case.http_apply(json.loads(graph_bytes))
        self.assertEqual(status, 200, reply)
        default_revision, default_generation, default_leaf = self.case.selected()
        self.assertEqual(default_revision, revision)
        self.assertNotEqual(default_generation, first_generation)
        self.assertEqual((first_leaf / "manifest.json").read_bytes(), first_manifest)
        default_manifest = (default_leaf / "manifest.json").read_bytes()

        executable = self.case.root / "ei-install/bin/ei"
        executable.write_text("#!/bin/sh\n# changed verified executable\nif [ \"$1\" = --capabilities ]; then printf '%s\\n' '{\"binary\":\"ei\",\"product_version\":\"0.2.1\",\"features\":[\"managed-config-v1\",\"eos-gateway-client-v2\"]}'; exit 0; fi\nexit 0\n")
        status, reply = self.case.http_apply(json.loads(graph_bytes))
        self.assertEqual(status, 200, reply)
        binary_revision, binary_generation, binary_leaf = self.case.selected()
        self.assertEqual(binary_revision, revision)
        self.assertNotIn(binary_generation, (first_generation, default_generation))
        self.assertEqual((default_leaf / "manifest.json").read_bytes(), default_manifest)
        self.assertTrue((binary_leaf / "manifest.json").is_file())
        self.assertEqual(self.case.graph_path.read_bytes(), graph_bytes)

    def assert_uncertain_publication_boundary(self, boundary):
        if boundary == "graph":
            self.case.apply()
        before = self.case.graph_path.read_bytes()
        old_revision = digest(before)
        graph = json.loads(before)
        if boundary == "graph":
            graph["cores"]["isos"]["layers"]["process"]["groups"]["component"]["nodes"]["eos"][
                "description"] = "Changed EOS process for publish boundary"
        self.case.env["ISOS_P04_TEST_FAIL_SYNC"] = boundary
        self.case.env["ISOS_P04_TEST_FAIL_SYNC_DEPLOYMENT"] = "default"
        self.case.start_server()
        status, reply = self.case.http_apply(graph)
        self.assertEqual(status, 500, reply)
        first = reply["error"]["details"]["diagnostics"][0]
        self.assertEqual((first["code"], first["path"], first["stage"]),
                         ("generated_commit_uncertain", "", "publish"))
        observed_revision = digest(self.case.graph_path.read_bytes())
        if boundary == "graph":
            self.assertTrue(observed_revision == old_revision
                            or json.loads(self.case.graph_path.read_bytes()) == graph)
        else:
            self.assertEqual(observed_revision, old_revision)
        self.case.close()
        self.case.server = None
        self.case.output = None
        self.case.env.pop("ISOS_P04_TEST_FAIL_SYNC")
        self.case.env.pop("ISOS_P04_TEST_FAIL_SYNC_DEPLOYMENT")
        self.case.start_server()
        revision, _, leaf = self.case.selected()
        self.assertEqual(revision, digest(self.case.graph_path.read_bytes()))
        self.assertTrue((leaf / "manifest.json").is_file())
        status, reply = self.case.control("run")
        self.assertEqual(status, 200, reply)
        self.assertEqual(reply["data"]["active_revision"], revision)
        status, reply = self.case.control("stop")
        self.assertEqual(status, 200, reply)

    def test_selection_parent_sync_uncertain_recovers(self):
        self.assert_uncertain_publication_boundary("selection")

    def test_marker_parent_sync_uncertain_recovers(self):
        self.assert_uncertain_publication_boundary("marker")

    def test_graph_parent_sync_uncertain_recovers(self):
        self.assert_uncertain_publication_boundary("graph")

    def test_owner_render_rejection_after_apply_stays_typed_over_http(self):
        self.case.seed_ei_distribution()
        graph = json.loads(self.case.graph_path.read_text())
        nodes = graph["cores"]["isos"]["layers"]["process"]["groups"]["component"]["nodes"]
        nodes["ei"] = {
            "id": "ei", "node_type": "isos.process.component@v0.2",
            "label": "EI agent", "description": "Managed agent for typed rejection",
            "capabilities": ["process"],
            "settings": {"role": "ei", "args": [],
                         "depends_on": ["cores/isos/layers/process/groups/component/nodes/eos"]},
        }
        self.case.apply(graph)
        graph_before = self.case.graph_path.read_bytes()
        self.case.env["P04_REJECT_RENDER"] = "1"
        self.case.start_server()
        status, reply = self.case.control("run")
        self.assertEqual(status, 422, reply)
        first = reply["error"]["details"]["diagnostics"][0]
        self.assertEqual((first["code"], first["path"], first["stage"]),
                         ("generated_owner_rejected", "/cores/isos/layers/ei_io", "render"))
        self.assertEqual(set(first), {"code", "path", "message", "stage"})
        self.assertEqual(self.case.graph_path.read_bytes(), graph_before)

    def test_generated_file_tamper_rejected_before_run(self):
        self.case.apply()
        before = self.case.graph_path.read_bytes()
        _, _, leaf = self.case.selected()
        with (leaf / "eos/graph.json").open("ab") as file:
            file.write(b"\n")
        self.case.start_server()
        self.assert_failure("manifest_file_tamper")
        self.assertEqual(self.case.graph_path.read_bytes(), before)

    def test_missing_secret_has_exact_pointer_and_no_value_leak(self):
        graph = json.loads(self.case.graph_path.read_text())
        eos = graph["cores"]["isos"]["layers"]["process"]["groups"]["component"]["nodes"]["eos"]
        eos["settings"]["env_from"] = {"P04_TARGET": "P04_MISSING_SECRET"}
        self.case.apply(graph)
        _, _, leaf = self.case.selected()
        self.assertNotIn(b"secret-value", (leaf / "manifest.json").read_bytes())
        self.case.start_server()
        status, reply = self.case.control("run")
        self.assertEqual(status, 422, reply)
        first = reply["error"]["details"]["diagnostics"][0]
        self.assertEqual(
            (first["code"], first["path"], first["stage"]),
            ("generated_missing_secret",
             "/cores/isos/layers/process/groups/component/nodes/eos/settings/env_from/P04_TARGET",
             "spawn"),
        )
        self.assertNotIn("P04_MISSING_SECRET", json.dumps(reply))

    def test_ei_defaults_are_selected_and_missing_file_keeps_generation(self):
        component, factory = self.case.seed_ei_distribution()
        graph = json.loads(self.case.graph_path.read_text())
        nodes = graph["cores"]["isos"]["layers"]["process"]["groups"]["component"]["nodes"]
        nodes["ei"] = {
            "id": "ei", "node_type": "isos.process.component@v0.2",
            "label": "EI agent", "description": "Managed agent for config preflight",
            "capabilities": ["process"],
            "settings": {"role": "ei", "args": [],
                         "depends_on": ["cores/isos/layers/process/groups/component/nodes/eos"]},
        }
        self.case.apply(graph)
        revision, generation, leaf = self.case.selected()
        manifest = json.loads((leaf / "manifest.json").read_text())
        self.assertEqual(manifest["default_roots"],
                         {"component": str(component), "factory": str(factory)})
        for relative in CASES["required_ei_files"]:
            self.assertTrue((leaf / relative).is_file(), relative)
        self.assertTrue((leaf / "ei/extra/nested.toml").is_file())
        self.assertIn(b"mode = 'isos'", (leaf / "ei/ei.toml").read_bytes())
        graph_before = self.case.graph_path.read_bytes()
        candidate = self.case.root / "candidate.nodes.json"
        for relative, fixture in (("prompts.toml", "missing_prompts"),
                                  ("missions.toml", "missing_missions"),
                                  ("safety/gate.toml", "missing_safety_gate")):
            path = component / relative
            original = path.read_bytes()
            path.unlink()
            self.assert_apply_failure(candidate, revision, fixture)
            path.write_bytes(original)

        models = component / "models.toml"
        original_models = models.read_bytes()
        models.unlink()
        models.symlink_to(self.case.root / "outside-models.toml")
        self.assert_apply_failure(candidate, revision, "default_symlink")
        models.unlink()
        models.write_bytes(original_models)

        adapter = self.case.root / "ei-install/bin/ei-config"
        original_adapter = adapter.read_text()
        adapter.write_text(original_adapter.replace(
            'print(json.dumps(reply))',
            'if request["op"] == "render":\n    reply["ok"] = False\n'
            '    reply["diagnostics"] = [{"path": "/cores/isos/layers/ei_io", '
            '"code": "stub_rejected", "message": "rejected"}]\n'
            'print(json.dumps(reply))'))
        self.assert_apply_failure(candidate, revision, "owner_rejection")
        adapter.write_text(original_adapter.replace(
            'print(json.dumps(reply))',
            'if request["op"] == "render":\n'
            '    reply["files"] = [{"path": "ei/../escape.toml", '
            '"content": "bad = true\\n"}]\n'
            'print(json.dumps(reply))'))
        self.assert_apply_failure(candidate, revision, "overlay_escape")
        adapter.write_text(original_adapter)

        self.assertEqual(self.case.graph_path.read_bytes(), graph_before)
        self.assertEqual(self.case.selected()[:2], (revision, generation))
        (factory / "models.toml").write_text("enabled = false\n")
        self.case.start_server()
        status, reply = self.case.control("run")
        self.assertEqual(status, 422, reply)
        self.assertEqual(
            {key: reply["error"]["details"]["diagnostics"][0][key] for key in ("code", "path", "stage")},
            {"code": "generated_manifest_mismatch", "path": "", "stage": "snapshot"},
        )
        self.assertEqual(self.case.graph_path.read_bytes(), graph_before)



if __name__ == "__main__":
    unittest.main(verbosity=2)
