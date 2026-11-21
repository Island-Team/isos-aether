# Contributing

<!-- doc-navigation:start -->
<details>
<summary>Folder: docs</summary>

Browse documents and subfolders here.

- [Repository](../README.md)
- Documents in this folder:
  - [Documentation](README.md)
  - [Documentation changelog](changelog.md)
  - [Shared contracts and source resources](contract.md)
  - [Coding and Documentation Conventions](conventions.v2.0.0.md)
  - [Unified IsOS release](release.md)
- Subfolders:
  - [ei/](ei/README.md)
  - [eos/](eos/README.md)
  - [isos/](isos/README.md)
  - [node/](node/README.md)

</details>
<!-- doc-navigation:end -->

[中文](../docs_cn/contributing_cn.md) · [Documentation index](README.md) · [Repository conventions](conventions.v2.0.0.md) · [Contract map](contract.md)

This guide is the working path for changes to IsOS, EI, EOS, the Web client, and the TUI. The [conventions](conventions.v2.0.0.md) set shared source and documentation rules; the owning interface defines public behavior. Start with the [first-use guide](isos/start/quick-start.md) if you have not run this checkout before.

## Find the owner and the current boundary

| Change | Read first | Source |
| --- | --- | --- |
| Saved graphs, Apply, packages, local API, owner supervision | [IsOS interfaces](isos/interfaces/README.md), [architecture](isos/core/architecture.md) | `crates/isos/` |
| Graph type IDs, node instances, ports, registry rules | [Shared node interfaces](node/interfaces/README.md) | `contract/node/`, owner registries |
| Cognition, northbound input, safety gate | [EI interfaces](ei/interfaces.md), [architecture](ei/core/architecture.md) | `ei/` |
| Actuation, hardware, skills, dispatcher admission | [EOS interfaces](eos/interfaces.md), [architecture](eos/core/architecture.md) | `crates/aether-*/` |
| Web and terminal clients | [UI guide](isos/core/ui/webui.md), [IsOS HTTP interface](isos/interfaces/api/http.md) | `ui/` |
| Shared wire payloads and generated validators | [Contract map](contract.md), [catalog interface](isos/interfaces/contract-catalog.md) | `contract/` |

Read the owning `interfaces/` page, the relevant `start/` task, and the focused `core/` design before changing behavior. IsOS owns shared contract edits. EI and EOS consume the applied graph and shared resources read-only. Keep actuator commands behind EOS Dispatcher admission. Current behavior belongs in current guides; planned behavior belongs in the owning `core/roadmap.md` with **Target design — not implemented**.

## Prepare a checkout

Use a focused `<change-type>/<topic>` branch from the confirmed integration target. This repository uses `fix`, `test`, `docs`, or `refactor` without the shared convention's `(v2)` suffix. Preserve unrelated work and check `git status` before editing. The source checkout uses Rust for IsOS and EOS, Python for EI, and Node.js for the UI. For the local product path, run `./install_dev.sh`, `./isos doctor`, and `./isos web`; the [first-use guide](isos/start/quick-start.md) covers package discovery, Apply, Run, logs, and Stop. For a focused change, build and run only the affected boundary.

Never commit ignored local selection, credentials, logs, caches, build output, or field evidence. Keep settings under root `config/`; use environment or ignored local files for secrets. The installed `share/isos/` layout is an output of installation, not a source directory. [Contract map](contract.md) identifies authored, checked, and generated files.

## Make a contract change

1. Identify the owning interface and its current callers. Check installation, runtime, tests, and examples before moving or removing a field or file. Public graph, HTTP, Protobuf, package, and source versions are independent.
2. Update the authoritative source: an owning interface page for public meaning, `contract/proto/` or `contract/node/` for shared wire definitions, `contract/schemas/models/` for private model validation, or the EI northbound wire model and checked schema together. Do not edit generated EOS JSON Schemas, the EOS manifest, or North HTTP OpenAPI by hand.
3. Update implementation, fixtures, tests, first-use instructions, and both language copies in the same change. Interface pages should let a reader understand the wire without opening a schema or `.proto` file: explain field type, unit, purpose, absence/default, validation, compatibility, and show small valid JSON request, success, and error examples using actual wire names. Keep one authoritative definition and link to it from design and usage pages.
4. Regenerate or check the affected output. Use `ei/.venv/bin/python scripts/generate_eos_contracts.py` for EOS runtime JSON, `ei/.venv/bin/python scripts/generate_north_http_openapi.py` for North HTTP OpenAPI, and `ei/.venv/bin/python scripts/ei/generate_eos_proto.py` for EI bindings. Use `ei/.venv/bin/python scripts/ei/check_northbound_schemas.py` for check-only EI northbound validation. Validate catalog coverage with `python3 scripts/check_contract_catalog.py`; `--explain <path>` identifies a file's source and purpose.

The contract source, generated files, and documentation must agree. A local check or install does not qualify or publish a package; follow the [package contract](isos/interfaces/package/README.md) for release rules.

## Write and move documentation

Put a public HTTP, WebSocket, RPC, IPC, plugin, skill, hardware, or data contract under the owner's `interfaces/` tree. Put first-use tasks in `start/`, current implementation in `core/`, and future work in `core/roadmap.md`. Mirror headings, field meanings, JSON examples, and relative links under `docs_cn/`, with `_cn` before the filename. Use indented, valid `json` blocks containing representative objects; put explanation beside the block. For JSON Lines, show one formatted JSON record and state that records are newline-delimited on the wire.

When moving or adding a page, update inbound links, [the documentation index](README.md), and the [English inventory](../doc.md). Use `rg` to find old paths and references. Check links and IsOS bilingual coverage before committing:

Run `python3 scripts/update_doc_navigation.py` after adding, moving, or renaming a page. It refreshes the collapsible parent, sibling, and child-folder links on every English and Chinese page; `--check` reports stale navigation without changing files.

```sh
python3 scripts/update_doc_navigation.py --check
python3 scripts/check_doc_links.py --all
python3 crates/isos/tools/check_isos_docs.py
git diff --check
```

## Verify the changed boundary

Run focused checks first, then the broader checks needed for the actual risk. The [agent guide](../AGENTS.md#change-workflow) lists required boundary checks. Common commands are:

| Boundary | Checks |
| --- | --- |
| Rust IsOS or EOS | `cargo fmt --all`; `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings`; relevant `cargo test -p <package>` |
| EI | From `ei/`: `.venv/bin/ruff check .`; `.venv/bin/pyright`; `PYTHONPATH=.:package .venv/bin/lint-imports --cache-dir ../data/cache/tools/import-linter-ei`; `.venv/bin/pytest -q` or focused tests |
| UI | From `ui/`: `npm run typecheck`; `npm test`; `npm run build` for browser and TUI output |
| Shared contracts | Generator `--check` commands, `python3 scripts/check_contract_catalog.py`, focused EOS/EI wire and installer tests |
| Documentation | Link, bilingual, inventory, and `git diff --check` audits |

For a runtime boundary, exercise success, rejection, dependency failure, and Stop where applicable. Save complete commands and output under ignored `data/test/<run-id>/`; summarize reproducible commands and outcomes in the owning guide. Record known failures precisely and distinguish existing fixture failures from new failures.

## Review and publish

Review `git diff` for unintended changes, generated output, private data, broken examples, and stale paths. Commit a coherent slice with a `docs:`, `refactor:`, `fix:`, `test:`, or `chore:` prefix. Follow the [shared change and release workflow](conventions.v2.0.0.md#change-and-release-workflow) for PR, testing branch, independent review, merge order, and cleanup. Confirm the integration target before publishing. A reviewed package release has separate qualification and immutable assets under the [package contract](isos/interfaces/package/README.md).

The 2026-11-16 readable-contract documentation pass checked all 234 Markdown files for relative links (none broken), 28 IsOS English/Chinese page pairs (no issues), 140 JSON blocks in changed pages (all parsed), and complete English coverage in `doc.md`. `git diff --check` passed. Reproducible commands and output are saved under ignored `data/test/20261116-readable-contracts/checks.log`.
