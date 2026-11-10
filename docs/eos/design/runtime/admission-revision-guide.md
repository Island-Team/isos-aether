# Governed actuator admission: development and verification

[中文](../../../../docs_cn/eos/design/runtime/admission-revision-guide_cn.md) · [Owner contract](../../interfaces/grpc.md#r03-governed-actuator-admission-target-contract) · [Design](admission-revision.md) · [Cases](../../../../tests/fixtures/r03_admission_revision/README.md)

IsOS issues a run-scoped grant for the selected graph revision. EOS validates the grant, active binding, health, deadline, and caller identity before Dispatcher releases an actuator command. A request admitted under a prior run or revision is rejected. The journal stores one result for each operation ID; a lost response or uncertain outcome never permits automatic physical replay.

## First-use replay

1. Build the Rust workspace and start IsOS with a verified graph containing separate `isos`, `ei`, and `eos` cores with registered `<core>.<layer>.<group>.<node>@v0.2` types.
2. Start EOS through IsOS and confirm the managed grant and active graph revision are delivered to the selected child only.
3. Submit a valid base or manipulator command through the public EOS interface. Read the same operation ID until a terminal result; confirm one journal record and one adapter release.
4. Repeat with an invalid grant, caller, target, revision, payload, and expired deadline. Each request must fail before adapter release and must not reveal another caller's result.
5. Exercise cancellation, process failure, a journal fault, and Stop while a command is pending. Confirm the release fence, durable result classification, and physical Stop order described in the [simulator stop contract](../../interfaces/simulator-stop.md).

Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, serial workspace tests, the [case checker](../../../../tests/fixtures/r03_admission_revision/README.md), and `python3 scripts/check_doc_links.py --all`. Save complete output under `data/test/<run-id>/` and record the exact revision, commands, environment, and case-to-test mapping with the run.
