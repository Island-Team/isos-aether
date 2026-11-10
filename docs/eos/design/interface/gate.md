# Camera preview gate node

[中文](../../../../docs_cn/eos/design/interface/gate_cn.md) · [EOS architecture](../architecture.md) · [vision](../../interfaces/vision.md) · [hardware](../../interfaces/hardware.md)

**Placement:** `isos/eos_interface/gate/<instance-id>` · **Type family:** `isos.eos_interface.camera_gate@v0.2`. The registered type is a reusable definition; the deployment chooses each instance ID and its own nonblank instance description.

**Status:** Registered for graph format `0.2.0`.

## Purpose and boundary

Transforms an admitted camera frame into a bounded read-only preview for IsosUI Vision. It cannot send camera control or actuator commands.

## Ports and settings

The owning interface contract defines the exact ports, setting types, units, defaults, and validation for this registered type.

## Lifecycle and algorithm

Bind one compatible camera source, validate and bound frame dimensions/age, encode or forward the latest permitted preview, and drop stale frames under backpressure. Viewer disconnect leaves camera acquisition and safety unaffected. Node state, rejection reason, and correlation IDs appear in the shared deployment log.
