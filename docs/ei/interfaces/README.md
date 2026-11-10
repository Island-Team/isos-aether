# EI external interfaces

[中文](../../../docs_cn/ei/interfaces/README.md) · [EI architecture](../design/architecture.md)

This directory describes what other owners and clients can consume. EI implementation and algorithms live under [design](../design/architecture.md).

| Contract | Consumer and boundary |
| --- | --- |
| [Setup](setup.md) | IsOS `ei-config` one-shot and owner socket exchange. |
| [Runtime](runtime.md) | Operator-facing EI runtime WebSocket API. |
| [Node](node.md) | IsOS graph node references and EI–EOS route ownership. |
| [EOS operations](eos.md) | EI–EOS discovery, operations, cancellation, and compatibility. |
| [Events](events.md), [safety reset](safety-reset.md), [vision evidence](vision-evidence.md), [reader features](reader-features.md) | External event and specialized owner exchanges. |
