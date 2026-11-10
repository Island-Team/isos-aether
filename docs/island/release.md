# Unified IsOS release

[中文](../../docs_cn/island/release_cn.md) · [Shared architecture](README.md) · [Install and upgrade](../isos/design/app/installer.md)

The repository produces one versioned IsOS product release. Development source lives in `isos-p`; `isos-aether` is the intended public snapshot. A qualified bundle release includes the IsOS manager, EOS programs and invoked adapters, EI programs and their dependencies, and the Web and terminal clients. EI and EOS are components of this release, not separately qualified products. Source version, graph format, HTTP schema, protobuf services, node type IDs, and external package identities remain independently versioned. The [IsOS package release contract](../isos/interfaces/packages/release.md) defines verified external package pins; it is not this product release procedure.

## Prepare the candidate

Start from a reviewed integration commit with its exact lockfiles and generated bindings. Record the source commit, toolchain versions, build environment, contract versions, supported platforms, and the intended tag. Review tracked source, tests, documentation, manifests, and Git history for credentials, private endpoints, device identities, calibration, saved routes, captures, runtime state, and unlicensed material. Keep deployment settings, native product skills, site scripts, and local data out of the public snapshot. Retain only C++ adapters invoked by the Rust runtime.

Before tagging, run the root Rust formatting check, strict workspace Clippy, serial workspace tests, locked binary build, applicable CMake/CTest adapter checks, EI Ruff, Pyright, import rules, schema checks, full tests, UI typecheck/tests/builds, and the bilingual documentation audits. Inspect the EI wheel to confirm that tests and private files are excluded. Verify that browser Origin admission remains restricted to the configured public client and that a missing EI configuration root fails explicitly. Exercise the real EOS–EI boundary with the exact selected wire and generated bindings: discovery, admission, duplicate operation IDs, terminal acknowledgement, timeout, cancellation, skill discovery, resource streams, and uncertain Stop. Run IsOS package verification, Apply, Run, Restart, Stop, and `isos doctor` against a clean installed candidate. Save complete outputs and tool versions under ignored `data/test/<run-id>/`; state any unrun gate or known failure in the release receipt.

For a clean public checkout, the core gate is:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets -- --test-threads=1
cargo build --locked --workspace --bins
cmake -S . -B "$BUILD_DIR" -DISLAND_BUILD_TESTS=ON
cmake --build "$BUILD_DIR"
ctest --test-dir "$BUILD_DIR" --output-on-failure
python3 scripts/check_doc_links.py --all
python3 crates/isos/tools/check_isos_docs.py
```

Set `BUILD_DIR`, `TMPDIR`, and `CARGO_TARGET_DIR` to suitable absolute locations when the checkout filesystem is small. Owner-specific test commands and any applicable hardware or simulator replay belong in the candidate receipt; a successful private preparation run alone is not a published-tag result.

With `ISLAND_BUILD_TESTS=ON`, root CTest runs the `aether-runtime` Rust tests as an EOS runtime check. It does not replace the full workspace or managed deployment replay above.

## v0.2.1 source-only beta

The [v0.2.1 release notes](release-0.2.1.md) define a source-only beta exception to the bundle publication path below. Publish the reviewed public snapshot from `isos-aether` as a GitHub pre-release with source archives only; do not attach an installer, binary bundle, or checksum for a bundle that was not built and verified. The existing Release IsOS workflow is for qualified bundles and must not be used for this source-only release. Keep the source version `0.2.1` and tag `v0.2.1` aligned, and record the exact public commit and completed checks in the release receipt. A private `isos-p` test run alone does not qualify the public snapshot.

The v0.2.1 support boundary starts at v0.2.1: code, saved graphs, packages, and interfaces from releases before v0.2.1 are unsupported, and no legacy migration path is promised. This does not change the independent identities of the current graph, HTTP, package, node, and owner contracts. v0.2.1 is testable source for its documented current behavior; it is not a third-party custom-node SDK. Internal Rust APIs and reserved extension declarations are not stable. Planned v0.2.2 custom-node loading and v0.2.3 cross-platform support are target work, not claims about this release or promises that every v0.2.1 extension will run unchanged.

Before publishing, complete the public-source review and applicable source build, test, and end-to-end gates above on the exact public snapshot. Mark every unrun platform or boundary and every known failure in the receipt. The release must say which host platforms were actually tested. A source-only GitHub pre-release must not be presented to `isos upgrade` as an installable bundle; the operator builds from the tagged source using the documented checkout path.

## Build and publish the one product

The manual [Release IsOS workflow](../../.github/workflows/release-isos.yml) accepts an existing `vMAJOR.MINOR.PATCH` tag only when the tag, checked-out commit, and remote `master` match. Rust, EI, and UI source versions must match the tag. Separate Linux x86_64 and aarch64 jobs build bundles with [the root installer](../../install.sh), verify adjacent SHA-256 files, install each bundle into a fresh prefix, and check the installed command. Publication starts only after both platform jobs succeed and refuses to replace an existing GitHub release. The workflow creates that release in the repository where it runs; it does not transfer source or assets from `isos-p` to `isos-aether`. Prepare and review the public snapshot separately before running the workflow there.

Each bundle contains `RELEASE.json` with product version, source commit, OS, and architecture, plus `MANIFEST.sha256` covering installed files and release identity. Record the reviewed manifest and external checksums in the release receipt. Do not tag or publish an unqualified development snapshot to satisfy a checklist.

IsosUI builds a local browser bundle, a hosted browser bundle, and a terminal client. The product archive includes the local browser bundle and terminal client. IsOS serves the local bundle at `/en/isos` and `/cn/isos` and supplies its API origin through `/isosui-runtime.json`; both clients connect to the IsOS server. The hosted bundle is an output for separately managed websites, not a second product installer. UI assets inherit the product version and source commit in `RELEASE.json`. Reproduce their checks from `ui/` with `npm ci`, `npm test`, `npm run typecheck`, and `npm run build`; verify installed Web and TUI attachment after `isos doctor`.

## Install and verify a published bundle

From a complete checkout, `./install.sh --prefix "$HOME/.local/isos"` builds and stages the unified product. To export an archive, use `./install.sh --bundle-output /absolute/isos-<version>-linux-<arch>.tar.gz`; its adjacent `.sha256` is the external checksum. For a downloaded pair, run `sha256sum -c isos-<version>-linux-<arch>.tar.gz.sha256`, then `./install.sh --bundle /absolute/archive.tar.gz`. The installer verifies the internal manifest and host architecture before switching `current`; a corrupt bundle must leave the current installation unchanged. `isos update` only lists releases, while `isos upgrade [version]` verifies and stages one. After installation, run `isos doctor`, inspect readiness and logs, then complete an Apply → Run → Stop replay. Existing processes keep their old executable until explicitly restarted.

Release notes state version and date, operator-visible changes, supported interfaces, installation or upgrade steps, known limits, and reproducible verification. Published tags and artifacts remain immutable.
