# IslandOS services

<!-- doc-navigation:start -->
<details>
<summary>Folder: isos/core</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [IsOS core](README.md)
  - [IsOS architecture](architecture.md)
  - [IsOS CLI](cli.md)
  - [IsOS configuration](config.md)
  - [Local core logs](logger.md)
  - [IsOS packages](pkg.md)
  - [IsOS roadmap](roadmap.md)
  - [IsOS server](server.md)
- Subfolders:
  - [ui/](ui/)

</details>
<!-- doc-navigation:end -->

[中文](../../../docs_cn/isos/core/service_cn.md) · [Core index](README.md)

## Independent service definition

[中文](../../../docs_cn/isos/core/service_cn.md) · [Manager IPC](../interfaces/manager.md#manager_protocol)

This page describes the service definition used by the owner-only manager, **not an HTTP route**. Services are outside the deployment graph and start only through an explicit CLI or manager operation. `isos serve` and `isos run` do not implicitly start them. Manager shutdown stops its services. The current definition is TOML `version="1.0.0"`; its version is independent of the IsOS HTTP schema.

```toml
version = "1.0.0"
id = "example"
command = ["/absolute/verified/launcher"]
cwd = "/absolute/verified"
depends_on = []
restart = "on_failure"
max_restarts = 2
start_timeout_ms = 30000
stop_timeout_ms = 5000

[artifact]
path = "/absolute/verified/launcher"
sha256 = "<64 lowercase hexadecimal digits>"

[readiness]
kind = "tcp"
target = "127.0.0.1:18003"
```

| Field | Type / unit | Purpose and validation | Absence/default |
| --- | --- | --- | --- |
| `version`, `id` | strings / definition version and stable ID | Exact parser version and service identity. | Required. |
| `command`, `cwd` | string array and absolute path / process input | Explicit launch input, never inferred from package ID. | Required. |
| `depends_on` | string array / service IDs | Order among independent services; cycles reject. | Empty if none. |
| `restart`, `max_restarts` | enum and integer / policy, attempts | Bound automatic recovery. | Required. |
| `start_timeout_ms`, `stop_timeout_ms` | positive integers / milliseconds | Bound readiness and termination. | Required. |
| `artifact.path`, `.sha256` | absolute path and 64-hex / bytes | Bind the selected executable. | Required. |
| `readiness.kind`, `.target` | strings / probe | Owner checks declared readiness before success. | Required. |

A service definition does not provide a package-specific parser or bypass the generic package contribution path for graph nodes.

## Selected package services

The `0.2.1` release schema can declare package runtime services for EOS hardware nodes. Apply reads only selected `adapter.package` and `adapter.settings.service_instance` pairs, validates their deployment-local package configuration, and records its exact digest in the immutable generated manifest. Run and Restart compare that digest before launching. IsOS deduplicates selected nodes by package ID, version, and service instance, starts each verified entrypoint as a direct child in its own session, and records a run-fenced receipt before reporting readiness. EOS receives a node-path keyed private socket handoff. Legacy `0.2.0` process nodes continue through their existing lifecycle.

Stop requests owner shutdown before terminating package services. A missing owner proof or unexpected package child exit remains unknown. The service receipt contains PID start time and run identity so a restarted manager cannot signal a reused PID or apply a stale Stop. This path currently supports Linux Unix sockets; other platform loaders remain future work.

Before a new Run, IsOS drains and clears a previous run's package-service receipt only when every recorded child is confirmed dead by PID and start time. A still-live child keeps the receipt and blocks the new Run. A failed start cleans up only a receipt belonging to that candidate run.

The 2026-11-18 stale-receipt recovery check is recorded under ignored `data/test/20261118-stale-package-receipt/`. Reproduce with `ISOS_AETHER_BIN_DIR=$PWD/target/debug cargo test -p isos --bin isos`, `cargo clippy -p isos --bin isos -- -D warnings`, and `cargo fmt --all -- --check`. After confirming the recorded child is dead, Stop, Apply the saved graph at its current revision, and Run; the local replay started EOS and EI with the MuJoCo source-installed package.
The focused receipt tests, Clippy, and formatting passed. The full IsOS run passed 362 tests and failed two: the demo fixture expectation differs from a pre-existing local `rgb8` edit, and a child-restart timing test passed when rerun alone.

The 2026-11-12 supervisor slice evidence is under ignored `data/test/20261112-package-service-supervisor/`. Reproduce with `ISOS_AETHER_BIN_DIR=<source-checkout>/target/debug cargo test -p isos --bin isos`, `cargo clippy -p isos --bin isos -- -D warnings`, `cargo fmt --all -- --check`, and `python3 scripts/check_doc_links.py --all`. All passed (320 tests, two ignored). The real package Apply → Run → Stop qualification is part of the combined integration slice, not this isolated supervisor test.

Selected service stdout and stderr are written to private mode `0600` files, then ingested as bounded, control-character-sanitized IsOS `package_service_output` events with the receipt's run ID, exact package pin, instance, stream, and generation. Status and Stop drain new bytes; a manager restart can continue from receipt offsets without breaking the child's output descriptors. An Apply generation also fingerprints declared external file requirements, including local native libraries, so changed bytes block Run before service launch.

The 2026-11-12 logging and external prerequisite follow-up is under ignored `data/test/20261112-package-service-supervisor-followup/`. Reproduce with `cargo build -p aether-runtime --bin aether-profile`, `ISOS_AETHER_BIN_DIR=$PWD/target/debug cargo test -p isos --bin isos`, `cargo clippy -p isos --bin isos -- -D warnings`, `cargo fmt --all -- --check`, and `python3 scripts/check_doc_links.py --all`. The isolated IsOS suite passed 322 tests with two ignored; the local EOS binary is required because catalog tests compare its live types.

The 2026-11-12 review fix requires each selected device to answer a framed `health` request with `ok`, `accepted`, `strict-socket-v1`, and the exact IsOS service generation before startup or status can report ready. Stop waits for exit and reaps the child after TERM or KILL within bounded deadlines. Package configuration reads use `O_NOFOLLOW` and compare the opened file's inode and device with the inspected private file. Evidence is under ignored `data/test/20261112-supervisor-review-fixes/`.

Package service Unix sockets use a short Linux runtime path under `/tmp/isos-package-<uid>/`. IsOS creates and verifies this owner-private mode `0700` directory, hashes the deployment runtime path, run ID, and exact service pin into the socket basename, and records the resulting path only in the run receipt and EOS handoff. The socket and `.stop` companion are removed on Stop. This keeps Unix socket addresses within the kernel limit even when `ISOS_HOME` or qualification paths are long. A focused long-path test covers the address bound and private directory mode.

The 2026-11-12 short-socket evidence is under ignored `data/test/20261112-short-package-socket/`. Reproduce with `ISOS_AETHER_BIN_DIR=$PWD/target/debug cargo test -p isos --bin isos`, `cargo clippy -p isos --bin isos -- -D warnings`, `cargo fmt --all -- --check`, and `python3 scripts/check_doc_links.py --all`. The IsOS suite passed 327 tests with two ignored; Clippy, formatting, and links passed.

## Managed component node

[中文](../../../docs_cn/isos/core/service_cn.md) · [Graph contract](../../node/interfaces/graph.md#instance) · [Process contribution](../interfaces/package/README.md#process)

The registered process type places an IsOS-managed component under `isos/process/component`. Its type declaration determines allowed capabilities, settings, ports, and lifecycle; an instance supplies stable identity, dependencies, exact package pin, and declared setting values. IsOS resolves only a verified entrypoint and supervises only its own direct child. A package-contributed process for another core is handed to that owner. No arbitrary graph command or package-ID-specific launcher is accepted.

Readiness is owner-verified. Failed readiness or unknown stop cannot become a successful run.
