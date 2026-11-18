"""Read the shared resource roots selected by the product configuration."""

from __future__ import annotations

import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import Any


class ConfigError(ValueError):
  """Raised when the product resource configuration is invalid."""


@dataclass(frozen=True, slots=True)
class ProductConfig:
  """The product settings EI needs to locate shared resources."""

  share: tuple[str, ...]


def load(path: Path, *, defaults: Path | None = None) -> ProductConfig:
  """Layer factory defaults beneath deployment settings and read resource roots."""
  document = _read(defaults, "product defaults") if defaults is not None else {}
  document = _deep_merge(document, _read(path, "product config"))
  return ProductConfig(share=_paths(document, "share"))


def _read(path: Path, label: str) -> dict[str, Any]:
  try:
    with path.open("rb") as stream:
      return tomllib.load(stream)
  except FileNotFoundError as exc:
    raise ConfigError(f"{label} not found: {path}") from exc
  except (OSError, tomllib.TOMLDecodeError) as exc:
    raise ConfigError(f"cannot read {label} {path}: {exc}") from exc


def _deep_merge(base: dict[str, Any], overlay: dict[str, Any]) -> dict[str, Any]:
  merged = dict(base)
  for key, value in overlay.items():
    current = merged.get(key)
    if isinstance(current, dict) and isinstance(value, dict):
      merged[key] = _deep_merge(current, value)
    else:
      merged[key] = value
  return merged


def _paths(document: dict[str, Any], key: str) -> tuple[str, ...]:
  value = document.get(key)
  if not isinstance(value, list) or not value:
    raise ConfigError(f"{key} must be a non-empty list of paths")
  paths: list[str] = []
  for item in value:
    if not isinstance(item, str) or not item.strip():
      raise ConfigError(f"{key} entries must be non-empty strings")
    path = Path(item.strip())
    if path.is_absolute():
      raise ConfigError(f"{key} entries must be prefix-relative paths: {item}")
    normalized = path.as_posix()
    if normalized in paths:
      raise ConfigError(f"duplicate {key} path: {normalized}")
    paths.append(normalized)
  return tuple(paths)
