"""Stable ``python -m ei.safety_cli`` entrypoint."""

from __future__ import annotations

import sys

if __name__ == "__main__":
  from ei.core.system.safety_cli import main

  raise SystemExit(main())

from ei.core.system import safety_cli as _safety_cli

sys.modules[__name__] = _safety_cli
