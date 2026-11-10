# EOS 配置参考

> 历史契约：当前按钮到模拟机械臂图未实现此接口。当前数据及命令路径参见[机械臂控制](arm-control_cn.md)；设置、Run、Status 和 Stop 参见[所有者生命周期](owner-health_cn.md)。


[English](../../../docs/eos/interfaces/configuration.md) · [图 profile](hardware_cn.md) · [IsOS 图](../../isos/interfaces/node/instance_cn.md)

在根目录 `config/deployments/` 编写部署图；IsOS 从该目录保存并 Apply 修订。EOS 不在 `config/` 中另设 TOML，它的设置由已 Apply 的图管理。IsOS 运行设置位于 `config/isos/`，EI 组件 TOML 位于 `config/ei/`，IsosUI 设置位于 `config/isosui.toml`。

EOS 配置来自已应用的 IsOS 图和选定的已验证软件包。图格式为 `0.2.0`，包含独立的 `isos`、`ei` 和 `eos` core；EOS 只读取自己的投影。每个已注册的 `isos.eos_<layer>.<name>@v0.2` 类型声明位置、设置、端口、能力和生命周期；实例值与类型化边选择设备及路线。IsOS 在启动 EOS 前验证精确软件包固定版本和资源；EOS 只读投影，不修改它。

## Layout

从 [`crates/isos/templates/generic-graph.toml`](../../../crates/isos/templates/generic-graph.toml) 开始。IsOS 保存修订并渲染选定的 EOS profile。运行时接收 `--profile`、`--journal-dir` 和 `--socket`；HTTP bridge 接收 `--socket` 与 `--listen`，监听地址必须为回环地址。持久 EOS 所有者监管其运行时工作进程；IsOS 负责部署编排。[进程设计](../design/runtime/process_cn.md)说明启动与就绪。

### EOS 文件存放位置

源码和便携式安装中，IsOS 将 EOS 路径设在 `data/eos/` 下。显式指定的绝对路径仍然有效。系统安装将持久 EOS 文件放在 `/var/lib/island/eos/` 下，将运行中的套接字放在 `/run/island/eos/` 下。

| 目录 | 内容 | 清理规则 |
| --- | --- | --- |
| `state/` | 官方命令日志及运行记录；运行期间包适配器宿主的短期文件 | 保留并备份命令日志；宿主子进程停止时清理其文件。 |
| `user/` | 用户提供的自定义存储、仿真器地图和资源库 | 保留并备份。 |
| `attachments/` | ASR 等附属于运行记录的文件 | 与对应记录一起保留。 |
| `cache/` | 可重新生成的运行数据（如有） | 仅在 EOS 停止时清理。 |
| `log/` | 每次部署的仿真器原始诊断 | 按日志策略保留。 |
| `tmp/` | 短期工作文件（如有） | EOS 停止后清理。 |
| `run/` | 仅所有者可访问的套接字和运行进程文件 | 启动时移除过期项。 |

仅创建实际使用的目录。IsOS 将收集的部署事件写入 `data/isos/log/`；仓库测试证据仍放在忽略的 `data/test/` 下。

## Binding files

相机、底盘或机械臂节点在图中指定适配器及类型化设置。mock CXX 适配器支持本地准入测试。MuJoCo 适配器使用软件包贡献的进程节点以及精确的软件包和资源选择。EOS 在创建适配器前检查已解析的仿真器套接字与设备绑定。[硬件契约](hardware_cn.md)定义身份、单位、界限、健康时序、观察与运动模式。

## 安全与路线

原语绑定输出恰好连接到一个兼容的执行器输入。Dispatcher 在最终释放前检查活动修订、目标、能力、健康、界限、期限和 Stop 状态。相机路线只读。bridge 仅转发有界的已准入操作；参见[控制](control_cn.md)与[视觉](vision_cn.md)。
