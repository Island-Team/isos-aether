#!/usr/bin/env python3
"""Validate current reader feature cases."""
import json
from pathlib import Path

fixture = json.loads((Path(__file__).parent / "cases.json").read_text())
assert fixture["schema_version"] == 1
assert fixture["graph_format"] == "0.2.0"
cases = fixture["cases"]
assert len(cases) == len({case["id"] for case in cases})
for case in cases:
    assert case["id"] and case["operation"] and case["mutation"]
    expected = case["expected"]
    if expected["result"] == "reject":
        assert expected["code"].startswith("reader_")
        assert expected["path"].startswith("/")
        assert expected["saved_bytes_unchanged"]
        assert expected["stop_calls"] == expected["spawn_calls"] == 0
print(f"P05 fixture valid: {len(cases)} current cases")
