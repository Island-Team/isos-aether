#!/usr/bin/env python3
"""Replay unified graph owner validation with the real EI and EOS adapters."""

import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import time
import unittest

from generated_config_preflight_cli import CleanHome


ROOT = Path(__file__).resolve().parents[3]
EI = ROOT / "ei"
BIN = ROOT / "target/debug"


def unified_graph(graph):
    """Add the registered EI stages to a current IsOS starter graph."""
    fixture = json.loads((Path(__file__).parent / "fixtures/ei_stage_reader/valid_ei.json").read_text())
    assert graph["version"] == fixture["version"] == "0.2.0"
    assert list(graph["cores"]) == list(fixture["cores"]) == ["isos"]
    graph["cores"]["isos"]["layers"].update(fixture["cores"]["isos"]["layers"])
    graph["cores"]["isos"]["layers"]["process"]["groups"]["component"]["nodes"]["ei"] = {
        "id": "ei", "node_type": "isos.process.component@v0.2",
        "label": "EI agent", "description": "Selected EI stage runtime",
        "capabilities": ["process"],
        "settings": {"role": "ei", "args": [], "depends_on": [
            "cores/isos/layers/process/groups/component/nodes/bridge"]},
    }
    return graph


class UnifiedOwnerReplay(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="isos-unified-owner-")
        self.case = CleanHome(self.temp.name)
        self.case.env.update({
            "ISOS_AETHER_BIN_DIR": str(BIN),
            "ISOS_EI_BIN_DIR": str(EI / "bin"),
            "ISOS_EI_DEFAULTS_DIR": str(ROOT / "config/ei"),
            "ISOS_EI_FACTORY_DEFAULTS_DIR": str(EI.parent / "share/ei/defaults/ei"),
            "EI_PYTHON": str(EI / ".venv/bin/python"),
            "ISLAND_PREFIX": str(EI),
        })
        self.graph = unified_graph(self.case.init())
        self.saved = self.case.graph_path.read_bytes()

    def tearDown(self):
        self.case.close()
        self.temp.cleanup()

    def test_real_owner_validation_and_revision_bound_render(self):
        candidate = self.case.root / "unified.nodes.json"
        candidate.write_text(json.dumps(self.graph))
        revision = hashlib.sha256(candidate.read_bytes()).hexdigest()
        for role, executable, layer in (("eos", BIN / "aether-profile", "eos_dispatcher"),
                                        ("ei", EI / "bin/ei-config", "ei_io")):
            projection = {"version": "0.2.0", "robot": self.graph["robot"],
                          "cores": {"isos": {"layers": {name: content for name, content in
                          self.graph["cores"]["isos"]["layers"].items()
                          if name.startswith("eos_" if role == "eos" else "ei_")}}}}
            request = {"schema_version": 2, "op": "render", "core": role,
                       "revision": revision, "graph": projection, "packages": [],
                       "endpoints": {"eos_gateway": {"transport": "tcp", "host": "127.0.0.1", "port": 16200},
                                     "ei_northbound": {"host": "127.0.0.1", "port": 16060}}}
            env = self.case.env.copy()
            validate = dict(request, op="validate")
            checked = subprocess.run([str(executable), "setup", "--json"], input=json.dumps(validate),
                                     env=env, text=True, capture_output=True, timeout=15)
            self.assertEqual(checked.returncode, 0, checked.stderr)
            self.assertTrue(json.loads(checked.stdout)["ok"], checked.stdout)
            output = subprocess.run([str(executable), "setup", "--json"], input=json.dumps(request),
                                    env=env, text=True, capture_output=True, timeout=15)
            self.assertEqual(output.returncode, 0, output.stderr)
            reply = json.loads(output.stdout)
            self.assertTrue(reply["ok"], reply)
            self.assertTrue(projection["cores"]["isos"]["layers"][layer])
            if role == "ei":
                stage = next(item for item in reply["files"] if item["path"] == "ei/stages.json")
                rendered = json.loads(stage["content"])
                self.assertEqual(rendered["revision"], revision)
                self.assertEqual(rendered["cores"], projection["cores"])
        self.case.command("config", "validate", candidate)
        self.assertEqual(self.case.graph_path.read_bytes(), self.saved)

    def test_rejects_unknown_ei_type_with_precise_path_and_preserves_saved_graph(self):
        original = self.case.graph_path.read_bytes()
        node = self.graph["cores"]["isos"]["layers"]["ei_io"]["groups"]["link"]["nodes"]["gateway"]
        node["node_type"] = "isos.ei_io.future@v0.2"
        candidate = self.case.root / "invalid.nodes.json"
        candidate.write_text(json.dumps(self.graph))
        result = self.case.command("config", "validate", candidate, success=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("/cores/isos/layers/ei_io/groups/link/nodes/gateway/node_type", result.stderr)
        self.assertEqual(self.case.graph_path.read_bytes(), original)

    def test_logical_eos_service_binding_validates_and_applies_without_address(self):
        node = self.graph["cores"]["isos"]["layers"]["ei_io"]["groups"]["link"]["nodes"]["gateway"]
        del node["settings"]["gateway"]
        node["service_bindings"] = {"gateway": {
            "owner": "eos", "service": "island.eos.v1.GatewayService", "version": "1.0.0"
        }}
        candidate = self.case.root / "logical-binding.nodes.json"
        candidate.write_text(json.dumps(self.graph))
        self.case.command("config", "validate", candidate)
        original_revision = hashlib.sha256(self.saved).hexdigest()
        self.case.command("config", "apply", candidate, "--revision", original_revision)
        applied = json.loads(self.case.graph_path.read_text())
        saved_node = applied["cores"]["isos"]["layers"]["ei_io"]["groups"]["link"]["nodes"]["gateway"]
        self.assertEqual(saved_node["service_bindings"], node["service_bindings"])
        self.assertNotIn("gateway", saved_node["settings"])

    def test_owner_options_reach_isos_catalog_for_draft(self):
        self.case.start_server()
        for _ in range(100):
            _, health = self.case.api("system/health")
            if all(owner["state"] == "idle" for owner in health["data"]["owners"]):
                break
            time.sleep(0.05)
        status, reply = self.case.api("catalog/node-options", {
            "schema_version": "0.1.0", "graph": self.graph,
        })
        self.assertEqual(status, 200, reply)
        groups = reply["data"]["setting_options"]
        planner = "isos/ei_cognition/decision/planner"
        for setting in ("controller.model", "reasoner.model"):
            group = next(row for row in groups if row["node_path"] == planner
                         and row["setting"] == setting)
            self.assertIsNone(group["selected"])
            self.assertTrue(group["options"])
            self.assertTrue(all(isinstance(option["available"], bool)
                                for option in group["options"]))


if __name__ == "__main__":
    unittest.main()
