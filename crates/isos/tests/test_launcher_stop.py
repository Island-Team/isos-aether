"""The shell launcher must finish relaying Stop warnings before returning."""

import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[3]


class LauncherStopTests(unittest.TestCase):
    def test_stop_waits_for_stderr_relay_in_dev_and_product_modes(self):
        for mode in ("dev", "product"):
            for status in (0, 7):
                with self.subTest(mode=mode, status=status), tempfile.TemporaryDirectory() as directory:
                    checkout = Path(directory)
                    shutil.copy(ROOT / "isos.sh", checkout / "isos.sh")
                    (checkout / "config/isos").mkdir(parents=True)
                    (checkout / "config/isos/launch.toml").write_text(f'[launch]\nmode = "{mode}"\n')
                    (checkout / "scripts").mkdir()
                    (checkout / "scripts/jsonl_log.py").write_text(
                        "import pathlib, sys, time\n"
                        "if sys.argv[4].startswith('child_stderr'):\n"
                        "    time.sleep(0.2)\n"
                        "path = pathlib.Path(sys.argv[1])\n"
                        "with path.open('a') as out: out.write(sys.argv[4] + '\\n')\n"
                    )
                    binary = checkout / ("target/debug/isos" if mode == "dev" else "installed/bin/isos")
                    binary.parent.mkdir(parents=True)
                    binary.write_text("#!/bin/sh\nprintf 'Stop returned\\n'\nprintf 'initial warning\\nfinal warning\\n' >&2\nexit \"$FAKE_STATUS\"\n")
                    binary.chmod(0o755)
                    output = checkout / "stdout"
                    errors = checkout / "stderr"
                    env = os.environ | {"FAKE_STATUS": str(status), "ISOS_INSTALL_ROOT": str(checkout / "installed")}
                    with output.open("w") as stdout, errors.open("w") as stderr:
                        result = subprocess.run([str(checkout / "isos.sh"), "stop"], stdout=stdout,
                                                stderr=stderr, env=env, timeout=5, check=False)
                    self.assertEqual(result.returncode, status, errors.read_text())
                    self.assertIn("Stop returned", output.read_text())
                    self.assertIn("initial warning\nfinal warning", errors.read_text())
                    logs = list((checkout / "data/isos/log/launch").rglob("launcher.jsonl"))
                    self.assertEqual(len(logs), 1)
                    self.assertIn("child_stderr initial warning\nchild_stderr final warning", logs[0].read_text())


if __name__ == "__main__":
    unittest.main()
