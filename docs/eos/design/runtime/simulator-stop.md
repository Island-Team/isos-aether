# EOS simulator Stop design

[中文](../../../../docs_cn/eos/design/runtime/simulator-stop_cn.md) · [Owner contract](../../interfaces/simulator-stop.md) · [Hardware](hardware.md) · [Process supervision](process.md) · [Cases](../../../../tests/fixtures/simulator_stop/README.md)

EOS owns the direct MuJoCo child and the only physical actuator route through Dispatcher. IsOS requests Stop and waits for EOS's selected terminal receipt. A graph revision or process run change fences earlier commands.

## Shutdown and physical fence

The run moves through `Running → Closing → Stopping → Terminal`. Entering `Closing` blocks new movement and closes public and private movement ingress. Dispatcher snapshots admitted base and manipulator targets. One `ManagedMujocoStopCoordinator` serves the child generation; controlled Stop, emergency Stop, and shutdown join its companion safety lane. The companion request advances one engine epoch and returns base and manipulator results together. Simultaneous Stop IDs retain separate journal identities but share the physical request. A timeout, malformed reply, failed part, or lost durability proof remains unknown and keeps movement inhibited.

EOS validates the main and companion sockets and selected generation before enabling managed movement. The companion safety lane is independent of ordinary movement and rendering work. Its physical result is classified before MuJoCo receives TERM or KILL. EOS then drains the child's process group, accounts for descendants, removes sockets, and emits one bounded terminal receipt through the selected pipe. Reaping a leader alone does not prove the group is drained. Forced cleanup does not upgrade an unknown physical outcome to confirmed Stop.

The owner deadline is bounded: admission closes first, physical Stop is classified next, child termination and group drain follow, and the terminal receipt is written only after final classification. IsOS treats a missing or uncertain receipt as degraded and retires the run's grants. The [simulator Stop contract](../../interfaces/simulator-stop.md) defines exact result fields and time limits.

## Verification

Replay a valid base and arm command followed by Stop with a verified `island/mujoco-sim@0.1.1` package. Check one physical stop epoch, both part results, group drain, socket removal, and a terminal selected-pipe receipt. Repeat with blocked ordinary movement, lost companion socket, malformed reply, MuJoCo death, EOS death, and stuck descendants; each failure must remain unknown or degraded without new movement release. Run the [case checker](../../../../tests/fixtures/simulator_stop/README.md), serial Rust tests, formatting, strict Clippy, and the bilingual link audit. Save full output under `data/test/<run-id>/`.
