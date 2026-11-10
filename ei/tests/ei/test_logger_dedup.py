"""Repeat suppression keeps the one EI core JSONL stream bounded."""

from __future__ import annotations

import json
import logging
import time
from pathlib import Path

import pytest

from ei.lib import logger as logger_mod
from ei.lib.config import LoggingConfig
from ei.lib.logger import configure_logging, get_logger


@pytest.fixture()
def log_dir(tmp_path: Path):
  root = logging.getLogger()
  saved_handlers, saved_level = root.handlers[:], root.level
  logger_mod._configured = False
  logger_mod._console_handler = None
  cfg = LoggingConfig()
  cfg = cfg.model_copy(
    update={
      "dedup": True,
      "dedup_window_s": 0.05,
      "console": cfg.console.model_copy(update={"enabled": False}),
    }
  )
  configure_logging(tmp_path / "log", cfg, replay_dir=tmp_path / "state")
  yield tmp_path / "log"
  for handler in root.handlers[:]:
    handler.close()
    root.removeHandler(handler)
  root.handlers, root.level = saved_handlers, saved_level
  logger_mod._configured = False


def _events(path: Path) -> list[dict[str, object]]:
  return [json.loads(line) for line in (path / "ei.jsonl").read_text().splitlines()]


def test_repeats_are_suppressed_and_summarized(log_dir: Path) -> None:
  log = get_logger("ei.core.test")
  for _ in range(5):
    log.warning("queue_full")
  assert len(_events(log_dir)) == 1
  time.sleep(0.07)
  log.warning("queue_full")
  events = _events(log_dir)
  assert len(events) == 2
  assert events[1]["fields"]["repeats"] == 5


def test_distinct_events_remain_visible(log_dir: Path) -> None:
  log = get_logger("ei.core.test")
  log.warning("queue_full")
  log.warning("link_down")
  log.error("queue_full")
  assert [(e["level"], e["event_code"]) for e in _events(log_dir)] == [
    ("warn", "queue_full"),
    ("warn", "link_down"),
    ("error", "queue_full"),
  ]
