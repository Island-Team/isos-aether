# Graph edges

[中文](../../../../docs_cn/isos/interfaces/node/edges_cn.md) · [Ports](ports.md)

An input reference has the form `{"node":"core/layer/group/node","output":"out"}`. The destination stores it under its declared input name. IsOS checks that both instances exist at registered placements, that the ports match, and that the graph has no cycle. A cross-core `topology_link` is admitted when both ends declare `transport: "none"` and `delivery_mode: "declaration"`; it needs no runtime bridge because no payload crosses the boundary.

A live edge additionally requires matching transport, delivery mode, exact registered payload ID, and the same protocol `contract` on sender and receiver ports. The trusted owner catalog must contain a `port_routes` entry naming the exact source type and output port, target type and input port, transport, delivery mode, payload ID, and protocol contract. For cross-core Unix socket bridges, both endpoint profiles must also declare the bridge port and have owner bridge adapters. IsOS rejects an edge without this route proof. The validation response names the exact `source`, `output`, `target`, and `input` so clients can identify the admitted connection. Static `topology_link` edges remain declarations and carry no payload.

The starter graph has seven edges. A button activation is passed through three EI nodes and four EOS nodes; EOS alone translates it into a six-joint delta and admits it to the mock arm.

The built-in catalog publishes only the exact seven live `port_routes` used by the demo. Other live edges require an owner implementation and matching route registration.
