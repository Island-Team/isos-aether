# IsOS node contract source

[Node interfaces](../../../../docs/isos/interfaces/node/README.md) · [节点接口](../../../../docs_cn/isos/interfaces/node/README_cn.md)

These authored JSON Schemas describe registered node type declarations and saved node instances for graph format `0.2.0`. They retain existing optional fields while defining the strict shape of `implementation`, `service_inputs`, `service_outputs`, and `service_bindings`. IsOS validates graph admission; EOS, EI, and UI use the same identities in projections and editors. The owner of a payload or service keeps its wire IDL under `share/<owner>/contracts/`.

The first declared cross-core service is `island.eos.v1.GatewayService` from `share/eos/contracts/eos/gateway.proto`. EI is the client, EOS the provider, gRPC the protocol, and loopback TCP the carrier. A saved binding contains the service identity; IsOS resolves an endpoint only for the selected run. The provider node path is reserved in the schema and rejected by this first Apply slice. The schemas describe other protocol and carrier values, but a publisher must advertise and implement a combination before Apply can admit it. A socket protocol must identify its framing with `socket_protocol`.

Files:

- `node-type.schema.json`: registered type placement, implementation, ports, and service declarations.
- `node-instance.schema.json`: saved instance and service binding shape.

Compatibility: add optional fields without changing existing graph meaning; change an established wire identity or required field only with an explicit version change. Graph format, service version, package version, and source release version are independent.
