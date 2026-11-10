# Rust EOS services

[中文](../../../../docs_cn/eos/design/runtime/services_cn.md) · [Runtime foundation](foundation.md) · [Rust skill contract](../../interfaces/skills/rust.md)

The root Cargo workspace builds the EOS services at source version `0.2.1`. IsOS applies graph format `0.2.0` with separate `isos`, `ei`, and `eos` cores and orchestrates the EOS owner. The persistent owner supervises its runtime worker. The current demo worker owns the declared mock arm adapter and Dispatcher admission. The older general `aether-runtime` path owns HAL adapters, a durable operation journal, and its direct MuJoCo child. The HTTP bridge and TUI use the runtime's owner-only Unix control socket; neither owns an actuator route. The config server publishes one validated, read-only graph snapshot. HTTP services bind to loopback.

| Executable | Invocation | Role |
| --- | --- | --- |
| `aether-profile` | `validate --file <graph>` | Validate the graph and registered node types before admission. |
| `aether-runtime` | `--profile <graph> --journal-dir <dir> --socket <path>` | Own adapters, Dispatcher, journal, gRPC listener, MuJoCo child, and governed Stop. IsOS supplies run, package, and grant inputs for managed use. |
| `aether-http-bridge` | `--socket <path> --listen <loopback:port>` | Proxy bounded control JSON at `/api/eos/v2/control`, health at `/api/eos/v2/health`, and authorized vision reads. |
| `aether-config-server` | `--profile <graph> --listen <loopback:port>` | Serve `/api/eos/v2/config`, `/config/validate`, `/types`, `/data-types`, `/adapter-families`, and `/primitive-specs`; no write route. |

`aether-control` supplies the bounded client shared by the bridge and TUI. Control requests are limited to 64 KiB, responses to 16 MiB, and socket reads have two-second deadlines. `capture` returns RGB8 as `rgb8_base64` and optional depth as `depth_u16_le_base64`; `observe` returns typed base or manipulator data. The bridge validates request size and identity before forwarding. Dispatcher remains the sole actuator admission and release path.

## Rust skill workers

The runtime loads only IsOS-selected, verified native Rust `.skill` executables. It stages verified bytes privately and starts a fresh confined `aether-runtime --skill-worker` child for inspection or invocation. The worker receives a private protocol socket and staged executable descriptor, then applies resource limits, Landlock, and seccomp. Primitive calls return through the host broker and Dispatcher; the worker has no direct adapter handle. EOS owns cancellation, bounded reap, and live execution status. See the [Rust skill contract](../../interfaces/skills/rust.md).

## First use and checks

From the repository root, run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `RUST_TEST_THREADS=1 cargo test --workspace --no-fail-fast`. Start the IsOS manager with `isos serve`, then run an applied deployment with `isos run`; inspect it with `isos status --json` and `isos logs eos --lines 200`. Use `isos deployment stop` for the deployment or `isos stop` for the manager. Direct service invocations are useful for focused boundary tests; production graph ownership and package resolution belong to IsOS.

The 2026-11-07 unified MuJoCo replay exercised Run, RGB camera capture, primitive completion, Restart, confirmed Stop, and child-failure rejection with the Rust services. Its command and raw output are under ignored `data/test/20261107-final/unified-mujoco-replay-fixed2/`. Keep the exact command, source revision, package pin, and environment with each new replay. Test failure and Stop behavior as well as successful control and observation.
