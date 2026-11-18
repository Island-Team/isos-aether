#!/usr/bin/env python3
"""Validate that the contract catalog classifies every source artifact once."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

KINDS = {
  "graph-schema", "native-abi", "protobuf-service", "protobuf-message",
  "json-schema", "protobuf-dependency", "generated-json-schema",
  "generated-openapi",
}
OWNERS = {"isos", "ei", "eos"}
ID = re.compile(r"^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$")
SUFFIXES = {".h", ".proto", ".json"}
ORIGINS = {"authored", "checked", "generated", "vendored"}


def validate(root: Path) -> dict[str, str]:
  catalog = json.loads((root / "catalog.json").read_text(encoding="utf-8"))
  if set(catalog) != {"format_version", "catalogs"} or catalog["format_version"] != 1:
    raise ValueError("catalog requires format_version 1 and catalogs")
  entries = catalog["catalogs"]
  if not isinstance(entries, list) or not entries:
    raise ValueError("catalogs must be a non-empty array")
  assigned: dict[str, str] = {}
  ids: set[str] = set()
  for entry in entries:
    if not isinstance(entry, dict) or set(entry) != {
      "id", "kind", "owner", "consumers", "domains", "paths", "purpose", "origin"
    }:
      raise ValueError("each catalog needs id, kind, owner, consumers, domains, paths, purpose, origin")
    identifier = entry["id"]
    if not isinstance(identifier, str) or not ID.fullmatch(identifier) or identifier in ids:
      raise ValueError(f"invalid or duplicate catalog id: {identifier!r}")
    ids.add(identifier)
    if (not isinstance(entry["kind"], str) or entry["kind"] not in KINDS or
        not isinstance(entry["owner"], str) or entry["owner"] not in OWNERS):
      raise ValueError(f"{identifier}: invalid kind or owner")
    if (not isinstance(entry["origin"], str) or entry["origin"] not in ORIGINS or
        not isinstance(entry["purpose"], str) or not entry["purpose"].strip()):
      raise ValueError(f"{identifier}: invalid origin or purpose")
    for field, allowed in (("consumers", OWNERS), ("domains", None)):
      labels = entry[field]
      if not isinstance(labels, list) or not labels or any(not isinstance(label, str) for label in labels):
        raise ValueError(f"{identifier}: {field} must contain string labels")
      if len(labels) != len(set(labels)):
        raise ValueError(f"{identifier}: {field} must contain unique labels")
      if any(not isinstance(label, str) or not ID.fullmatch(label) or
             (allowed is not None and label not in allowed) for label in labels):
        raise ValueError(f"{identifier}: invalid {field} label")
    patterns = entry["paths"]
    if not isinstance(patterns, list) or not patterns or any(not isinstance(pattern, str) for pattern in patterns):
      raise ValueError(f"{identifier}: paths must contain string patterns")
    if len(patterns) != len(set(patterns)):
      raise ValueError(f"{identifier}: paths must contain unique patterns")
    for pattern in patterns:
      if not isinstance(pattern, str) or pattern.startswith("/") or ".." in Path(pattern).parts:
        raise ValueError(f"{identifier}: unsafe pattern {pattern!r}")
      matches = sorted(path for path in root.glob(pattern) if path.is_file())
      if not matches:
        raise ValueError(f"{identifier}: pattern matches no files: {pattern}")
      for path in matches:
        relative = path.relative_to(root).as_posix()
        if path.suffix not in SUFFIXES or path.is_symlink():
          raise ValueError(f"{identifier}: unsupported artifact: {relative}")
        if relative in assigned:
          raise ValueError(f"{relative}: assigned to both {assigned[relative]} and {identifier}")
        assigned[relative] = identifier
  actual = {
    path.relative_to(root).as_posix()
    for path in root.rglob("*") if path.is_file() and path.suffix in SUFFIXES
    and path.name != "catalog.json"
  }
  missing = sorted(actual - assigned.keys())
  if missing:
    raise ValueError(f"unclassified contracts: {', '.join(missing)}")
  return assigned


def main() -> int:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1] / "contract")
  parser.add_argument("--list", action="store_true", help="list classified paths")
  parser.add_argument("--catalog", help="list only this catalog ID")
  parser.add_argument("--domain", help="list only catalogs with this domain label")
  parser.add_argument("--consumer", choices=sorted(OWNERS), help="list only catalogs read by this component")
  parser.add_argument("--explain", help="explain one path relative to the contract root")
  args = parser.parse_args()
  try:
    assigned = validate(args.root)
    catalogs = json.loads((args.root / "catalog.json").read_text(encoding="utf-8"))["catalogs"]
  except (OSError, ValueError, json.JSONDecodeError) as error:
    print(f"contract catalog: {error}", file=sys.stderr)
    return 1
  if args.explain:
    path = Path(args.explain).as_posix()
    if path not in assigned:
      print(f"contract catalog: unclassified path: {path}", file=sys.stderr)
      return 1
    entry = next(entry for entry in catalogs if entry["id"] == assigned[path])
    print(f"Path: {path}")
    print(f"Catalog: {entry['id']}")
    print(f"Purpose: {entry['purpose']}")
    print(f"Origin: {entry['origin']}")
    print(f"Kind: {entry['kind']}")
    print(f"Owner: {entry['owner']}")
    print(f"Consumers: {', '.join(entry['consumers'])}")
    print(f"Domains: {', '.join(entry['domains'])}")
    source = Path(path)
    if entry["origin"] == "generated" and source.parts[:2] in {("schemas", section) for section in ("actions", "body", "world")}:
      proto = Path("proto", *source.parts[1:]).with_suffix(".proto")
      print(f"Generated from: {proto.as_posix()}")
    elif path == "schemas/north-http.openapi.json":
      print("Generated from: proto/external/http/http.proto annotations")
    elif entry["origin"] == "checked":
      print("Checked against: ei/io/models/")
    if source.suffix == ".json" and source.name != "catalog.json":
      document = json.loads((args.root / source).read_text(encoding="utf-8"))
      description = document.get("description")
      if isinstance(description, str) and description.strip():
        print(f"Description: {' '.join(description.split())}")
  elif args.list or args.catalog or args.domain or args.consumer:
    selected = {
      entry["id"] for entry in catalogs
      if (args.catalog is None or entry["id"] == args.catalog)
      and (args.domain is None or args.domain in entry["domains"])
      and (args.consumer is None or args.consumer in entry["consumers"])
    }
    for path, catalog in sorted(assigned.items()):
      if catalog in selected:
        print(f"{catalog}\t{path}")
  else:
    print(f"Contract catalog: {len(assigned)} artifacts in {len(catalogs)} groups")
    for entry in catalogs:
      count = sum(catalog == entry["id"] for catalog in assigned.values())
      print(f"{entry['id']} ({count}, {entry['origin']}): {entry['purpose']}")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
