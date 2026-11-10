# IsOS skill selection cases

[中文](README_cn.md) · [Package contract](../../../../../docs/isos/interfaces/packages/process.md) · [Rust skill contract](../../../../../docs/eos/interfaces/skills/rust.md)

The current selection uses a saved graph in format `0.2.0` with one `isos` core and an EOS process node of type `isos.process.component@v0.2`. Its `settings.skill_executables` entries pin verified `kind:"skill"` packages and exact `.skill` paths. IsOS writes a private, revision-bound selection into the generated manifest and delivers its path only to EOS. EOS verifies and stages each executable before advertising the catalog.

[`cases.json`](cases.json), [`preflight_cases.json`](preflight_cases.json), and [`retirement_cases.json`](retirement_cases.json) describe graph, package, probe, and Stop outcomes. The JSON examples contain synthetic hashes and paths; the runtime tests generate real receipts and private files. The checkers validate the tables and examples:

```sh
python3 crates/isos/tests/fixtures/skill_selection/check_cases.py
python3 crates/isos/tests/fixtures/skill_selection/preflight_cases.py
```

The owner tests cover revision-bound generation, package mutation rejection, EOS-only handoff, candidate preflight, and checked Stop. The worker and catalog tests in `crates/aether-runtime/tests/` cover verified staging and bounded worker lifecycle. Record complete integrated replay output and exact binary revisions under ignored `data/test/<run-id>/`.
