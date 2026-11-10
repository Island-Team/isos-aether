"""Cognitive safety gate — sole egress authority over primitive calls."""

from ei.core.safety.audit import safety_audit_entry
from ei.core.safety.gate import SafetyGate

__all__ = ["SafetyGate", "safety_audit_entry"]
