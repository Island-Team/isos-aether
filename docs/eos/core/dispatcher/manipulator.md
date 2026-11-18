# Manipulator primitive binding node

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos/core/dispatcher</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [dispatcher/arm group](arm.md)
  - [Base primitive binding node](base.md)
  - [Shared control dispatcher node](control.md)
  - [V2 dispatcher tree](dispatcher.md)
  - [Routed connection scheduling](scheduler.md)

</details>
<!-- doc-navigation:end -->

[中文](../../../../docs_cn/eos/core/dispatcher/manipulator_cn.md)


<a id="manipulator-primitive-binding-node"></a>

## Manipulator primitive binding node

**Placement:** `isos/eos_dispatcher/manipulator/<instance-id>` · **Type family:** `isos.eos_dispatcher.primitive_manipulator@v0.2`. The registered type is a reusable definition; the deployment chooses each instance ID and its own nonblank instance description.

**Status:** Historical `isos.eos_*` design reference. This type is not a current built-in EOS node-library entry.

### Purpose and boundary

The node names one versioned arm operation and one explicit edge to a compatible manipulator; the hardware adapter still owns the device.

In the IsosUI EOS panel, this **active instance** appears as one selectable primitive control. The panel shows its type and instance descriptions, single target, declared movement mode and EOS live health; an invocation uses the existing `primitive_typed` bridge and displays the correlated result. The graph edge is a binding, not an executable call.

### Ports and settings

The owning interface contract defines the exact ports, setting types, units, defaults, and validation for this registered type.

### Lifecycle and algorithm

Validate the binding catalog, edge target, joint order and typed command header, then ask Dispatcher to admit. A failed or uncertain arm motion stops via the owning adapter. Node state, rejection reason, and correlation IDs appear in the shared deployment log.
