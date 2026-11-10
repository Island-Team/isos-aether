# 注册节点类型

[English](../../../../docs/isos/interfaces/node/type.md) · [节点接口](README_cn.md) · [实例](instance_cn.md)

注册类型固定 `core`、`layer`、`group`、设置、端口、能力和生命周期。类型 ID 为 `<core>.<layer>.<group>.<type>@v0.2`，其中 `core` 为 `isos`、`ei` 或 `eos`。实例只提供身份与已声明的值，不能改变位置或行为。Apply 时 IsOS 合并归属方目录并验证所选类型。EI 和 EOS 实现自身接口行为；图节点只是这些接口的包装。

| 字段 | 当前含义 |
| --- | --- |
| `id`、`core`、`layer`、`group` | 准确的版本化身份和保存位置。 |
| `kind`、`description`、`lifecycle` | 展示角色和归属方生命周期。 |
| `capabilities` | 归属方声明的功能；允许空数组。 |
| `settings` | 实例可用值和默认值。 |
| `inputs`、`outputs` | 类型化连接声明。 |
| `deprecated` | 旧版类型仍可用于已保存图，但不出现在 IsosUI 的添加菜单中。 |
| `implementation`、`service_inputs`、`service_outputs` | 可选的归属方实现和服务声明；仅声明不会启用路由。 |

起始图包含一个 IsOS 按钮、三个 EI 传递节点和四个 EOS 节点，通过七条声明的边连接。按钮发出 `button_activation@v0.2`；EOS 技能和调度器将 `arm_joint_delta@v0.2` 传给模拟机械臂。模拟机械臂是带六个关节名称和有界运动设置的 Manipulator/Adapter 实例。准确 ID 和字段见[起始图](../../../../crates/isos/templates/generic-graph.toml)。

权威声明结构见[共享类型模式](../../../../share/isos/contracts/node/node-type.schema.json)。另见[端口](ports_cn.md)和[连接](edges_cn.md)。
