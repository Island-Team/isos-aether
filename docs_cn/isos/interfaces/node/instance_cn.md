# 保存的节点实例

[English](../../../../docs/isos/interfaces/node/instance.md) · [配置形态](profile_cn.md) · [类型](type_cn.md)

图格式 `0.2.0` 恰有三个核心键：`isos`、`ei`、`eos`。实例位于 `cores.<core>.layers.<layer>.groups.<group>.nodes.<id>`。映射键等于 `id`；`node_type` 选择在该准确位置注册的 `<core>.<layer>.<group>.<type>@v0.2`。规范引用路径为 `<core>/<layer>/<group>/<id>`。

| 字段 | 含义 |
| --- | --- |
| `id`、`node_type` | 实例身份和注册的包装类型。 |
| `label`、`description` | 展示文本。 |
| `settings` | 仅包含所选类型声明的设置。 |
| `inputs` | 指向已声明来源端口的引用。 |
| `package` | 软件包类型要求时使用的准确验证版本。 |

起始图包含一个 IsOS 按钮、三个 EI 节点和四个 EOS 节点，通过七条声明的边连接。保存图不包含实时进程状态、归属方套接字路径、凭据、事件值或硬件命令。IsOS 管理修订与 Apply；EI 和 EOS 接收各自核心投影。EOS 模拟机械臂实例声明六个关节、适配器和运动限制。参见[实例模式](../../../../share/isos/contracts/node/node-instance.schema.json)。
