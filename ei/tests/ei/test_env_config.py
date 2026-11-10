"""EI settings are selected by TOML and explicit arguments."""

from __future__ import annotations

from ei.lib.config import (
  _config_doc,
  load_launch_config,
  load_mode_config,
  load_model_config,
)


def test_factory_then_overlay_toml_precedence(tmp_path, monkeypatch) -> None:
  defaults = tmp_path / "defaults"
  overlay = tmp_path / "overlay"
  defaults.mkdir()
  overlay.mkdir()
  (defaults / "ei.toml").write_text("[server]\nport=16060\n[launch]\nbackend=false\n")
  (overlay / "ei.toml").write_text("[server]\nport=17171\n")
  monkeypatch.setenv("EI_SERVER_PORT", "18181")
  monkeypatch.setenv("EI_LAUNCH_BACKEND", "true")
  merged = _config_doc(overlay, "ei.toml", defaults)
  assert merged["server"]["port"] == 17171
  assert merged["launch"]["backend"] is False


def test_runtime_settings_ignore_legacy_environment(etc_root, monkeypatch) -> None:
  baseline = load_launch_config(etc=str(etc_root), mode="sandbox")
  monkeypatch.setenv("EI_SERVER_PORT", "0")
  monkeypatch.setenv("EI_LOG_LEVEL", "verbose")
  monkeypatch.setenv("EI_DECISION_MISSION_CAPACITY", "invalid")
  config = load_launch_config(etc=str(etc_root), mode="sandbox")
  assert config.server.port == baseline.server.port
  assert config.logging.level == baseline.logging.level
  assert config.decision.mission_capacity == baseline.decision.mission_capacity


def test_explicit_mode_and_model_names_keep_priority(etc_root, monkeypatch) -> None:
  monkeypatch.setenv("EI_MODE", "missing")
  assert load_mode_config(str(etc_root), mode="sandbox").mode == "sandbox"
  monkeypatch.setenv("EI_MODEL", "missing")
  assert load_model_config(str(etc_root), name="gemma4-fast").model == "Gemma4-26B"
