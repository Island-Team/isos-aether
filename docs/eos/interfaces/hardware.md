# EOS mock arm hardware boundary

[中文](../../../docs_cn/eos/interfaces/hardware_cn.md) · [EOS interfaces](README.md) · [Arm control](arm-control.md)

The graph `0.2.0` registers `eos.hardware.mock.arm@v0.2` as a six-joint Manipulator/Adapter node. Its input carries `arm_joint_delta@v0.2` from the EOS dispatcher. The node declares six ordered joint names, the `mock-arm` adapter family, `joint_positions` capability, health timing, observation bounds, and movement limits. The worker initializes joint positions to zero for each run and accumulates each admitted distinct delta once.

IsOS admits the full graph before Apply; EOS validates its four-node projection before Run. The dispatcher checks event identity, per-click bounds, and resulting joint positions before the mock adapter updates state. The effective position bound is the smaller of `movement.limits.max_joint_position_rad` and 3.14 rad. Stop closes admission before the child exits. The [arm control contract](arm-control.md) defines payloads, owner route, terminal results, and errors. No camera, base, C++ adapter, or physical arm is constructed for this demo.
