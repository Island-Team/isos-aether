#!/usr/bin/env python3
"""Exercise selected EOS reader probes and the current graph setup boundary."""

import json
from pathlib import Path
import subprocess
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]
BIN = ROOT / "target/debug"
PROFILE = ROOT / "etc/eos/backends/mock-cxx.toml"
READERS = ("aether-profile", "aether-runtime", "aether-config-server")


def call(binary, *args, request=None):
    result = subprocess.run(
        [str(BIN / binary), *args],
        input=None if request is None else json.dumps(request),
        text=True, capture_output=True, timeout=5, check=True,
    )
    assert len(result.stdout) <= 65536, "probe output exceeds 64 KiB"
    assert len(result.stdout.splitlines()) == 1, "probe must write one JSON object"
    return json.loads(result.stdout)


def setup(op, graph=None):
    request = {"schema_version": 1, "op": op, "core": "eos"}
    if graph is not None:
        request.update(revision="fixture", graph=graph, endpoints={}, packages=[])
    return call("aether-profile", "setup", "--json", request=request)


class ReaderFeaturesReplay(unittest.TestCase):
    def test_selected_probes_advertise_only_current_graph(self):
        probes = {name: call(name, "--capabilities") for name in READERS}
        probes["aether-http-bridge"] = call("aether-http-bridge", "--capabilities")
        for name, probe in probes.items():
            with self.subTest(name=name):
                self.assertEqual(probe["binary"], name)
                self.assertEqual(len(probe["features"]), len(set(probe["features"])))
                if name in READERS:
                    self.assertEqual(probe["graph_version"], "0.2.0")
                    self.assertEqual(probe["supported_graph_versions"], ["0.2.0"])
                    self.assertEqual(probe["supported_node_type_versions"], ["0.2"])
                    ids = probe["supported_node_types"]
                    self.assertTrue(ids)
                    self.assertEqual(ids, sorted(set(ids)))
                    self.assertTrue(all(value.startswith("isos.eos_") and value.endswith("@v0.2") for value in ids))
                else:
                    self.assertNotIn("supported_node_types", probe)

    def test_setup_identity_matches_profile_probe(self):
        probe = call("aether-profile", "--capabilities")
        reply = setup("identity")
        self.assertTrue(reply["ok"], reply)
        self.assertEqual(reply["files"], [])
        for field in ("product_version", "graph_version", "node_type_version",
                      "supported_graph_versions", "supported_node_type_versions",
                      "supported_node_types", "features"):
            self.assertEqual(reply["identity"][field], probe[field], field)

    def test_setup_accepts_current_graph_and_rejects_old_format(self):
        graph = tomllib.loads(PROFILE.read_text())
        self.assertEqual(list(graph["cores"]), ["isos"])
        self.assertEqual(graph["version"], "0.2.0")
        accepted = setup("validate", graph)
        self.assertTrue(accepted["ok"], accepted)
        self.assertEqual(accepted["files"], [])
        rejected = setup("validate", {**graph, "version": "1.0.0"})
        self.assertFalse(rejected["ok"])
        self.assertEqual(rejected["files"], [])


if __name__ == "__main__":
    unittest.main(verbosity=2)
