"""Product-level lifecycle and path utilities for Island-ei.

Keep this namespace small: component behavior belongs to :mod:`ei` or
the independently built EOS runtime, never here.
"""

from island.path import Layout, Mode

__all__ = ["Layout", "Mode"]
