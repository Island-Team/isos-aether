# Base primitive binding node

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos/core/dispatcher</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [dispatcher/arm group](arm.md)
  - [Shared control dispatcher node](control.md)
  - [V2 dispatcher tree](dispatcher.md)
  - [Manipulator primitive binding node](manipulator.md)
  - [Routed connection scheduling](scheduler.md)

</details>
<!-- doc-navigation:end -->

[中文](../../../../docs_cn/eos/core/dispatcher/base_cn.md)


<a id="base-primitive-binding-node"></a>

## Base primitive binding node

**Placement:** `isos/eos_dispatcher/base/<instance-id>` · **Type family:** `isos.eos_dispatcher.primitive_base@v0.2`. The registered type is a reusable definition; the deployment chooses each instance ID and its own nonblank instance description.

**Status:** Historical `isos.eos_*` design reference. This type is not a current built-in EOS node-library entry.

### Purpose and boundary

The node names one versioned base operation and one graph edge to a compatible base device; it owns no adapter and no admission bypass.

In the IsosUI EOS panel, this **active instance** appears as one selectable primitive control. The panel shows its type and instance descriptions, single target, declared movement mode and EOS live health; an invocation uses the existing `primitive_typed` bridge and displays the correlated result. The graph edge is a binding, not an executable call.

### Ports and settings

The owning interface contract defines the exact ports, setting types, units, defaults, and validation for this registered type.

### Lifecycle and algorithm

Resolve exactly one compatible device from the explicit edge, compare name/version/mode with the admitted primitive catalog, then pass a typed command header through Dispatcher. Reject ambiguity before release. Node state, rejection reason, and correlation IDs appear in the shared deployment log.
