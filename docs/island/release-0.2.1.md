# IsOS v0.2.1 source-only beta release notes

[中文](../../docs_cn/island/release-0.2.1_cn.md) · [Release process](release.md) · [Documentation](../README.md)

**Status:** release candidate, not yet published. **Date:** 2026-11-10 candidate; use the publication date in the GitHub release.

## Operator-visible changes

This is a native, testable source snapshot of the unified IsOS, EI, EOS, Web, and TUI product at source version `0.2.1`. It is a beta for development and evaluation. The current demo uses graph format `0.2.0` and the registered `isos`, `ei`, and `eos` node types. The [first-use guide](../isos/start/README.md) describes the supported setup, Apply, readiness, control, logs, and Stop path. No prebuilt executable or product installer is part of this source-only release.

## Supported interfaces and compatibility

v0.2.1 is the oldest supported product baseline. Code, saved graphs, packages, and interfaces from versions before v0.2.1 are unsupported; this release offers no legacy migration path. A source version does not itself establish wire compatibility. The current graph format is `0.2.0` with `@v0.2` node types; the IsOS HTTP schema is `0.1.0`; the package contract is `isos.package@v1` with schema `0.1.0`. Consult the [IsOS](../isos/interfaces/README.md), [EI](../ei/interfaces/README.md), and [EOS](../eos/interfaces/README.md) contracts for the exact supported fields and owner protocols. The selected binaries and package pins must pass their capability and verification checks.

The public extension contract for third-party custom nodes is not complete in v0.2.1. Package declarations and reserved implementation fields do not grant a general native loader or stable plugin ABI. Internal Rust module APIs may change in v0.2.2. Do not treat code built against those internals as a supported v0.2.1 plugin.

## Build and evaluate

Check out the immutable `v0.2.1` tag from the public `Island-Team/isos-aether` repository. Follow the [source quick start](../isos/start/README.md) and run `./install_dev.sh`, `./isos doctor`, and `./isos web` on a host listed in the published verification receipt. Build and test from that tag; there is no v0.2.1 binary download to install with `isos upgrade`. Check exact package pins and owner readiness before running a graph. Stop the deployment through IsOS after evaluation.

## Known limits and planned work

This beta supports only the host platforms named in its published verification receipt. It does not provide a stable custom-node SDK, a general `.so` loader, or a cross-platform build claim. **Target design — not implemented:** v0.2.2 is planned to define and test custom-node packaging, loading, lifecycle, and Stop; v0.2.3 is planned to extend supported host platforms. These plans do not promise that private v0.2.1 extension code will work unchanged in later releases.

## Verification

The GitHub release receipt must record the exact public commit, tested host OS and architecture, toolchain versions, reproducible commands and outcomes, end-to-end replay, and every unrun gate or known failure. Results from the private preparation repository do not replace checks on the public snapshot. This candidate note does not claim those checks have passed.
