# Rust skill executable contract

[中文](../../../../docs_cn/eos/interfaces/skills/rust_cn.md) · [SkillService](../skills.md) · [Development guide](../../design/runtime/skills/host-rust-guide.md)

EOS runs selected Rust skill executables as confined child processes. An author implements the `aether_skill::Skill` trait and calls `aether_skill::serve` from `main`. The skill declares one `Descriptor` with a name, SemVer version, description, object-valued parameter and result JSON Schemas, declared primitives, and reentrancy. A single executable may register up to 256 skills. Names use lowercase letters, digits, `.`, `_`, and `-`; each separator must be followed by a letter or digit.

The author contract is [`crates/aether-skill/src/lib.rs`](../../../../crates/aether-skill/src/lib.rs). `Skill::execute` receives a command ID, object arguments, and a `Host`. `Host::call_primitive` is the only actuator route. The result is one `Completion` with an object result. `Host::is_cancelled` reports a Stop request. EOS validates descriptor schemas and primitive declarations before publishing the catalog, then validates execution arguments and broker admission for each request.

## Graph selection

The EOS process node's optional `settings.skill_executables` is an array of at most 16 explicit entries. Each entry has `package: {id, version}` and `executable`, a relative path ending in `.skill`. IsOS verifies the package, exact path and digest at Apply and before Run or Restart. It writes `isos/skill-selection.json` into the selected generation and passes its path to EOS through `EOS_SKILL_SELECTION_FILE`. Package installation alone does not select a skill.

The selection file has integer `schema_version: 1`, the saved graph's SHA-256 `active_revision`, and an `executables` array sorted by package ID, package version and executable path. Each entry contains `package_id`, `package_version`, absolute `package_root` and `payload_root`, `receipt_sha256`, `release_sha256`, `payload_sha256`, relative `executable`, and `executable_sha256`. All digests are lowercase SHA-256 hex. EOS rejects changed files, path escapes, links, nonregular files, unsafe permissions, unsupported ELF dependencies and duplicate skill identities. It stages verified bytes privately and inspects each executable in a confined worker before publishing any descriptor.

The runtime reports feature `rust-skill-v1`. A nonempty selection requires that exact feature during IsOS candidate preflight. The preflight report echoes package identity, executable path, digest and skill count, with no package code path or grant exposed through SkillService.

## Process and protocol

Each inspection and execution starts a fresh `aether-runtime --skill-worker` child. The runtime passes only a private `SOCK_SEQPACKET` socket and the staged executable file descriptor. The child clears its environment, verifies its peer and descriptors, sets resource limits, Landlock and seccomp, then executes the staged Rust program. The process protocol is version `2` and carries `Inspect`, `Invoke`, `PrimitiveReply`, `Catalog`, `PrimitiveCall`, `Completion` and `Failure` messages. Messages are bounded; large messages use a sealed memory file. EOS owns cancellation, bounded reap, broker authorization and live status.

The skill process cannot open a network socket or create another process. Its primitive request is checked against the admitted descriptor, graph binding, dispatcher and deadline. Stop requests signal the process and lead to bounded termination if it does not exit. EOS stops admitted actuators before terminating its simulator child.

## Authoring and verification

The runnable [`echo` example](../../../../crates/aether-skill/examples/echo.rs) shows the trait and completion shape. Build a skill as a Linux executable whose final filename ends in `.skill`; use a Cargo locked build. Build the binary with Cargo, include it in a verified IsOS skill release, and select its exact path through the graph.

Run `cargo fmt --all -- --check`, `cargo test -p aether-skill`, and `cargo test -p aether-runtime --features test-hooks --test rust_skill_worker --test rust_skill_catalog`. The process tests cover catalog admission, changed payload rejection, brokered primitive calls, completion and child reap.
