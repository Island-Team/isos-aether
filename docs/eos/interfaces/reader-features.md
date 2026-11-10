# EOS reader identity

[中文](../../../docs_cn/eos/interfaces/reader-features_cn.md) · [Setup](setup-v1.md) · [Owner lifecycle](owner-health.md)

IsOS probes the selected EOS executable before Apply. `aether-profile --capabilities` reports source package version `0.2.1`, graph format `0.2.0`, node type version `0.2`, and four built-in EOS node types:

- `eos.interface.ingress.trigger@v0.2`
- `eos.skill.demo.joint_delta@v0.2`
- `eos.dispatcher.arm.delta@v0.2`
- `eos.hardware.mock.arm@v0.2`

The `binary` is `aether-profile`; features are `catalog`, `setup-v2`, and `arm-activation-owner`. The catalog also provides `button_activation@v0.2` and `arm_joint_delta@v0.2` data types. Live health and activation results come from the [owner lifecycle](owner-health.md) and [arm control](arm-control.md).
