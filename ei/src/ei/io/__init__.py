"""EI northbound WebSocket interface — serves the wire spec in ``docs/ei/interfaces/runtime.md``.

``io`` is the WebUI boundary and nothing else: input from the operator, output
to the operator. Backend traffic belongs to ``core/link`` (southbound) and
``core/perception`` (ingress); this package only projects what they own. It
sits outside ``ei.core`` — ``ei.io`` may import core, core never imports
io — and is started by the composition root (``ei/core/__init__.py``); see
``docs/ei/design/runtime/io.md``.

Routes under ``/api/aether/ei/``: ``health`` (liveness plus gate verdict
counters), ``chat`` (translated hearing ingress), ``missions/stream``
(read-only mission projection),
``review`` (operator promotion of concluded plans into reference memory),
``context`` (the live prompt snapshot), ``prompts`` (the raw planner system
message), ``primitives`` and ``skills`` (discovered catalogs), and the
runtime-facing routes ``perception/world/state``, ``perception/world/detection``,
and ``perception/world/asr``, plus ``primitive/invoke`` for operator primitive
invocation. The package holds
no domain state of its own — it projects the shared :class:`ServerState`, the
perception stores, and per-part :class:`PartStore` exchanges that producers
elsewhere publish to.
"""

from ei.io.northbound import (
  BACKEND_PATH,
  CHAT_PATH,
  CONTEXT_PATH,
  HEALTH_PATH,
  MISSIONS_COMMAND_PATH,
  MISSIONS_STREAM_PATH,
  MODEL_IO_PATH,
  PART_PATHS,
  PERCEPTIONS_PATH,
  PRIMITIVE_INVOKE_PATH,
  PRIMITIVES_PATH,
  PROMPTS_PATH,
  REVIEW_PATH,
  SKILLS_COMMAND_PATH,
  SKILLS_EXECUTION_PATH,
  SKILLS_PATH,
  HealthServer,
  NorthboundServer,
)
from ei.io.state import ServerState
from ei.io.store import (
  ContextStore,
  PartStore,
  PrimitivesStore,
  PromptsStore,
  SkillsStore,
)

__all__ = [
  "BACKEND_PATH",
  "CHAT_PATH",
  "CONTEXT_PATH",
  "HEALTH_PATH",
  "MISSIONS_COMMAND_PATH",
  "MISSIONS_STREAM_PATH",
  "MODEL_IO_PATH",
  "PART_PATHS",
  "PERCEPTIONS_PATH",
  "PRIMITIVES_PATH",
  "PRIMITIVE_INVOKE_PATH",
  "PROMPTS_PATH",
  "REVIEW_PATH",
  "SKILLS_COMMAND_PATH",
  "SKILLS_EXECUTION_PATH",
  "SKILLS_PATH",
  "ContextStore",
  "HealthServer",
  "NorthboundServer",
  "PartStore",
  "PrimitivesStore",
  "PromptsStore",
  "ServerState",
  "SkillsStore",
]
