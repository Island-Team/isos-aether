# EOS on ARM64

[中文](../../../../docs_cn/eos/design/runtime/arm64_cn.md) · [Build and run](../../start/quick-start.md) · [MuJoCo](../process/mujoco-operations.md) · [Architecture](../architecture.md)

Build the Rust workspace on the target host with a supported Rust toolchain and platform C/C++ compiler for the CXX hardware adapter boundary. Run the workspace checks in the [build guide](../../start/quick-start.md). The runtime executables are `aether-runtime`, `aether-profile`, `aether-http-bridge`, `aether-config-server`; IsOS starts them as private owner programs.

Use `isos tui` to inspect and control the applied deployment.

A physical adapter needs its own hardware integration tests for typed samples, health, command bounds, cancellation, and Stop before deployment.
