"""Resolve immutable Island-ei filesystem roles at composition roots.

Submodules receive a :class:`Layout`; they must not rediscover the checkout,
inspect the current working directory, or interpret path environment variables.
"""

from __future__ import annotations

import os
from collections.abc import Mapping
from dataclasses import dataclass
from enum import StrEnum
from pathlib import Path


class Mode(StrEnum):
  """Supported repository and installation layouts."""

  SOURCE = "source"
  PORTABLE = "portable"
  SYSTEM = "system"


@dataclass(frozen=True, slots=True)
class Layout:
  """Resolved filesystem roots shared by Island composition code.

  All values are absolute, normalized paths. The object does not create any
  directories; launchers decide which writable roots their process needs.
  """

  mode: Mode
  prefix: Path
  etc: Path
  share: Path
  libexec: Path
  state: Path
  cache: Path
  log: Path
  run: Path
  tmp: Path
  src: Path | None = None

  @property
  def is_source_ei_checkout(self) -> bool:
    """Whether this layout selects EI inside the root IsOS checkout."""
    return self.mode is Mode.SOURCE and _is_ei_checkout(self.prefix)

  @property
  def contracts(self) -> Path:
    """Canonical versioned wire-contract root."""
    return self.share / "contracts"

  @property
  def prompts(self) -> Path:
    """Factory prompt root shipped with EI."""
    return self.share / "prompts"

  @property
  def ei_defaults(self) -> Path:
    """Factory configuration inspected by IsOS before managed launch."""
    if self.mode is Mode.SYSTEM:
      return self.prefix / "share/defaults/ei"
    if self.is_source_ei_checkout:
      return self.share / "ei/defaults/ei"
    return self.share / "defaults/ei"

  @property
  def doc(self) -> Path:
    """Product-documentation root for the selected filesystem layout."""
    if self.mode is Mode.SOURCE:
      if self.is_source_ei_checkout:
        return self.prefix.parent / "docs"
      return self.prefix / "docs"
    if self.mode is Mode.SYSTEM:
      return self.prefix / "share/doc/island"
    return self.share / "doc"

  def component_etc(self, component: str) -> Path:
    """Return one component's namespaced configuration root."""
    if self.is_source_ei_checkout and component == "ei":
      return self.prefix.parent / "config/ei"
    return self.etc / _component(component)

  def component_state(self, component: str) -> Path:
    """Return one component's durable-state root."""
    return self.state / _component(component) / "state"

  def component_cache(self, component: str) -> Path:
    """Return one component's rebuildable-cache root."""
    return self.cache / _component(component) / "cache"

  def component_log(self, component: str) -> Path:
    """Return one component's diagnostic-log root."""
    return self.log / _component(component) / "log"

  def component_run(self, component: str) -> Path:
    """Return one component's process-lifetime root."""
    if self.mode is Mode.SYSTEM and self.run == Path("/run/island"):
      return self.run / _component(component)
    return self.run / _component(component) / "run"

  def component_tmp(self, component: str) -> Path:
    """Return one component's disposable scratch root."""
    return self.tmp / _component(component) / "tmp"

  @classmethod
  def resolve(
    cls,
    *,
    mode: Mode | str | None = None,
    prefix: Path | str | None = None,
    etc: Path | str | None = None,
    share: Path | str | None = None,
    libexec: Path | str | None = None,
    state: Path | str | None = None,
    cache: Path | str | None = None,
    log: Path | str | None = None,
    run: Path | str | None = None,
    tmp: Path | str | None = None,
    environ: Mapping[str, str] | None = None,
  ) -> Layout:
    """Resolve a complete layout with CLI values taking precedence.

    Resolution order for each root is explicit argument, ``ISLAND_*``
    environment value, then the selected mode's default. Relative explicit or
    environment paths are rejected so launch behavior cannot depend on CWD.

    Args:
      mode: ``source``, ``portable``, or ``system``. Defaults to
        ``ISLAND_MODE`` and then ``source`` in a checkout.
      prefix: Immutable installation prefix or checkout root.
      etc: Product configuration root containing component namespaces.
      share: Architecture-independent resource root.
      libexec: Private executable root.
      state: Durable state root containing component namespaces.
      cache: Rebuildable cache root containing component namespaces.
      log: Diagnostic log root containing component namespaces.
      run: Boot- or process-lifetime runtime root.
      tmp: Disposable scratch root.
      environ: Environment mapping, injectable for deterministic tests.

    Raises:
      ValueError: If a mode is unsupported or an override is not absolute.
    """
    env = os.environ if environ is None else environ
    checkout = _checkout_root()
    selected_value = mode or env.get("ISLAND_MODE")
    selected = (
      Mode(selected_value)
      if selected_value is not None
      else Mode.SOURCE
      if (checkout / "pyproject.toml").is_file()
      else Mode.SYSTEM
    )

    prefix_value = _pick("prefix", prefix, env.get("ISLAND_PREFIX"))
    if prefix_value is None:
      prefix_path = Path("/usr") if selected is Mode.SYSTEM else checkout
    else:
      prefix_path = _absolute("prefix", prefix_value)

    defaults = _defaults(selected, prefix_path)
    values = {
      "etc": (etc, env.get("ISLAND_ETC")),
      "share": (share, env.get("ISLAND_SHARE")),
      "libexec": (libexec, env.get("ISLAND_LIBEXEC")),
      "state": (state, env.get("ISLAND_STATE")),
      "cache": (cache, env.get("ISLAND_CACHE")),
      "log": (log, env.get("ISLAND_LOG")),
      "run": (run, env.get("ISLAND_RUN")),
      "tmp": (tmp, env.get("ISLAND_TMP")),
    }
    resolved = {
      name: _resolve_root(name, explicit, inherited, defaults[name])
      for name, (explicit, inherited) in values.items()
    }
    return cls(
      mode=selected,
      prefix=prefix_path.resolve(),
      src=(prefix_path / "src").resolve() if selected is Mode.SOURCE else None,
      **resolved,
    )


def _checkout_root() -> Path:
  """Return the source checkout containing this module without using CWD."""
  return Path(__file__).resolve().parents[2]


def _defaults(mode: Mode, prefix: Path) -> dict[str, Path]:
  """Build the mode-specific default root map."""
  if mode is Mode.SYSTEM:
    return {
      "etc": Path("/etc/island"),
      "share": prefix / "share/island",
      "libexec": prefix / "libexec/island",
      "state": Path("/var/lib/island"),
      "cache": Path("/var/lib/island"),
      "log": Path("/var/lib/island"),
      "run": Path("/run/island"),
      "tmp": Path("/var/lib/island"),
    }
  data_root = (
    prefix.parent / "data"
    if mode is Mode.SOURCE and _is_ei_checkout(prefix)
    else prefix / "data"
  )
  return {
    "etc": prefix / "etc",
    "share": prefix.parent / "share"
    if mode is Mode.SOURCE and _is_ei_checkout(prefix)
    else prefix / "share",
    "libexec": prefix / "libexec",
    "state": data_root,
    "cache": data_root,
    "log": data_root,
    "run": data_root,
    "tmp": data_root,
  }


def _is_ei_checkout(prefix: Path) -> bool:
  """Recognize EI inside this source workspace without changing other layouts."""
  return (
    (prefix / "pyproject.toml").is_file()
    and (prefix / "src/ei").is_dir()
    and (prefix.parent / "Cargo.toml").is_file()
  )


def _pick(
  _name: str, explicit: Path | str | None, inherited: str | None
) -> Path | str | None:
  """Select an explicit value before its environment fallback."""
  return explicit if explicit is not None else inherited


def _resolve_root(
  name: str,
  explicit: Path | str | None,
  inherited: str | None,
  default: Path,
) -> Path:
  """Resolve one root according to argument, environment, and default order."""
  selected = _pick(name, explicit, inherited)
  return default.resolve() if selected is None else _absolute(name, selected)


def _absolute(name: str, value: Path | str) -> Path:
  """Normalize one caller-controlled root and reject CWD-relative values."""
  path = Path(value).expanduser()
  if not path.is_absolute():
    raise ValueError(f"{name} root must be absolute: {value}")
  return path.resolve()


def _component(component: str) -> str:
  """Validate a brief component namespace used below role roots."""
  if component not in {"ei", "eos", "isos"}:
    raise ValueError(f"unknown Island component: {component}")
  return component
