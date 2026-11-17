# IsOS UI button group

<!-- doc-navigation:start -->
<details>
<summary>Folder: isos/interfaces/node/ui</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [IsOS UI monitor group](monitor.md)
  - [IsOS UI Vision group](vision.md)

</details>
<!-- doc-navigation:end -->

[中文](../../../../../docs_cn/isos/interfaces/node/ui/button_cn.md) · [Node index](../README.md) · [Client event request](../../api/client.md#button)

`isos.ui.button.trigger@v0.2` is an IsOS-owned input node at `isos/ui/button/<id>`. Its `activation` output carries `button_activation@v0.2` as an event. The payload has a fresh `event_id` and `kind:"activate"`; it contains no movement target. The node has no settings. An authored edge must lead through the declared EI safety path and EOS dispatcher route before movement can be admitted.

This is the UI→EI graph port. Its private delivery is the run-fenced IsOS-to-EI Unix socket event. The separate UI→EOS operator control uses the typed primitive request to the EOS loopback bridge and EOS Dispatcher admission. That request is a control endpoint associated with a selected primitive binding, not an executable graph edge or an alternative output of this button.

IsOS accepts an activation only for the active run and exact saved revision. A duplicate event ID returns its recorded status without repeating movement. The client reads the eventual result separately using the same event ID. Invalid paths, stale identities, and missing routes fail before owner delivery. The [event API](../../api/client.md#button) owns the exact request, response, and error codes.
