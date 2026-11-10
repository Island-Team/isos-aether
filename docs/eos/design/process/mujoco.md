# MuJoCo simulator process node

[中文](../../../../docs_cn/eos/design/process/mujoco_cn.md) · [EOS architecture](../architecture.md) · [node-contributions](../../interfaces/node-contributions.md) · [mujoco](../../interfaces/mujoco.md) · [hardware](../../interfaces/hardware.md)

**Placement:** `isos/eos_process/mujoco/<instance-id>` · **Type family:** `isos.eos_process.mujoco@v0.2`. The registered type is a reusable definition; the deployment chooses each instance ID and its own nonblank instance description.


## Purpose and boundary

IsOS verifies package `island/mujoco-sim@0.1.1`, resolves scene and executable, and applies the graph. EOS supervises the simulator as its direct child and owns the internal socket adapter.

## Ports and settings

The owning interface contract defines the exact ports, setting types, units, defaults, and validation for this registered type.

## Lifecycle and algorithm

Validate package and scene, spawn one child, wait for owner-only socket readiness, then admit declared device adapters. On stop, close admission and stop actuators before terminating and reaping the simulator. A failed stop remains visible. Node state, rejection reason, and correlation IDs appear in the shared deployment log.
