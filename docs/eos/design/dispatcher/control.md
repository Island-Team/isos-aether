# Shared control dispatcher node

[中文](../../../../docs_cn/eos/design/dispatcher/control_cn.md) · [EOS architecture](../architecture.md) · [control](../../interfaces/control.md) · [primitive-bindings](../../interfaces/primitive-bindings.md)

**Placement:** `isos/eos_dispatcher/control/<instance-id>` · **Type family:** `isos.eos_dispatcher.control@v0.2`. The registered type is a reusable definition; the deployment chooses each instance ID and its own nonblank instance description.

**Status:** Registered for graph format `0.2.0`.

## Purpose and boundary

The single actuator admission owner checks identity, health, freshness, deadlines, limits and safety before any backend release. A graph node represents this owner and its typed route; it does not create a second dispatcher.

## Ports and settings

The owning interface contract defines the exact ports, setting types, units, defaults, and validation for this registered type.

## Lifecycle and algorithm

Normalize a request, reject invalid or stale work, reserve command identity, record admission, release once, then observe or stop. Priority on a routed input only schedules delivery; it never overrides these checks. Node state, rejection reason, and correlation IDs appear in the shared deployment log.
