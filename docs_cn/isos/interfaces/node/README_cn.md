# Graph 0.2.0 节点接口

[English](../../../../docs/isos/interfaces/node/README.md) · [IsOS 接口](../README_cn.md)

图包含且仅包含 `isos`、`ei`、`eos` 三个核心。注册类型 ID 为 `<core>.<layer>.<group>.<node>@v0.2`，前缀必须与实例路径一致。注册类型决定位置、端口和生命周期；实例提供 ID、类型、标签、设置和声明的输入。每个核心发布并验证自己的类型，IsOS 在 Apply 时合并目录并验证整张图。

起始图包含两条互不相连的八节点链，共十六个按钮和监控节点。IsOS 管理保存图和运行栅栏；EI 在输入、认知和安全节点验证并传递激活事件；EOS 将事件映射为有界的六关节增量，经调度器准入后更新模拟机械臂。按钮不携带运动目标。独立的只读返回链将同一个机械臂的观测状态经过 EOS 和 EI 的每一层传到 IsOS UI 监控节点。

| 核心 | 起始图中的接口包装节点 | 运行职责 |
| --- | --- | --- |
| IsOS | `isos.ui.button.trigger@v0.2`、`isos.ui.monitor.arm@v0.2` | 发出带运行栅栏的激活事件并显示返回观测。 |
| EI | 分别处理激活与监控状态的输入、认知和治理节点。 | 验证并传递两条声明的路径。 |
| EOS | 分别处理控制与监控的接口、技能、调度器和硬件节点。 | 准入运动、采样同一个模拟机械臂，并在继续运动前检查其状态。 |

十四条边都在保存图中显式声明。`button_activation@v0.2` 跨越 IsOS → EI 和 EI → EOS 的归属方套接字；`arm_joint_delta@v0.2` 留在 EOS 内部。已接受的激活及其最终结果共用事件 ID；结果单独查询，不沿观测链返回。返回链独立于按钮请求，将最新的 `arm_monitor_state@v0.2` 快照从 EOS 经 EI 传到 IsOS。模拟适配器在内存中保存六个关节位置，不连接实体硬件。客户端边界见[节点事件](../ui/node-events_cn.md)和[机械臂监控](../ui/node-monitor_cn.md)。

IsOS 只向每个核心发送其自身的 `cores.<owner>` 投影。核心服务可在 IsOS 退出后继续运行。IsOS 重启后重新连接，并读取持久化的运行身份；一个核心失败不会停止另一个。显式部署 Stop 请求两个核心停止，两个回复都确认后才移除运行身份。

参见 [EI setup](../../../ei/interfaces/setup.md)、[EOS 健康接口](../../../eos/interfaces/owner-health_cn.md)、[图 API](../http/graph_cn.md) 和 [起始图](../../../../crates/isos/templates/generic-graph.toml)。
