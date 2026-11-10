# Manipulator adapter node

[中文](../../../../docs_cn/eos/design/hardware/manipulator_cn.md) · [EOS architecture](../architecture.md) · [hardware](../../interfaces/hardware.md) · [primitive-bindings](../../interfaces/primitive-bindings.md) · [mujoco](../../interfaces/mujoco.md)

**Placement:** `isos/eos_hardware/manipulator/<instance-id>` · **Type family:** `isos.eos_hardware.manipulator@v0.2`. The registered type is a reusable definition; the deployment chooses each instance ID and its own nonblank instance description.

**Status:** Registered for graph format `0.2.0`.

## Purpose and boundary

One arm owns ordered joint state, optional tool pose, and its physical stop seam. Node profile joint order is authoritative.

## Ports and settings

The owning interface contract defines the exact ports, setting types, units, defaults, and validation for this registered type.

## Lifecycle and algorithm

Validate joint names, order, finite targets, mode and bounds before Dispatcher admission. The adapter reports fresh observed state; an acknowledgment alone does not prove arrival. Stop or uncertain outcome cannot be hidden by a graph edge. Node state, rejection reason, and correlation IDs appear in the shared deployment log.
