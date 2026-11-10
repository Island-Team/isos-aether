"""Resource-root ownership checks for the source and installed layouts."""

from __future__ import annotations

from dataclasses import replace
from pathlib import Path

import pytest

from island.cfg import ConfigError, load
from island.path import Layout, Mode
from island.share import (
  SharedResources,
  configured_resources,
  repository_resources,
  require_eos_contracts,
)


def test_source_checkout_has_one_resource_root() -> None:
  root = Path(__file__).resolve().parents[2]
  resources = repository_resources(root)

  assert resources.roots == (root.parent / "share",)
  assert (
    require_eos_contracts(resources)[0]
    == root.parent / "share/eos/contracts/eos/gateway.proto"
  )
  assert resources.require("ei/defaults/ei/models.toml").is_file()
  assert resources.require("isos/defaults/island.toml").is_file()


def test_installed_ei_resolves_bundled_product_config_and_resources(
  tmp_path: Path,
) -> None:
  """Match the product install tree when no system product override exists."""
  source = Path(__file__).resolve().parents[2]
  ei_prefix = tmp_path / "share/isos/ei"
  bundled = ei_prefix / "etc/island.toml"
  bundled.parent.mkdir(parents=True)
  bundled.write_text(
    (source.parent / "config/ei/island.toml")
    .read_text()
    .replace('share = ["../share"]', 'share = ["share", ".."]')
  )
  defaults = ei_prefix / "share/defaults/island.toml"
  defaults.parent.mkdir(parents=True)
  defaults.write_bytes((source.parent / "share/isos/defaults/island.toml").read_bytes())
  ei_default = ei_prefix / "share/defaults/ei/ei.toml"
  ei_default.parent.mkdir(parents=True)
  ei_default.write_bytes((source.parent / "config/ei/ei.toml").read_bytes())
  eos_contract = tmp_path / "share/isos/eos/contracts/eos/gateway.proto"
  eos_contract.parent.mkdir(parents=True)
  eos_contract.write_text('syntax = "proto3";\n')
  layout = Layout.resolve(
    mode=Mode.SYSTEM,
    prefix=ei_prefix,
    etc=tmp_path / "no-system-overlay",
    environ={},
  )

  resources = configured_resources(layout)

  assert resources.roots == (ei_prefix / "share", tmp_path / "share/isos")
  assert resources.require("defaults/ei/ei.toml") == ei_default
  assert resources.require("eos/contracts/eos/gateway.proto") == eos_contract

  system_config = tmp_path / "no-system-overlay/island.toml"
  system_config.parent.mkdir(parents=True)
  system_config.write_text(
    bundled.read_text().replace('share = ["share", ".."]', 'share = ["share"]')
  )
  assert configured_resources(layout).roots == (ei_prefix / "share",)
  assert configured_resources(layout, system_config).roots == (ei_prefix / "share",)


def test_missing_resource_root_rejects(tmp_path: Path) -> None:
  root = Path(__file__).resolve().parents[2]
  config = load(
    root.parent / "config/ei/island.toml",
    defaults=root.parent / "share/isos/defaults/island.toml",
  )
  layout = Layout.resolve(mode=Mode.PORTABLE, prefix=tmp_path, environ={})

  with pytest.raises(ConfigError, match="shared resource root not found"):
    SharedResources.resolve(config, layout)


def test_duplicate_logical_resource_rejects(tmp_path: Path) -> None:
  root = Path(__file__).resolve().parents[2]
  config = load(
    root.parent / "config/ei/island.toml",
    defaults=root.parent / "share/isos/defaults/island.toml",
  )
  for directory in ("first", "second"):
    file = tmp_path / directory / "eos/contracts/eos/gateway.proto"
    file.parent.mkdir(parents=True)
    file.write_text("duplicate")
  layout = Layout.resolve(mode=Mode.PORTABLE, prefix=tmp_path, environ={})

  with pytest.raises(ConfigError, match="duplicate shared resource"):
    SharedResources.resolve(replace(config, share=("first", "second")), layout)
