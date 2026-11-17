#!/usr/bin/env python3
"""Check frozen R03 design cases and current documentation anchors."""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent


def check() -> None:
  data = json.loads((HERE / "cases.json").read_text())
  assert set(data) == {"schema_version", "gate", "status", "cases"}
  assert data["schema_version"] == 1 and data["gate"] == "R03"
  assert data["status"] == "design_review_pending"
  cases = data["cases"]
  assert len(cases) == 25
  assert [case["id"] for case in cases] == [f"R03-{i:02d}" for i in range(1, 26)]
  assert all(set(case) == {"id", "class", "stimulus", "expect", "evidence"} for case in cases)
  assert all(all(isinstance(value, str) and value for value in case.values()) for case in cases)
  assert {case["evidence"] for case in cases} == {"unit", "boundary", "integration", "journal"}
  assert "same graph" in cases[17]["stimulus"]
  assert "PERMISSION_DENIED" in cases[20]["expect"]
  assert "old/current target union" in cases[22]["expect"]
  grpc = (ROOT / "docs/isos/interfaces/server/eos.md").read_text()
  dispatcher = (ROOT / "docs/eos/core/dispatcher/dispatcher.md").read_text()
  assert "R03 governed actuator admission target contract" in grpc
  for term in ("R03 governed actuator admission design", "run_id", "OUTCOME_UNKNOWN", "First-use replay"):
    assert term in dispatcher


if __name__ == "__main__":
  check()
  print("R03 design fixture: 25 cases and documentation anchors valid")
