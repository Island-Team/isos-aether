# Shared control dispatcher node

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos/core/dispatcher</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [dispatcher/arm group](arm.md)
  - [Base primitive binding node](base.md)
  - [V2 dispatcher tree](dispatcher.md)
  - [Manipulator primitive binding node](manipulator.md)
  - [Routed connection scheduling](scheduler.md)

</details>
<!-- doc-navigation:end -->

[中文](../../../../docs_cn/eos/core/dispatcher/control_cn.md)


<a id="shared-control-dispatcher-node"></a>

## Shared control dispatcher node

**Placement:** `isos/eos_dispatcher/control/<instance-id>` · **Type family:** `isos.eos_dispatcher.control@v0.2`. The registered type is a reusable definition; the deployment chooses each instance ID and its own nonblank instance description.

**Status:** Historical `isos.eos_*` design reference. This type is not a current built-in EOS node-library entry.

### Purpose and boundary

The single actuator admission owner checks identity, health, freshness, deadlines, limits and safety before any backend release. A graph node represents this owner and its typed route; it does not create a second dispatcher.

### Ports and settings

The owning interface contract defines the exact ports, setting types, units, defaults, and validation for this registered type.

### Lifecycle and algorithm

Normalize a request, reject invalid or stale work, reserve command identity, record admission, release once, then observe or stop. Priority on a routed input only schedules delivery; it never overrides these checks. Node state, rejection reason, and correlation IDs appear in the shared deployment log.
