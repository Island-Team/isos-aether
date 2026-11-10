# External process interfaces

[Interface guide](../README.md) · [中文](../../../../docs_cn/eos/interfaces/external/README_cn.md)

These pages define EOS communication with external processes. Transport details do not define a spatial namespace.

- [HTTP/SSE adapter](http.md): WebUI entry point; the adapter calls EOS gRPC services.
- [Model providers](models.md): EOS outbound capability calls over HTTP, WebSocket, or configured gRPC; private schemas live in `share/eos/contracts/eos/external/models/schemas/`.
- [Simulator process](simulator.md): process-side transport and lifecycle for the simulator boundary.

The skill mailbox is documented with native skill authoring, because it belongs to the skill runtime.
