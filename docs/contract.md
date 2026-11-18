# Shared contracts and source resources

<!-- doc-navigation:start -->
<details>
<summary>Folder: docs</summary>

Browse documents and subfolders here.

- [Repository](../README.md)
- Documents in this folder:
  - [Documentation](README.md)
  - [Documentation changelog](changelog.md)
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

[中文](../docs_cn/contract_cn.md) · [Documentation](README.md)

IsOS manages the root [`contract/`](../contract) tree. EI and EOS consume these shared contracts read-only. The owning [interface pages](isos/interfaces/README.md) explain public payloads with readable JSON examples; contract files are the machine-readable source for validation and generation. Product and EI factory configuration live under root [`config/`](../config), including [`config/defaults/`](../config/defaults). The installed product keeps its own `share/isos/` layout.

| Source directory | Owner and contents | Source status |
| --- | --- | --- |
| [`contract/catalog.json`](../contract/catalog.json) | IsOS contract catalogs and labels; [interface](isos/interfaces/contract-catalog.md) | Authored classification |
| [`contract/node/`](../contract/node/README.md) | IsOS node type and instance JSON Schemas | Authored contracts |
| [`contract/proto/`](../contract/proto/README.md) | IsOS-managed shared Protobuf services and typed payload source | Authored contracts |
| [`contract/proto/google/api/`](../contract/proto/google/api/annotations.proto) | Minimal compatible declarations for standard Google API HTTP annotation imports | Authored dependency declarations |
| [`contract/schemas/actions/`](../contract/schemas/actions), [`body/`](../contract/schemas/body), [`world/`](../contract/schemas/world) | Shared EOS runtime JSON Schemas; [`eos.manifest.json`](../contract/schemas/eos.manifest.json) records identities and checksums | Generated from shared Protobuf contracts |
| [`contract/schemas/models/`](../contract/schemas/models) | Private provider-neutral model request and response validation | Authored contracts |
| [`contract/schemas/northbound/`](../contract/schemas/northbound) | Shared EI northbound JSON Schemas | Checked against EI wire models |
| [`contract/schemas/north-http.openapi.json`](../contract/schemas/north-http.openapi.json) | Shared North HTTP adapter description | Generated from the shared EOS HTTP annotations |

IsOS maintains this source tree and the shared contract source and generated schemas. EI and EOS read these resources; their runtime safety and execution responsibilities stay with those components. EI settings are authored in `config/ei/`; factory defaults in `config/defaults/ei/` link to canonical EI settings where applicable. The source checkout has no `share/` tree. Build and install rules preserve installed `share/isos/` paths. Teaching examples live under [`examples/`](../examples/README.md); public field meanings belong to each component's `interfaces/` pages.

The source flattening was checked on 2026-11-16 with `ei/.venv/bin/python scripts/ei/generate_eos_proto.py --check`, `ei/.venv/bin/python scripts/generate_eos_contracts.py --check`, `ei/.venv/bin/python scripts/generate_north_http_openapi.py --check`, `ei/.venv/bin/python scripts/ei/check_northbound_schemas.py`, `ei/.venv/bin/python -m pytest ei -q` (744 passed), `python3 -m unittest crates/isos/tests/test_product_installer.py -q` (7 passed), `cargo test -p isos domain::registry::node_schema --quiet` (4 passed), `ei/.venv/bin/python tests/eos/test_contract_interfaces.py`, and `python3 scripts/check_doc_links.py --all` (232 files, zero broken links). The installed product retains `share/isos/` as its namespace. Workspace compilation also passed with `cargo check --workspace --quiet`. `cargo test -p isos --quiet` had six failures among 354 tests: the focused starter graph fixture expects 1280 but gets 128, and other failures report an unavailable prebuilt `aether-profile`. Commands and outcomes are recorded under ignored `data/test/20261116-share-root/checks.log`.

The source config move was checked on 2026-11-16: the installer suite passed 7 tests; EI passed 744 tests, Ruff, Pyright, and four import rules; generated IsOS config passed 23 Rust tests and Clippy; formatting, documentation links, and IsOS bilingual page checks passed. Commands and outcomes are recorded under ignored `data/test/20261116-config-root/checks.log`.

The root contract relocation was checked on 2026-11-16 with the contract catalog, EI suite, installer suite, generators, workspace check and Clippy, focused Rust tests, formatting, and documentation audits. The two `package_observer` integration tests fail on a source/input data-class fixture mismatch. Reproducible commands and outcomes are in ignored `data/test/20261116-root-contract/checks.log`.
