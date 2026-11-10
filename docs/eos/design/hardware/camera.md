# Camera adapter node

[中文](../../../../docs_cn/eos/design/hardware/camera_cn.md) · [EOS architecture](../architecture.md) · [hardware](../../interfaces/hardware.md) · [vision](../../interfaces/vision.md) · [mujoco](../../interfaces/mujoco.md)

**Placement:** `isos/eos_hardware/camera/<instance-id>` · **Type family:** `isos.eos_hardware.camera@v0.2`. The registered type is a reusable definition; the deployment chooses each instance ID and its own nonblank instance description.

**Status:** Registered for graph format `0.2.0`.

## Purpose and boundary

One configured camera owns one adapter and produces typed frames, health, and lifecycle status. RGB is required when advertised; aligned depth is optional and must be reported unavailable when absent.

## Ports and settings

The owning interface contract defines the exact ports, setting types, units, defaults, and validation for this registered type.

## Lifecycle and algorithm

Validate the node profile and adapter family, construct exactly one device adapter, sample into the bounded latest-value hub, then publish only validated fresh frames. A bad frame degrades this camera alone; it never fabricates a replacement frame. Node state, rejection reason, and correlation IDs appear in the shared deployment log.
