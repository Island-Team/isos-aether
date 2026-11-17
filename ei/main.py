"""Stable ``python -m ei.main`` launch entrypoint."""

from __future__ import annotations

import sys

if __name__ == "__main__":
  from ei.core.system.launch import main

  raise SystemExit(main())

from ei.core.system import launch as _launch

sys.modules[__name__] = _launch
