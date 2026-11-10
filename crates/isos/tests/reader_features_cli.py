#!/usr/bin/env python3
"""Exercise current reader identity admission through the IsOS CLI."""

import copy
import json
from pathlib import Path
import tempfile
import unittest

from generated_config_preflight_cli import CleanHome


class ReaderFeaturesReplay(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="isos-reader-")
        self.case = CleanHome(self.temp.name)
        self.graph = self.case.init()

    def tearDown(self):
        self.case.close()
        self.temp.cleanup()

    def test_selected_readers_admit_current_graph(self):
        self.case.apply(self.graph)
        revision, _, leaf = self.case.selected()
        self.assertEqual(json.loads((leaf / "manifest.json").read_text())["revision"], revision)
        check = self.case.command("version", "check", "--json")
        self.assertEqual(check.returncode, 0, check.stderr)
        eos = next(core for core in json.loads(check.stdout)["cores"] if core["core"] == "eos")
        self.assertTrue(eos["ready"], eos)
        self.assertEqual(eos["diagnostics"], [])

    def test_unregistered_eos_type_rejected(self):
        candidate = copy.deepcopy(self.graph)
        layers = candidate["cores"]["isos"]["layers"]
        layers["eos_dispatcher"]["groups"]["control"]["nodes"]["command"]["node_type"] = "isos.eos_dispatcher.future@v0.2"
        path = Path(self.temp.name) / "invalid.nodes.json"
        path.write_text(json.dumps(candidate))
        result = self.case.command("config", "validate", str(path), success=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unsupported_node_type", result.stderr)


if __name__ == "__main__":
    unittest.main()
