# IsOS design

[Architecture](architecture.md) is the current overview. Focused pages follow the Rust source boundaries:

- [Server and launcher](app/server.md) covers CLI admission, loopback server readiness, and manager handoff.
- [Package admission](domain/admission.md) follows verified exact pins into an owner launch.
- [Package manager](domain/packages.md) records the implemented package path and explicitly labeled target work.
- [Manager services](runtime/services.md) covers local owner configuration.
- [Presentation selection](ui/presentation.md) covers output ownership and current unavailability.
- [Registered node families](node/README.md) maps process and UI consumers.
- [Control path](control-path.md) traces UI key input through EOS admission.
- [Verification](verification.md) records commands and evidence; [remaining work](remaining-work.md) lists open gates.

Public fields are owned by [interfaces](../interfaces/README.md).

[中文](../../../docs_cn/isos/design/README_cn.md)
