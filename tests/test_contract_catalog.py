"""Contract catalog coverage and rejection tests."""

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from scripts.check_contract_catalog import validate

ROOT = Path(__file__).resolve().parents[1] / "contract"


class ContractCatalogTests(unittest.TestCase):
  def test_repository_catalog_covers_contracts(self):
    assigned = validate(ROOT)
    self.assertIn("node/node-type.schema.json", assigned)
    self.assertIn("proto/gateway.proto", assigned)
    self.assertIn("schemas/eos.manifest.json", assigned)
    self.assertIn("schemas/north-http.openapi.json", assigned)

  def test_explain_generated_schema_points_to_source(self):
    script = Path(__file__).resolve().parents[1] / "scripts/check_contract_catalog.py"
    result = subprocess.run(
      [sys.executable, str(script), "--explain", "schemas/actions/arm_movej.v1.json"],
      check=True, capture_output=True, text=True,
    )
    self.assertIn("Generated from: proto/actions/arm_movej.v1.proto", result.stdout)
    self.assertIn("Description: Release one complete", result.stdout)

  def test_rejects_unclassified_and_duplicate_artifacts(self):
    with tempfile.TemporaryDirectory() as directory:
      root = Path(directory)
      (root / "example.proto").write_text('syntax = "proto3";\n')
      entry = {
        "id": "service", "kind": "protobuf-service", "owner": "isos",
        "consumers": ["ei"], "domains": ["owner-exchange"],
        "paths": ["example.proto"], "purpose": "Describe a test service.",
        "origin": "authored",
      }
      catalog = {"format_version": 1, "catalogs": [entry]}
      (root / "catalog.json").write_text(json.dumps(catalog))
      self.assertEqual(validate(root), {"example.proto": "service"})
      (root / "unlisted.proto").write_text('syntax = "proto3";\n')
      with self.assertRaisesRegex(ValueError, "unclassified contracts"):
        validate(root)
      (root / "unlisted.proto").unlink()
      catalog["catalogs"].append({**entry, "id": "second"})
      (root / "catalog.json").write_text(json.dumps(catalog))
      with self.assertRaisesRegex(ValueError, "assigned to both"):
        validate(root)


if __name__ == "__main__":
  unittest.main()
