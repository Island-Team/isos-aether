# 配置 EOS

[English](../../../../docs/eos/design/runtime/configure.md) · [配置参考](../../interfaces/configuration_cn.md) · [MuJoCo 首次使用](../process/mujoco-operations_cn.md) · [架构](../architecture_cn.md)

使用 IsOS 创建或编辑图格式 `0.2.0` 的部署。从 [`crates/isos/templates/generic-graph.toml`](../../../../crates/isos/templates/generic-graph.toml) 开始，选择已注册节点和精确、已验证的软件包资源，然后检查图并 Apply。IsOS 渲染 EOS profile 并监督进程；EOS 只读已应用图。

对于 MuJoCo，选择软件包贡献的 `isos.eos_process.mujoco@v0.2` 节点、场景，并连接声明的相机、底盘和机械臂适配器。软件包固定版本和适配器设置须与硬件样本及命令分离。[配置参考](../../interfaces/configuration_cn.md)说明字段与准入。

对于 Rust 技能，使用 `aether_skill::Skill` 构建 `.skill` 可执行文件，将其放入已验证软件包，并通过 EOS 进程节点的 `settings.skill_executables` 选择。IsOS 检查精确字节；EOS 在受限工作进程中执行，并通过 Dispatcher 代理原语调用。参见 [Rust 技能编写指南](skills/host-rust-guide_cn.md)。

按照 [MuJoCo 指南](../process/mujoco-operations_cn.md)完成设置 → Apply → 就绪 → 控制 → 因果日志 → Stop。
