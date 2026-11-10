# EOS MuJoCo adapter binding

> Historical contract. This interface is not implemented by the current button-to-mock-arm graph. See [arm control](arm-control.md) for the active data and command path and [owner lifecycle](owner-health.md) for setup, Run, Status, and Stop.


[中文](../../../docs_cn/eos/interfaces/mujoco_cn.md) · [Hardware contract](hardware.md) · MuJoCo lifecycle · [R06 stop](simulator-stop.md)

EOS owns the internal `mujoco-v2` adapter factory and its mapping from admitted hardware nodes to simulator devices. The external `island/mujoco-sim@0.1.1` package owns the [framed socket contract](https://github.com/Island-Team/isos-mujoco-sim/blob/master/docs/mujoco/interfaces/socket.md); that page defines wire fields, units, examples and errors. IsOS owns package verification and graph admission. Socket protocol `2.0.0`, graph version, type IDs and package release are independent identities.

## Current adapter settings

An EOS camera, base or manipulator instance selects `adapter.type = "mujoco-v2"`. The hardware [profile](hardware.md) owns common fields and limits. All settings below are read by the adapter after graph admission; unknown keys or a mismatch with the verified backend fail admission.

| Field | Type / unit | Purpose, default, validation and compatibility |
| --- | --- | --- |
| `adapter.settings.socket_path` | absolute string / Unix path | Required owner-only socket; must equal the selected backend's `socket` and fit the platform path limit. No implicit default. Existing key retained. |
| `adapter.settings.simulator_node` | nonblank string / device ID | Required device in the package descriptor, such as `cam-0`, `base-0`, or `arm-0`. This is the existing socket `node` value; it is **not** a canonical graph path. Existing meaning retained. |
| `adapter.settings.device_id` | nonblank string / device ID | Required in a managed graph; must match `simulator_node` and the descriptor's device of the hardware kind. Existing meaning retained. |
| `adapter.settings.backend` | canonical node path / — | Required in a managed graph; names the single declared MuJoCo process node, also present in `references`. Existing meaning retained. |
| `node.frame_id` | nonblank string / coordinate frame | Required for hardware observations; EOS adds it to typed values and does not ask the simulator to choose a graph frame. |
| `node.joint_names` | six unique strings / joint names | Required for a manipulator; fixes ordered observation and target slots. Other hardware kinds omit it. |


## R05 managed strict client target


EOS validates the immutable profile before child spawn: exactly one supported backend package pin, one backend path in each adapter's `references` and `adapter.settings.backend`, identical owner-only socket paths, `simulator_node == device_id`, and the package's supported device ID for the node's hardware kind and selected scene. For the exact `0.1.1` catalog, the scene IDs are `pick-place` and `navigation`, and the camera/base/manipulator device IDs are `cam-0`/`base-0`/`arm-0`. IsOS verifies descriptor bytes and scene support at Apply and again before Run/Restart; EOS checks its own graph-to-device binding and then obtains live confirmation from the child. A mismatched pin, scene, kind or binding fails before any adapter becomes ready. No second graph manifest or author-controlled bypass is introduced.

The Q01/R06 archive verifier compares the SHA-bound selected release with the SHA-bound IsOS installed release view without altering either record. It normalizes only two schema-defined empty omissions: IsOS may synthesize `node_contributions.data_types: []` when the selected release omits that field; an installed asset may omit `required_libraries` or `tested_ubuntu` when the selected asset declares an empty array. Nonempty SONAME and tested-Ubuntu arrays must match exactly, including order and values. An unexpected nonempty data type, changed library, missing library, changed Ubuntu version or missing nonempty Ubuntu declaration fails the offline gate. The installed receipt's `release_sha256` still binds the raw installed release bytes before semantic comparison.

Startup readiness is a bounded health exchange, not a successful Unix connect. For every declared MuJoCo device, EOS sends `health` with that exact simulator device ID and a fresh transport request ID, then requires response `version:"2.0.0"`, the echoed ID, `ok:true`, `accepted:true` and `features` containing the exact string `strict-socket-v1`. Camera health must also prove the renderer can capture a frame; a GUI/EGL or JPEG failure is unavailable, even if the Unix socket exists. Unknown optional features are ignored. A missing, malformed, false or contradictory proof fails the backend startup and keeps EOS gRPC health and `eos_ready` unavailable; the direct child is terminated and reaped and the failure is emitted with its run and backend node identity. The health exchange uses one monotonic deadline across nonblocking connect, frame write and every partial read. A bind-before-listen refusal or timed-out exchange is retried while the overall backend startup deadline remains; a malformed or contradictory response fails immediately. The same proof is required on later adapter health refresh. A lost socket or child marks affected nodes unavailable and never synthesizes an observation.

The EOS transport bounds metadata to 64 KiB and payload to 8 MiB, checks frame length, JSON type, protocol and echoed request ID, and requires a bounded nonempty UTF-8 request/command ID of at most 128 bytes. The adapter sends one graph-bound device ID per operation; the simulator checks exact device and operation kind, finite scene limits, and `(node,command_id)` deduplication for the whole child generation. EOS treats a socket `accepted:true` as release acknowledgment only. It records `Completed` only after a fresh base lease-expiry or arm-position observation; timeout, disconnect or contradictory evidence yields durable `OutcomeUnknown`. After a released uncertain command, EOS may query or reconcile but never automatically resends it, even if the simulator can deduplicate a retransmission. A conflicting same-ID payload remains rejected by Dispatcher before socket release.


## Request and lifecycle algorithm

The adapter maps one graph node to one verified simulator device ID, connects to the private socket for a bounded request, checks response identity, size and type, then converts a camera JPEG or simulator observation to typed EOS hardware data. It does not perform navigation or inverse kinematics. For movements, the EOS dispatcher first checks node health, limits, deadline, command identity and safety; the adapter then sends the admitted movement. A socket acknowledgment alone does not prove physical completion: EOS confirms base lease expiry or arm position from fresh observations before recording `Completed`. Missing or conflicting evidence yields `OutcomeUnknown`, and EOS never automatically replays an uncertain released command.

On IsOS-managed startup, EOS starts the verified simulator as its direct child; a local explicit development launch starts a selected but unreceipted binary under the same strict-mode rule. R05 requires strict health proof before graph-declared adapters and backend status become ready. A disconnected socket or unavailable device marks affected nodes unavailable. The [R06 managed stop contract](simulator-stop.md) defines the implemented companion lane, physical epoch fence, independent acknowledgments, terminal receipt and child reap order. A failed physical stop remains visible and cannot be reported as confirmed safety. See the [foundation design](../design/runtime/foundation.md).
