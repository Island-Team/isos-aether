# Documentation

<!-- doc-navigation:start -->
<details>
<summary>Folder: docs</summary>

Browse documents and subfolders here.

- [Repository](../README.md)
- Documents in this folder:
  - [Documentation changelog](changelog.md)
  - [Shared contracts and source resources](contract.md)
  - [Contributing](contributing.md)
  - [Coding and Documentation Conventions](conventions.v1.4.0.md)
  - [Unified IsOS release](release.md)
- Subfolders:
  - [ei/](ei/README.md)
  - [eos/](eos/README.md)
  - [isos/](isos/README.md)
  - [node/](node/README.md)

</details>
<!-- doc-navigation:end -->

[中文](../docs_cn/README_cn.md) · [Repository](../README.md)

IsOS owns the saved graph, package verification, Apply, and deployment run identity. EI and EOS each own a persistent process lifecycle interface and worker health. The current graph format is `0.2.1` with `isos`, `ei`, and `eos` cores and `<core>.<layer>.<group>.<node>@v0.2` type IDs. The [current demo graph](../config/graph-pool/demo.nodes.json) has separate button-to-arm and arm-observation chains.

| Read first | Purpose |
| --- | --- |
| [Shared nodes](node/README.md) | Graph, type, port, data, and registry contracts used across owners. |
| [IsOS start](isos/start/quick-start.md) | Configure, verify packages, apply, run, inspect, and stop. |
| [EI](ei/README.md) | Cognition, safety, and owner interfaces. |
| [EOS](eos/README.md) | Execution, hardware, dispatcher, and simulator contracts. |
| [IsosUI](isos/core/ui/webui.md) | Graph workbench and live owner state. |
| [IsOS architecture](isos/core/architecture.md) | Cross-owner contracts, data flow, and runtime targets. |
| [Unified release](release.md) | One product release, qualification gates, and published bundle verification. |
| [Changelog](changelog.md) | Completed UI delivery and earlier source beta planning. |
| [Contributing](contributing.md) | Repository-wide contribution entry point and documentation workflow. |
| [Conventions](conventions.v1.4.0.md) | Source, contract, and documentation rules. |
| [Shared contracts](contract.md) | Ownership and location of authored, checked, and generated contract files. |

The owning component's `interfaces/` tree defines public fields and wire behavior with readable JSON examples. The [shared contract map](contract.md) points contributors to machine-readable sources and generated artifacts. Source version `0.2.3` and graph format `0.2.1` are independent of HTTP, protobuf, and package identities.

## Current node boundaries

| Scope | Available behavior | Owning contract |
| --- | --- | --- |
| Starter graph | The current 17-node demo has its documented owner routes and runtime paths. | [Graph nodes](isos/interfaces/node/README.md) |
| Isolated EI and EOS libraries | Direct, typed one-node calls and fixtures are available. Their new IDs are absent from graph-supported identities, and ordinary graph setup rejects them. | [EI cognition](ei/interfaces.md#cognition_isolated_library) and [EOS handlers](eos/interfaces.md#isolated_library) |
| EOS physical scene types | Metadata and reference descriptors are registered for validation; they provide no node, decoder, artifact reader, or capture path. | [Scene data](eos/interfaces.md#scene_data) |

Current guides describe implemented behavior. Future algorithms and target behavior appear in the [IsOS](isos/core/roadmap.md), [EI](ei/core/roadmap.md), and [EOS](eos/core/roadmap.md) roadmaps.

## Documentation layout verification

The 2026-11-15 node and runtime layout change was checked from the repository root with `python3 scripts/check_doc_links.py --all`, `python3 crates/isos/tools/check_isos_docs.py`, a repository-wide Markdown link audit using `scripts/check_doc_links.py`'s `check_file`, an English inventory comparison against `doc.md`, and `git diff --check`. The checks passed: zero broken relative links, 71 aligned IsOS English/Chinese pages, and all 274 English Markdown files represented in `doc.md`. Complete output is saved under ignored `data/test/20261115-node-core-doc-layout/checks.log`.

The subsequent semantic consolidation was checked with the same documentation and inventory audits plus `python3 tests/fixtures/r03_admission_revision/check_cases.py`. All passed: zero broken relative links, 65 aligned IsOS page pairs, 212 English Markdown files represented in `doc.md`, and 25 frozen R03 cases with current documentation anchors. Complete output is saved under ignored `data/test/20261115-merge-related-docs/checks.log`.

The 2026-11-15 README pruning moved EOS runtime and external-interface navigation into their owning indexes and removed three redundant English/Chinese README pairs. From the repository root, `python3 scripts/check_doc_links.py --all` checked 365 Markdown files with zero broken relative links, `python3 crates/isos/tools/check_isos_docs.py` found 65 aligned IsOS page pairs with zero issues, and `git diff --check` passed. Complete output is under ignored `data/test/20261115-prune-readmes/`. The historical `doc.md` inventory was left untouched.
