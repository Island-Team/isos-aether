# Mobile base adapter node

[中文](../../../../docs_cn/eos/design/hardware/base_cn.md) · [EOS architecture](../architecture.md) · [hardware](../../interfaces/hardware.md) · [primitive-bindings](../../interfaces/primitive-bindings.md) · [mujoco](../../interfaces/mujoco.md)

**Placement:** `isos/eos_hardware/base/<instance-id>` · **Type family:** `isos.eos_hardware.base@v0.2`. The registered type is a reusable definition; the deployment chooses each instance ID and its own nonblank instance description.

**Status:** Registered for graph format `0.2.0`.

## Purpose and boundary

One base owns observations and the physical stop seam. An input edge describes the command route but cannot authorize direct actuation.

## Ports and settings

The owning interface contract defines the exact ports, setting types, units, defaults, and validation for this registered type.

## Lifecycle and algorithm

Resolve the verified adapter, validate command source and typed limits, ask Dispatcher to admit, then release to the one adapter. Lease expiry or stop calls the adapter stop seam; uncertain completion remains `OutcomeUnknown` and is never retried automatically. Node state, rejection reason, and correlation IDs appear in the shared deployment log.
