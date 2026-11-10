"""dotenv loader — file as fallback, environment as authority."""

from __future__ import annotations

from pathlib import Path

from ei.lib.dotenv import load_env_file
from ei.main import load_dotenv
from island.path import Layout


def _write(tmp_path: Path, body: str) -> Path:
  env = tmp_path / ".env"
  env.write_text(body, encoding="utf-8")
  return env


def test_parses_keys_comments_quotes_and_export(tmp_path, monkeypatch):
  for key in ("MOONSHOT_API_KEY", "EI_MODEL_API_KEY", "EMPTY"):
    monkeypatch.delenv(key, raising=False)
  env = _write(
    tmp_path,
    "# comment\n"
    "MOONSHOT_API_KEY=sk-test123\n"
    'export EI_MODEL_API_KEY="quoted value"\n'
    "BAD LINE\n"
    "EMPTY=\n",
  )
  loaded = load_env_file(env)
  assert sorted(loaded) == ["EI_MODEL_API_KEY", "EMPTY", "MOONSHOT_API_KEY"]
  import os

  assert os.environ["MOONSHOT_API_KEY"] == "sk-test123"
  assert os.environ["EI_MODEL_API_KEY"] == "quoted value"
  assert os.environ["EMPTY"] == ""


def test_environment_wins_over_file(tmp_path, monkeypatch):
  monkeypatch.setenv("MOONSHOT_API_KEY", "sk-from-env")
  env = _write(tmp_path, "MOONSHOT_API_KEY=sk-from-file\n")
  assert load_env_file(env) == []
  import os

  assert os.environ["MOONSHOT_API_KEY"] == "sk-from-env"


def test_missing_file_loads_nothing(tmp_path):
  assert load_env_file(tmp_path / "absent.env") == []


def test_load_dotenv_returns_names_without_writing_raw_stderr(
  tmp_path, monkeypatch, capsys
) -> None:
  layout = Layout.resolve(prefix=tmp_path, mode="source", environ={})
  etc_root = tmp_path / "etc/ei"

  monkeypatch.setattr("ei.main.resolve_etc_root", lambda *_args, **_kwargs: etc_root)
  monkeypatch.setattr(
    "ei.lib.dotenv.load_env_file",
    lambda path: ["MODEL_API_KEY"] if path == layout.prefix / ".env" else [],
  )

  assert load_dotenv(None, layout) == [(layout.prefix / ".env", ["MODEL_API_KEY"])]
  assert capsys.readouterr().err == ""


def test_source_ei_checkout_reads_repository_root_only(tmp_path, monkeypatch) -> None:
  (tmp_path / "Cargo.toml").write_text("[workspace]\n")
  ei_root = tmp_path / "ei"
  (ei_root / "src/ei").mkdir(parents=True)
  (ei_root / "pyproject.toml").write_text("[project]\nname='ei'\n")
  layout = Layout.resolve(prefix=ei_root, mode="source", environ={})
  seen = []
  monkeypatch.setattr(
    "ei.lib.dotenv.load_env_file", lambda path: seen.append(path) or []
  )

  assert load_dotenv(None, layout) == []
  assert seen == [tmp_path / ".env"]
