# Configure EOS

[中文](../../../../docs_cn/eos/design/runtime/configure_cn.md) · [Configuration reference](../../interfaces/configuration.md) · [MuJoCo first use](../process/mujoco-operations.md) · [Architecture](../architecture.md)

Use IsOS to create or edit a graph format `0.2.0` deployment. Start from [`crates/isos/templates/generic-graph.toml`](../../../../crates/isos/templates/generic-graph.toml), select registered nodes and exact verified package resources, then Check graph and Apply. IsOS renders the EOS profile and supervises its processes. EOS treats the applied graph as read-only.

For MuJoCo, choose the package-contributed `isos.eos_process.mujoco@v0.2` node, select a scene, and connect declared camera, base, and manipulator adapters. Keep the package pin and adapter settings separate from hardware samples and commands. The [configuration reference](../../interfaces/configuration.md) describes fields and admission.

For Rust skills, build a `.skill` executable with `aether_skill::Skill`, include it in a verified package, and select it through the EOS process node's `settings.skill_executables`. IsOS checks exact bytes; EOS runs a confined worker and brokers primitive calls through Dispatcher. See [Rust skill authoring](skills/host-rust-guide.md).

Follow setup → Apply → readiness → control → causal logs → Stop in the [MuJoCo guide](../process/mujoco-operations.md).
