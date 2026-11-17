"""Compatibility entrypoint for the IsOS configuration adapter."""

from __future__ import annotations

if __name__ == "__main__":
  from ei.core.system.setup import main

  raise SystemExit(main())

# Keep the imported module identical so existing public and private helpers,
# including monkeypatch targets, resolve to their canonical implementation.
else:
  import sys

  from ei.core.system import setup as _setup

  sys.modules[__name__] = _setup
