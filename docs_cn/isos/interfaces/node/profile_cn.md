# 保存图中的节点配置

[English](../../../../docs/isos/interfaces/node/profile.md) · [节点接口](README_cn.md) · [实例](instance_cn.md)

节点配置是保存的实例，不是第二份类型声明，也不是实时状态记录。图格式 `0.2.0` 包含 `isos`、`ei`、`eos` 三个核心。注册类型固定 `<core>.<layer>.<group>.<type>@v0.2`；配置位于匹配的 `cores.<core>.layers.<layer>.groups.<group>.nodes.<id>` 路径。映射键与 `id` 一致。

```text
cores.isos.layers.ui.groups.button.nodes.trigger
cores.ei.layers.io.groups.input.nodes.trigger
cores.ei.layers.cognition.groups.intent.nodes.forwarder
cores.ei.layers.governance.groups.safety.nodes.gate
cores.eos.layers.interface.groups.ingress.nodes.trigger
cores.eos.layers.skill.groups.demo.nodes.joint_delta
cores.eos.layers.dispatcher.groups.arm.nodes.delta
cores.eos.layers.hardware.groups.mock.nodes.arm
```

每个配置选择类型，并可提供标签、描述、已声明设置，以及注册类型具有端口时的输入引用。起始图的七条连接具有类型化输入和输出。[通用图模板](../../../../crates/isos/templates/generic-graph.toml)包含完整的可部署示例。

IsOS 保存并验证图，然后渲染归属方投影。EI 和 EOS 分别验证自己的投影并监督进程。归属方状态独立报告就绪和故障。模拟硬件适配器在内存中保存六个关节位置，不连接实体硬件。
