"""EI runtime core. Load process composition only when an entrypoint needs it."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
  from ei.core.system.composition import launch, launch_empty

__all__ = ["launch", "launch_empty"]


def __getattr__(name: str) -> Any:
  if name not in __all__:
    raise AttributeError(name)
  from ei.core.system import composition

  return getattr(composition, name)
