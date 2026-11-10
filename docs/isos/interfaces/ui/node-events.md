# Button activation event

[中文](../../../../docs_cn/isos/interfaces/ui/node-events_cn.md) · [Node types](../node/README.md)

IsosUI sends one general activation from a registered button node to IsOS with `POST /api/isos/v1/nodes/events`. IsOS verifies local Host and Origin, the active run and saved revision, and the exact declared path through EI and EOS before forwarding to the private EI owner. The browser cannot choose an arm, joint target, owner address, or actuator grant.

```json
{
  "schema_version": "0.2.0",
  "source_node": "isos/ui/button/trigger",
  "output": "activation",
  "data_type": "button_activation@v0.2",
  "run_id": "run-1",
  "active_revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "payload": { "event_id": "click-1", "kind": "activate" }
}
```

All fields are required; unknown fields are rejected. `source_node` has four ASCII path segments of at most 48 bytes each, using letters, digits, `_`, or `-`. `event_id` has 1–64 ASCII letters, digits, `_`, `-`, or `.`; `run_id` uses the same grammar and at most 128 bytes. The revision is 64 lowercase hexadecimal characters. The body is at most 2048 bytes. A distinct click uses a fresh event ID; an uncertain click retains its original ID.

The graph must have one admitted path from the button through EI input, cognition, and safety, then EOS ingress, skill, dispatcher, and mock arm. IsOS sends `{core:"ei",op:"activate",run_id,revision,source_node,event_id,kind}` on the EI owner socket. A successful submission returns the event ID, command ID, and `accepted` status after EOS reserves the command. A duplicate ID returns its recorded status without another movement. The browser does not wait for arm movement in the submission request.

`POST /api/isos/v1/nodes/events/result` reads the correlated result without resubmitting movement. Its exact request is `{schema_version:"0.2.0",source_node,run_id,active_revision,event_id}` with the same identities as submission. IsOS checks the active run, saved graph, and declared path before asking EI for the result; it rechecks the run before replying. The response includes `event_id`, `command_id`, and `status`: `accepted`, `running`, `succeeded`, `failed`, `cancelled`, or `outcome_unknown`. Only `succeeded` includes six finite `joint_positions_rad` values. IsosUI reads this route until a terminal status or a bounded wait ends; a later read uses the same event ID.

Invalid submission fields return `400 node_event_invalid`, an oversized body `413 node_event_oversized`, a stale run or graph `409 run_fence_stale` or `graph_fence_stale`, and an invalid route `422 node_route_invalid` or `node_edges_invalid`. Invalid result reads return `400 node_result_invalid`, `413 node_result_oversized`, `422 node_route_invalid`, or `503 owner_result_unavailable` or `owner_result_invalid` as appropriate. Owner transport failure or uncertain submission returns `503 owner_outcome_unknown`; query the result using the same event ID before considering another submission. Stop closes admission before worker exit.
