# Shared contract catalog

<!-- doc-navigation:start -->
<details>
<summary>Folder: isos/interfaces</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [Interfaces for nodes, packages, cores, and IsosUI](README.md)
  - [Core owner contracts](core.md)
  - [Owner implementation notes](manager.md)
- Subfolders:
  - [api/](api/README.md)
  - [node/](node/README.md)
  - [package/](package/README.md)
  - [server/](server/README.md)

</details>
<!-- doc-navigation:end -->

[中文](../../../docs_cn/isos/interfaces/contract-catalog_cn.md) · [IsOS interfaces](README.md) · [Shared contracts](../../contract.md)

IsOS maintains [`contract/catalog.json`](../../../contract/catalog.json). This is a discovery and ownership catalog for shared contract artifacts, independent of the graph's `<core>/<layer>/<group>/<node>` type and instance identities. A catalog entry has a stable `id`, `purpose`, `origin`, `kind`, `owner`, `consumers`, `domains`, and `paths`. `owner` names the source maintainer; `consumers` name readers. `origin` says whether files are authored, checked against another model, generated, or vendored. These labels do not grant runtime authority or replace the owning wire interfaces.

| To find | Look in | Edit |
| --- | --- | --- |
| Graph node rules and native ABI | `node/` | The schema or header there |
| Cross-owner service or payload | `proto/` | The matching Protobuf source |
| EOS runtime JSON validation | `schemas/actions/`, `schemas/body/`, `schemas/world/` | The matching `<domain>/<name>.proto` source |
| EI northbound validation | `schemas/northbound/` | The shared schema and EI wire model together |
| Private model-provider validation | `schemas/models/` | The matching private JSON Schema |
| North HTTP API description | `schemas/north-http.openapi.json` | The EOS HTTP Protobuf annotations |

One artifact belongs to exactly one catalog. `paths` are repository-relative glob patterns under `contract/`. Every `.proto`, `.h`, and `.json` artifact there must be classified; Markdown guides and the catalog file are excluded. A pattern must match at least one file. The supported kinds are `graph-schema`, `native-abi`, `protobuf-service`, `protobuf-message`, `protobuf-dependency`, `json-schema`, `generated-json-schema`, and `generated-openapi`. Catalog IDs and domain labels use lowercase hyphenated names. Owners and consumers use `isos`, `ei`, or `eos`.

The initial catalogs separate node interfaces, native ABI, owner services, typed payloads, external interfaces, private model schemas, Google API compatibility declarations, generated EOS runtime schemas, EI northbound schemas, and generated North HTTP OpenAPI. Project imports are relative to `proto/`; the `island.eos.*` package names remain unchanged. This changes Protobuf descriptor file names and requires regenerated consumers. `proto/google/api/` holds minimal compatible HTTP annotation declarations under their standard import paths. When adding a contract, select the catalog by its purpose and consumers, then update its path patterns or add a catalog and run the validator. Keep the owning interface page and generated outputs current.

Run `python3 scripts/check_contract_catalog.py` from the repository root for a purpose and count summary and to check metadata, coverage, and duplicate assignments. Use `--explain schemas/actions/arm_movej.v1.json` to see why one file exists and which Protobuf source generates it. Add `--list` to show assignments; `--catalog`, `--domain`, and `--consumer` filter the list by labels (for example, `--domain perception --consumer ei`). `python3 -m unittest tests/test_contract_catalog.py` tests accepted, unclassified, duplicate, and file-explanation cases. The catalog is copied with contracts into installed `share/isos/contract/`; its paths remain relative to that contract root. The single North HTTP description lives directly under `schemas/`; Protobuf package names remain stable; consumers regenerate bindings for the flat import paths.

The updated catalog was checked on 2026-11-16: the validator classified 155 artifacts in 21 groups, three catalog tests and seven product installer tests passed, the North HTTP generator check and Ruff passed, and documentation checks found 0 broken links across 234 files and 28 aligned IsOS page pairs. Reproducible commands and results are saved under ignored `data/test/20261116-contract-catalog-usability/checks.log`.

The flat `proto/` layout was checked on 2026-11-16 with `python3 scripts/check_contract_catalog.py`, `python3 -m unittest tests/test_contract_catalog.py -q`, the three contract generator `--check` commands, `cargo fmt --all -- --check`, `cargo check --workspace --quiet`, `cargo clippy --workspace --all-targets -- -D warnings`, `ei/.venv/bin/python -m pytest ei -q`, Ruff, Pyright, import rules, the product installer tests, EOS wire checks, the R02 fixture, and both documentation checks. All passed; EI reported 744 passing tests. Commands and output are saved in ignored `data/test/20261116-flat-proto/checks.log`.

The flat `schemas/` layout was checked on 2026-11-16 with the EOS and North HTTP generator checks, the EI northbound schema check, catalog validation and tests, EOS contract and manifest tests, 744 EI tests, product installer tests, the R02 fixture, both documentation checks, Ruff, Pyright, import rules, Rust formatting, `cargo check --workspace --quiet`, and `cargo clippy --workspace --all-targets -- -D warnings`. All passed. Commands and results are saved in ignored `data/test/20261116-flat-schemas/checks.log`.
