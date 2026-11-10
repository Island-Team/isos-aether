# EOS architecture

[中文](../../../docs_cn/eos/design/architecture_cn.md) · [EOS](../README.md) · [External interfaces](../interfaces/README.md)

EOS is an independent execution core. Its persistent `aether-profile owner` offers setup, Run, Status, Stop, activation, and monitor reads over a private Unix socket. IsOS owns saved graph revisions and starts or reconnects to the EOS owner; an IsOS restart does not stop EOS. The owner starts one `aether-runtime` worker for the admitted run.

## Current graph boundary

Graph format `0.2.0` has separate `isos`, `ei`, and `eos` cores. EOS receives only the EOS projection. The control path is interface ingress → built-in demo skill → arm dispatcher → six-joint mock arm adapter. The independent return path begins at the arm output observation source → monitor dispatcher → status skill → interface egress. The graph declares six in-process EOS edges, one EI-to-EOS activation bridge, and one EOS-to-EI observation bridge. EOS validates all eight nodes and their links before Run. The [arm control contract](../interfaces/arm-control.md) defines event and delta admission; the [arm monitor contract](../interfaces/arm-monitor.md) defines the observation payload and read route.

## Process lifecycle

The [owner lifecycle](../interfaces/owner-health.md) defines the run fence and worker supervision. EOS verifies the projected graph and exact runtime executable digest before launch. Status checks the live worker; Stop closes admission and reaps it. The applied graph remains read-only. The mock adapter retains six joint positions for one worker run; it has no physical hardware connection.

## Future architecture

A public v2 node API and physical ROS 2 or vendor adapters remain separate targets. A physical adapter needs measured stop latency and an independent velocity watchdog before admission. A position-capable gripper would need an explicit target contract. A future world-fusion producer could use the reserved `world.snapshot` schema to combine detections across samples; the current runtime does not publish that resource.
