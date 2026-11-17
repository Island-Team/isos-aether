"""Resource-root ownership checks for the source and installed layouts."""

from __future__ import annotations

from dataclasses import replace
from pathlib import Path

import pytest
from ei.lib.layout import Layout, Mode
from ei.lib.product_config import ConfigError, load
from ei.lib.resources import (
  SharedResources,
  configured_resources,
  repository_resources,
  require_eos_contracts,
)


def test_source_checkout_has_one_resource_root() -> None:
  root = Path(__file__).resolve().parents[2]
  resources = repository_resources(root)

  assert resources.roots == (root.parent / "contract",)
  assert (
    require_eos_contracts(resources)[0]
    == root.parent / "contract/proto/gateway.proto"
  )
  assert (root.parent / "config/defaults/ei/models.toml").is_file()
  assert (root.parent / "config/defaults/island.toml").is_file()


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
  )
  defaults = ei_prefix / "share/defaults/island.toml"
  defaults.parent.mkdir(parents=True)
  defaults.write_bytes((source.parent / "config/defaults/island.toml").read_bytes())
  ei_default = ei_prefix / "share/defaults/ei/ei.toml"
  ei_default.parent.mkdir(parents=True)
  ei_default.write_bytes((source.parent / "config/ei/ei.toml").read_bytes())
  eos_contract = tmp_path / "share/isos/contract/proto/gateway.proto"
  eos_contract.parent.mkdir(parents=True)
  eos_contract.write_text('syntax = "proto3";\n')
  layout = Layout.resolve(
    mode=Mode.SYSTEM,
    prefix=ei_prefix,
    etc=tmp_path / "no-system-overlay",
    environ={},
  )

  resources = configured_resources(layout)

  assert layout.contracts == tmp_path / "share/isos/contract"
  assert resources.roots == (tmp_path / "share/isos/contract",)
  assert ei_default.is_file()
  assert resources.require("proto/gateway.proto") == eos_contract

  system_config = tmp_path / "no-system-overlay/island.toml"
  system_config.parent.mkdir(parents=True)
  system_config.write_text(
    bundled.read_text().replace('share = ["../contract"]', 'share = ["missing"]')
  )
  with pytest.raises(ConfigError, match="shared resource root not found"):
    configured_resources(layout)
  assert configured_resources(layout, bundled).roots == (tmp_path / "share/isos/contract",)


def test_missing_resource_root_rejects(tmp_path: Path) -> None:
  root = Path(__file__).resolve().parents[2]
  config = load(
    root.parent / "config/ei/island.toml",
    defaults=root.parent / "config/defaults/island.toml",
  )
  layout = Layout.resolve(mode=Mode.PORTABLE, prefix=tmp_path, environ={})

  with pytest.raises(ConfigError, match="shared resource root not found"):
    SharedResources.resolve(config, layout)


def test_duplicate_logical_resource_rejects(tmp_path: Path) -> None:
  root = Path(__file__).resolve().parents[2]
  config = load(
    root.parent / "config/ei/island.toml",
    defaults=root.parent / "config/defaults/island.toml",
  )
  for directory in ("first", "second"):
    file = tmp_path / directory / "proto/gateway.proto"
    file.parent.mkdir(parents=True)
    file.write_text("duplicate")
  layout = Layout.resolve(mode=Mode.PORTABLE, prefix=tmp_path, environ={})

  with pytest.raises(ConfigError, match="duplicate shared resource"):
    SharedResources.resolve(replace(config, share=("first", "second")), layout)
