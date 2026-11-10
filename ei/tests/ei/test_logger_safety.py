"""The EI core writes one safe local JSONL diagnostic stream."""

from __future__ import annotations

import json
import logging
from pathlib import Path

import pytest
from pydantic import ValidationError

from ei.lib import logger as logger_mod
from ei.lib.config import LoggingConfig
from ei.lib.logger import bind_log_context, configure_logging, get_logger


@pytest.fixture()
def log_dir(tmp_path: Path):
  root = logging.getLogger()
  saved_handlers, saved_level = root.handlers[:], root.level
  logger_mod._configured = False
  logger_mod._console_handler = None
  cfg = LoggingConfig()
  cfg = cfg.model_copy(
    update={"console": cfg.console.model_copy(update={"enabled": False})}
  )
  configure_logging(tmp_path / "log", cfg, replay_dir=tmp_path / "state")
  yield tmp_path / "log"
  for handler in root.handlers[:]:
    handler.close()
    root.removeHandler(handler)
  root.handlers, root.level = saved_handlers, saved_level
  logger_mod._configured = False
  logger_mod._log_context.set(None)


def _events(path: Path) -> list[dict[str, object]]:
  return [json.loads(line) for line in (path / "ei.jsonl").read_text().splitlines()]


def test_unknown_logging_fields_are_rejected() -> None:
  with pytest.raises(ValidationError):
    LoggingConfig.model_validate({"unexpected": True})


def test_one_core_jsonl_has_common_envelope_and_context(log_dir: Path) -> None:
  with bind_log_context(mission_id="mission-1"):
    get_logger("ei.core.test").warning("health_probe", detail="ready")
  event = _events(log_dir)[0]
  assert {"timestamp", "level", "source", "message"} <= event.keys()
  assert event["level"] == "warn"
  assert event["source"] == "ei.core.test.py@aether-v0.2.1"
  assert event["message"] == "health probe"
  assert event["fields"]["detail"] == "ready"
  assert event["fields"]["mission_id"] == "mission-1"
  assert not (log_dir / "events.jsonl").exists()
  assert not (log_dir / "main.log").exists()
  assert not (log_dir.parent / "state" / "ei.jsonl").exists()


def test_sensitive_values_are_redacted_before_local_write(log_dir: Path) -> None:
  get_logger("ei.main").info(
    "provider_request", prompt="private prompt", api_key="private key", detail="safe"
  )
  event = _events(log_dir)[0]
  assert event["fields"]["prompt"] == "<excluded>"
  assert event["fields"]["api_key"] == "<redacted>"
  assert event["fields"]["detail"] == "safe"


def test_third_party_record_uses_same_jsonl(log_dir: Path) -> None:
  logging.getLogger("websockets.server").info("listening on %s", "unix:///tmp/eos.sock")
  event = _events(log_dir)[0]
  assert event["source"].startswith("ei.websockets.server.py@")
