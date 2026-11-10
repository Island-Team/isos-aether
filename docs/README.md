# Documentation

[中文](../docs_cn/README_cn.md) · [Repository](../README.md)

IsOS owns the saved graph, package verification, Apply, and deployment run identity. EI and EOS each own a persistent process lifecycle interface and worker health. The current graph format is `0.2.0` with `isos`, `ei`, and `eos` cores and `<core>.<layer>.<group>.<node>@v0.2` type IDs. The [current demo graph](isos/interfaces/node/README.md) has separate button-to-arm and arm-observation chains.

| Read first | Purpose |
| --- | --- |
| [IsOS start](isos/start/README.md) | Configure, verify packages, apply, run, inspect, and stop. |
| [EI](ei/README.md) | Cognition, safety, and owner interfaces. |
| [EOS start](eos/start/README.md) | Execution, hardware, dispatcher, and simulator operations. |
| [IsosUI](isosui/README.md) | Graph workbench and live owner state. |
| [Shared architecture](island/README.md) | Cross-owner contracts and data flow. |
| [Unified release](island/release.md) | One product release, qualification gates, and published bundle verification. |
| [v0.2.1 source beta notes](island/release-0.2.1.md) | Candidate scope, compatibility boundary, and source-only verification. |
| [Conventions](island/conventions.v1.4.0.md) | Source, contract, and documentation rules. |

The owning component's `interfaces/` tree defines public fields and wire behavior. Source version `0.2.1` and graph format `0.2.0` are independent of HTTP, protobuf, and package identities.

Current guides describe implemented behavior. Future algorithms and target behavior appear only in clearly marked architecture sections, beginning with the [shared architecture](island/architecture.md#target-architecture-not-implemented).
