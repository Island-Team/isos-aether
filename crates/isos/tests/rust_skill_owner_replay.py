#!/usr/bin/env python3
"""Replay an IsOS linked Rust skill selection through Apply, Run, Restart, and Stop.

Build `isos`, `aether-*`, and `aether-skill-fixture` first, then run this file.
"""

import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

from generated_config_preflight_cli import CleanHome, EOS_BIN_DIR, WORKSPACE


class RustSkillOwnerReplay(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="isos-rust-skill-")
        self.case = CleanHome(self.temp.name)
        self.case.env["ISOS_AETHER_BIN_DIR"] = str(EOS_BIN_DIR)
        self.case.seed_ei_distribution()
        self.graph = self.case.init()
        fixture = json.loads((WORKSPACE / "crates/isos/tests/fixtures/unified_mujoco/current_draft.json").read_text())
        layers = fixture["cores"]["isos"]["layers"]
        for name in ("eos_hardware", "eos_dispatcher"):
            self.graph["cores"]["isos"]["layers"][name] = layers[name]
        hardware = self.graph["cores"]["isos"]["layers"]["eos_hardware"]
        for group in hardware["groups"].values():
            for node in group["nodes"].values():
                node["adapter"] = {"type": "mock-cxx"}
                node.pop("references", None)

    def tearDown(self):
        self.case.close()
        self.temp.cleanup()

    def probe_skill(self, revision, phase):
        grants = list((self.case.home / "run/control-grants/default").glob("*/ei-control.grant"))
        self.assertEqual(len(grants), 1, grants)
        interpreter = os.environ.get("ISOS_SKILL_PROBE_PYTHON", sys.executable)
        env = dict(os.environ, PYTHONPATH=str(WORKSPACE / "ei/src"))
        result = subprocess.run(
            [interpreter, str(Path(__file__).with_name("skill_grpc_probe.py")),
             "127.0.0.1:16200", revision, grants[0].read_text().strip(), phase],
            env=env, capture_output=True, text=True, timeout=20)
        eos_log = self.case.home / "data/isos/log/deployments/default/events/eos.jsonl"
        self.assertEqual(result.returncode, 0,
                         result.stdout + result.stderr + eos_log.read_text()[-4000:])
        print(result.stdout.strip(), flush=True)

    def test_selected_package_survives_restart_and_stops(self):
        source = Path(self.temp.name) / "skill-bundle"
        executable = source / "lib/island/skills/example.skill"
        executable.parent.mkdir(parents=True)
        for directory in (source, source / "lib", source / "lib/island", executable.parent):
            directory.chmod(0o755)
        fixture = Path(os.environ.get(
            "ISOS_SKILL_FIXTURE_BIN", EOS_BIN_DIR / "aether-skill-fixture"))
        self.assertTrue(fixture.is_file(), fixture)
        shutil.copyfile(fixture, executable)
        executable.chmod(0o755)
        release = {
            "contract": "isos.package@v1", "schema_version": "0.1.0",
            "id": "test/rust-skill", "kind": "skill", "version": "0.2.1",
            "node_contributions": {}, "source_commit": "a" * 40,
            "assets": [{"os": "linux", "arch": os.uname().machine,
                        "url": "https://github.com/Island-Team/fixture/releases/download/v0.2.1/skill.tar.gz",
                        "size_bytes": 1, "sha256": "b" * 64}],
        }
        (source / "release.json").write_text(json.dumps(release))
        (source / "release.json").chmod(0o600)
        self.case.command("pkg", "link", "test/rust-skill", "0.2.1", source)
        node = self.graph["cores"]["isos"]["layers"]["process"]["groups"]["component"]["nodes"]["eos"]
        node["settings"]["skill_executables"] = [{
            "package": {"id": "test/rust-skill", "version": "0.2.1"},
            "executable": "lib/island/skills/example.skill",
        }]
        revision = self.case.apply(self.graph)
        _, _, leaf = self.case.selected()
        selection = json.loads((leaf / "isos/skill-selection.json").read_text())
        self.assertEqual(selection["active_revision"], revision)
        self.assertEqual(selection["executables"][0]["executable_sha256"],
                         hashlib.sha256(executable.read_bytes()).hexdigest())
        self.case.start_server()
        for operation in ("run", "restart", "stop"):
            status, reply = self.case.control(operation, timeout=90)
            self.assertEqual(status, 200, (operation, reply))
            if operation != "stop":
                self.probe_skill(revision, "after_restart" if operation == "restart" else "before_restart")
        status, state = self.case.api("runtime/process-status")
        self.assertEqual(status, 200, state)
        self.assertEqual(state["data"]["state"], "stopped")
        self.assertEqual(json.loads(self.case.graph_path.read_text()),
                         json.loads((self.case.root / "candidate.nodes.json").read_text()))


if __name__ == "__main__":
    unittest.main()
