"""EI node declarations loaded from IsOS-verified package stores."""

from __future__ import annotations

import ctypes
import hashlib
import json
import re
from pathlib import Path
from typing import Any

from ei.core.system.isolated import PORTS

BUILTIN_PLACEMENTS = {
  "ei.io.input.trigger@v0.2": ("io", "input"),
  "ei.cognition.intent.context@v0.2": ("cognition", "intent"),
  "ei.cognition.intent.forwarder@v0.2": ("cognition", "intent"),
  "ei.governance.safety.gate@v0.2": ("governance", "safety"),
  "ei.io.input.arm_status@v0.2": ("io", "input"),
  "ei.cognition.state.arm_status@v0.2": ("cognition", "state"),
  "ei.governance.monitor.arm_status@v0.2": ("governance", "monitor"),
  "ei.io.link.gateway@v0.2": ("io", "link"),
  "ei.evidence.perception.admission@v0.2": ("evidence", "perception"),
  "ei.cognition.context.builder@v0.2": ("cognition", "context"),
  "ei.cognition.missions.queue@v0.2": ("cognition", "missions"),
  "ei.cognition.decision.planner@v0.2": ("cognition", "decision"),
  "ei.governance.monitor.health@v0.2": ("governance", "monitor"),
  **{node_id: ("cognition", spec[0]) for node_id, spec in PORTS.items()},
}


class _NodeApi(ctypes.Structure):
  _fields_ = [
    ("abi_version", ctypes.c_uint32),
    ("struct_size", ctypes.c_uint32),
    ("create", ctypes.c_void_p),
    ("start", ctypes.c_void_p),
    ("input", ctypes.c_void_p),
    ("health", ctypes.c_void_p),
    ("stop", ctypes.c_void_p),
    ("destroy", ctypes.c_void_p),
  ]


def close_library(library: ctypes.CDLL) -> None:
  loader = ctypes.CDLL("libdl.so.2")
  loader.dlclose.argtypes = [ctypes.c_void_p]
  loader.dlclose.restype = ctypes.c_int
  loader.dlclose(library._handle)


class NodeRegistryError(ValueError):
  """Reject an unverifiable or unsupported EI contribution."""


def _safe_file(root: Path, name: str) -> Path:
  if not isinstance(name, str) or not name or name.startswith("/"):
    raise NodeRegistryError("node library must be package-relative")
  parts = Path(name).parts
  if any(part in {"", ".", ".."} for part in parts):
    raise NodeRegistryError("unsafe node library path")
  path = root.joinpath(*parts)
  if not path.resolve().is_relative_to(root.resolve()) or not path.is_file():
    raise NodeRegistryError("node library is missing or escapes package store")
  return path


def _implementation(
  root: Path, node: dict[str, Any], libraries: dict[str, str]
) -> None:
  implementation = node.get("implementation")
  if not isinstance(implementation, dict):
    raise NodeRegistryError("EI node implementation is required")
  kind = implementation.get("kind")
  if kind == "native_plugin":
    name = implementation.get("library")
    if (
      implementation.get("abi") != "isos_node_v1"
      or not isinstance(name, str)
      or not name.endswith(".so")
    ):
      raise NodeRegistryError("EI supports only isos_node_v1 Linux .so nodes")
    digest = libraries.get(name)
    if digest is None or not re.fullmatch(r"[0-9a-f]{64}", digest):
      raise NodeRegistryError("EI node library lacks a declared SHA-256")
    path = _safe_file(root, name)
    if hashlib.sha256(path.read_bytes()).hexdigest() != digest:
      raise NodeRegistryError("EI node library digest changed")
    library: ctypes.CDLL | None = None
    try:
      library = ctypes.CDLL(str(path), mode=ctypes.RTLD_LOCAL)
      entry = library.isos_node_v1
      entry.argtypes = []
      entry.restype = ctypes.POINTER(_NodeApi)
      table = entry()
      if (
        not table
        or table.contents.abi_version != 1
        or table.contents.struct_size < ctypes.sizeof(_NodeApi)
        or any(
          not getattr(table.contents, name)
          for name in ("create", "start", "input", "health", "stop", "destroy")
        )
      ):
        raise NodeRegistryError("EI node ABI table is incomplete or unsupported")
    except (OSError, AttributeError, ValueError) as exc:
      raise NodeRegistryError("EI node ABI cannot be loaded") from exc
    finally:
      if library is not None:
        close_library(library)
  elif kind == "external":
    if (
      not isinstance(implementation.get("interface"), str)
      or not implementation["interface"]
      or implementation.get("version") != "1"
      or not isinstance(implementation.get("lifecycle_adapter"), str)
      or not implementation["lifecycle_adapter"]
    ):
      raise NodeRegistryError("EI external node interface is invalid")
  else:
    raise NodeRegistryError("unsupported EI node implementation")


def load_registry(packages: Any) -> dict[str, dict[str, Any]]:
  """Return built-ins and exact EI declarations from verified package references.

  IsOS authenticates the package store. EI rechecks the release and library bytes
  before claiming support, so a changed store cannot become an executable route.
  """
  registry: dict[str, dict[str, Any]] = {
    node_id: {"id": node_id, "core": "ei", "layer": layer, "group": group}
    for node_id, (layer, group) in BUILTIN_PLACEMENTS.items()
  }
  if not isinstance(packages, list) or len(packages) > 256:
    raise NodeRegistryError("packages must be a bounded array")
  for package in packages:
    if (
      not isinstance(package, dict)
      or not {"id", "version", "path"} <= set(package)
      or set(package) - {"id", "version", "path", "payload_root"}
    ):
      raise NodeRegistryError("invalid verified package reference")
    root = Path(package["path"])
    payload_root = Path(package.get("payload_root", package["path"]))
    if not root.is_absolute() or not payload_root.is_absolute():
      raise NodeRegistryError("package store must be absolute")
    release_path = _safe_file(root, "release.json")
    if release_path.stat().st_size > 4_194_304:
      raise NodeRegistryError("package release is too large")
    release = json.loads(release_path.read_bytes())
    if (
      release.get("id") != package["id"] or release.get("version") != package["version"]
    ):
      raise NodeRegistryError("package release identity changed")
    contributions = release.get("node_contributions") or {}
    libraries = {
      item["path"]: item["sha256"]
      for item in contributions.get("libraries", [])
      if isinstance(item, dict)
      and isinstance(item.get("path"), str)
      and isinstance(item.get("sha256"), str)
    }
    for node in contributions.get("node_types", []):
      if not isinstance(node, dict) or node.get("core") != "ei":
        continue
      node_id = node.get("id")
      if not isinstance(node_id, str) or not re.fullmatch(
        r"ei\.[a-z0-9_]+\.[a-z0-9_]+\.[a-z0-9_]+@v0\.2", node_id
      ):
        raise NodeRegistryError("invalid EI node type ID")
      if node_id in registry:
        raise NodeRegistryError(f"duplicate EI node type {node_id}")
      if not all(
        isinstance(node.get(key), str) and node[key] for key in ("layer", "group")
      ):
        raise NodeRegistryError("EI node placement is missing")
      _implementation(payload_root, node, libraries)
      metadata: dict[str, Any] = {
        "_package_root": str(payload_root),
        "_package_id": package["id"],
        "_package_version": package["version"],
      }
      if node["implementation"]["kind"] == "external":
        matches = [
          item
          for item in contributions.get("processes", [])
          if isinstance(item, dict) and item.get("node_type") == node_id
        ]
        if (
          len(matches) != 1
          or matches[0].get("lifecycle_adapter")
          != node["implementation"]["lifecycle_adapter"]
        ):
          raise NodeRegistryError("EI external node requires one matching process")
        entrypoint = matches[0].get("entrypoint")
        if not isinstance(entrypoint, str):
          raise NodeRegistryError("EI external entrypoint is invalid")
        _safe_file(payload_root, entrypoint)
        metadata["_entrypoint"] = entrypoint
      registry[node_id] = {**node, **metadata}
  return registry


def load_claims(
  packages: list[dict[str, Any]], registry: dict[str, dict[str, Any]]
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
  """Publish only explicit package route and bridge declarations owned by EI."""
  routes: list[dict[str, Any]] = []
  bridges: list[dict[str, Any]] = []
  for package in packages:
    root = Path(package["path"])
    release = json.loads(_safe_file(root, "release.json").read_bytes())
    contributions = release.get("node_contributions") or {}
    for route in contributions.get("port_routes", []):
      if not isinstance(route, dict):
        raise NodeRegistryError("invalid package port route")
      source, target = route.get("source_type"), route.get("target_type")
      if not isinstance(source, str) or not isinstance(target, str):
        raise NodeRegistryError("invalid package route endpoint")
      source_ei, target_ei = source in registry, target in registry
      if not source_ei and not target_ei:
        continue
      if source in BUILTIN_PLACEMENTS or target in BUILTIN_PLACEMENTS:
        raise NodeRegistryError(
          "EI package route cannot replace built-in safety delivery"
        )
      if (not source_ei and not source.startswith("eos.")) or (
        not target_ei and not target.startswith("eos.")
      ):
        raise NodeRegistryError("EI package route requires EI or EOS endpoint")
      # The route declaration comes from a verified package, while its endpoints
      # may be registered by other verified packages selected for this run.
      expected_carrier = (
        "unix_socket"
        if not source_ei
        or not target_ei
        or registry[source]["implementation"]["kind"] == "external"
        or registry[target]["implementation"]["kind"] == "external"
        else "in_process"
      )
      if (
        route.get("transport") != expected_carrier
        or route.get("delivery_mode") != "event"
      ):
        raise NodeRegistryError("unsupported EI package carrier")
      if (
        not isinstance(route.get("contract"), dict)
        or route["contract"].get("version") != "1"
      ):
        raise NodeRegistryError("invalid EI package route contract")
      routes.append(route)
    for bridge in contributions.get("bridge_adapters", []):
      if not isinstance(bridge, dict) or bridge.get("core") != "ei":
        continue
      node_id = bridge.get("node_type")
      if node_id not in registry or registry[node_id].get("_package_root") != str(
        Path(package.get("payload_root", package["path"]))
      ):
        raise NodeRegistryError("EI package bridge node is unregistered")
      if bridge.get("adapter") != "unix_socket":
        raise NodeRegistryError("unsupported EI package bridge adapter")
      bridges.append(bridge)
  return routes, bridges
