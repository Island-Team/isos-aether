# 图连接

[English](../../../../docs/isos/interfaces/node/edges.md) · [端口](ports_cn.md)

输入引用格式为 `{"node":"core/layer/group/node","output":"out"}`，目标节点把它保存到已声明的输入名下。IsOS 检查两个实例是否存在于注册位置、端口是否匹配，以及图是否无环。如果两端都声明 `transport: "none"` 和 `delivery_mode: "declaration"`，则允许跨核心 `topology_link`；由于不跨边界传输载荷，不需要运行时桥接。

实时连接还要求两端的传输通道、投递模式、准确注册的负载 ID 和协议 `contract` 一致。可信归属方目录中的 `port_routes` 条目必须准确列出源节点类型与输出端口、目标节点类型与输入端口、传输通道、投递模式、负载 ID 和协议契约。跨核心 Unix 套接字桥接还要求两个端点配置均声明桥接端口，并且两个归属方均提供桥接适配器。缺少路由证明时，IsOS 拒绝连接。验证响应提供准确的 `source`、`output`、`target`、`input`，供客户端识别已准入连接。静态 `topology_link` 连接仍只是声明，不承载负载。

起始图有七条连接。按钮激活事件通过三个 EI 节点和四个 EOS 节点传递；仅 EOS 将其转换为六关节增量，并准入至模拟机械臂。

内置目录仅发布演示图使用的七条准确实时 `port_routes`。其他实时连接需要归属方实现及匹配的路由注册。
