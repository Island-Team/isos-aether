# 节点端口

[English](../../../../docs/isos/interfaces/node/ports.md) · [类型](type_cn.md) · [连接](edges_cn.md)

输入端口负责接收，输出端口负责发送。实时端口通过 `transport` 声明通信通道，并声明投递模式、准确注册的负载 `data_type`，以及包含 `owner`、`id`、`version` 的协议 `contract`。两端必须一致，而且 IsOS 仅在归属方支持实际路由时准入连接。仅负载名称相同不能保证投递。

起始图的八个类型具有类型化端口。IsOS → EI 和 EI → EOS 边界使用 `unix_socket`；归属方内部连接使用 `in_process`。激活连接承载 `button_activation@v0.2`；两条 EOS 命令连接承载 `arm_joint_delta@v0.2`。

| 字段 | 含义 |
| --- | --- |
| `name` | 在本方向内唯一的端口名。 |
| `data_type` | 准确的注册负载类型；静态声明使用 `topology_link`。 |
| `required` | 准入是否要求连接。 |
| `transport` | 必需的通道：静态声明使用 `none`；实时端口使用明确支持的载体。 |
| `delivery_mode` | 静态声明使用 `declaration`；否则为准确的实时投递行为。 |
| `contract` | 实时端口必须提供 `{owner,id,version}` 协议身份；静态端口禁止提供。 |
| `cardinality` | 连接数量，通常为 `one`。 |

`none` 与 `declaration` 必须成对出现。Apply 时 IsOS 检查来源和目标声明。EI 与 EOS 分别验证自己的归属方投影。运行时健康来自归属方进程状态，不来自端口。[共享类型模式](../../../../share/isos/contracts/node/node-type.schema.json)定义完整的端口词汇。
