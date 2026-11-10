# Local HTTP routes

This folder is for IsOS server and local-client implementers. The routes begin at `/api/isos/v1/`; a request passes local Host/Origin admission and is decoded by `app`, while graph, package, and runtime owners make the decision. The [external node-event request](../ui/node-events.md) is documented with the node contract.

- [Envelope](envelope.md) defines loopback request admission and success and error response fields, including diagnostic code, stage, and graph path.
- [Version](version.md) returns API identity so a client can select its supported interface.
- [Graph](graph.md) covers draft validation, saved revision, and Apply; the saved instance schema is in [nodes](../node/instance.md).
- [Graph pool](graph-pool.md) stores named drafts independently of the active deployment.
- [Catalog](catalog.md) exposes registered types and selector options without changing the saved graph.
- [Packages](packages.md) covers install, link, inspect, and removal around exact verified pins.
- [Cores](cores.md) exposes owner discovery and readiness to local clients.
- [Runtime](runtime.md) covers Run, Restart, Stop, status, and logs with run/revision fences.
- [Owner control projection](owners.md) forwards bounded public EOS operations and Vision streams through IsOS while EOS retains admission.
- [Presentations](presentations.md) exposes selected output metadata; [UI selection](../ui/selection.md) owns the generic display rules.

[中文](../../../../docs_cn/isos/interfaces/http/README_cn.md)
