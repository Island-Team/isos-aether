"""Memory stores and lazy vector recall with separate write paths."""

from __future__ import annotations

from typing import TYPE_CHECKING

from ei.core.cognition.memory.consolidate import memory_consolidation_entry
from ei.core.cognition.memory.docs import index_docs_corpus, load_docs_corpus
from ei.core.cognition.memory.local import LocalMemoryStore
from ei.core.cognition.memory.recall import RecallService
from ei.core.cognition.memory.reference import ReferenceStore
from ei.core.cognition.memory.store import EpisodicStore
from ei.core.cognition.memory.working import WorkingStore

if TYPE_CHECKING:
  from ei.core.cognition.memory.vec import SqliteVecIndex

__all__ = [
  "EpisodicStore",
  "LocalMemoryStore",
  "RecallService",
  "ReferenceStore",
  "SqliteVecIndex",
  "WorkingStore",
  "index_docs_corpus",
  "load_docs_corpus",
  "memory_consolidation_entry",
]


def __getattr__(name: str) -> object:
  if name == "SqliteVecIndex":
    from ei.core.cognition.memory.vec import SqliteVecIndex

    return SqliteVecIndex
  raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
