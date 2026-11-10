# Registered IsOS type catalog

A **core** owns execution, a **layer** groups a major domain, a **group** names a type family, and a **node** is a configured instance. A [registered type](../../interfaces/node/type.md) fixes placement, ports, capabilities, settings, and lifecycle; the saved instance chooses values.

- [Managed component](process/managed-component.md) describes the IsOS-owned process family: a verified package entrypoint is supervised as IsOS's direct child, with owner readiness and stop checks.
- [Vision panel consumer](ui/vision-panel.md) describes how a UI-owned node refers to an output port and selects a rendering method. IsosUI owns layout and subscriptions; IsOS does not render payloads.

Package-contributed types use the same [generic node contract](../../interfaces/node/README.md) and [package verification](../../interfaces/packages/README.md). This catalog explains roles; field-level authority stays in those contracts.

## Owner selector implementation evidence

The first owner selector slice projects EI planner model bindings and EOS adapter, MuJoCo device, and simulator-node choices through setup `options`. IsOS validates the owner response before returning it to the UI. EOS and EI recheck effective selections during `validate` and `render`, including EI defaults. The UI blocks Save and Run if a required owner group or effective choice is unavailable. The draft owner type catalog and options preview run the same selected local setup adapter so a persistent owner on an older generation does not block draft editing. It is read-only and has no authority to admit Apply; Apply still revalidates with the live owner.

From the repository root, run `cargo fmt --all && cargo fmt --all -- --check`, `cargo test -p isos --bin isos --offline`, `cargo test -p aether-runtime --bin aether-profile --offline`, `cargo clippy -p isos -p aether-runtime --bins --offline -- -D warnings`, `cargo build --workspace --bins --offline`, `ei/.venv/bin/python -m pytest ei/tests/ei/test_config_cli.py -q`, `cd ui && npm run typecheck && npm test -- --run && npm run build`, `python3 scripts/check_doc_links.py --all`, and `ei/.venv/bin/python -m unittest discover -s crates/isos/tests -p unified_owner_replay_cli.py -v`. The 2026-11-09 run passed: 349 IsOS tests, 16 EOS profile tests, 16 EI setup tests, 215 UI tests, zero broken bilingual links, and four real-owner replay cases. Full output is in ignored `data/test/20261109-owner-options-*.log`.

[中文](../../../../docs_cn/isos/design/node/README_cn.md)

## Strict port contract evidence

Run `cargo fmt --all -- --check`, `ISOS_AETHER_BIN_DIR="$PWD/target/debug" cargo test -p isos`, `cargo test -p aether-node-profile`, `cd ui && npm run typecheck && npm test && npm run build`, `python3 scripts/check_doc_links.py --all`, and `python3 crates/isos/tools/check_isos_docs.py`. The 2026-11-09 run passed the Rust and UI suites, both UI builds, formatting, and both documentation audits. Complete command output is in ignored `data/test/20261109-strict-node-ports/`. A nonfatal Clippy run passed with existing IsOS dead-code warnings; `-D warnings` remains blocked by those warnings.

## Connected button demo evidence

The first connected button slice contained eight nodes and seven typed edges. The current demo also includes the eight-node observation path described below. IsOS admits a run-fenced button event, EI passes it through input, cognition, and safety, and EOS converts it to a bounded six-joint delta for the mock arm. The built-in type catalogs replaced the old health-only demo IDs. From the repository root, build `isos` and `aether-runtime` binaries, then run `ISOS_AETHER_BIN_DIR="$PWD/target/debug" RUST_TEST_THREADS=1 cargo test --workspace --all-targets`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets`, `cd ui && npm run typecheck && npm test && npm run build`, `python3 scripts/check_doc_links.py --all`, and `python3 crates/isos/tools/check_isos_docs.py`.

The 2026-11-09 isolated replay ran init → validate → Apply → Run → readiness → control → logs → confirmed Stop. One click returned `[0.05,-0.04,0.03,-0.02,0.01,-0.01]` radians; retrying its event ID did not move twice; a new ID accumulated the delta; a stale revision returned 409. Complete command output is under ignored `data/test/20261109-arm-button-integration/`.

## Mock arm monitor replay evidence

The 16-node starter adds eight read-only return nodes bound to the same mock arm. The combined replay used `cargo fmt --all -- --check`, `ISOS_AETHER_BIN_DIR="$PWD/target/debug" RUST_TEST_THREADS=1 cargo test --workspace --all-targets`, `ei/.venv/bin/python -m pytest ei/tests -q`, `cd ui && npm run typecheck && npm test && npm run build`, and the bilingual link audits. An isolated `config init` → validate → Apply → Run → readiness → button activation → monitor read → logs → confirmed Stop replay returned six zeros before movement, the exact six commanded positions afterward, and null monitor values after Stop. The active saved graph was then replaced directly and the browser showed the same positions on both the node and monitor panel after a click. Complete outputs and payloads are in ignored `data/test/20261109-arm-monitor-integration/`.
