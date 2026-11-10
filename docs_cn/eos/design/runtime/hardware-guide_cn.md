# 开发 EOS 硬件适配器

[English](../../../../docs/eos/design/runtime/hardware-guide.md) · [硬件接口](../../interfaces/hardware_cn.md) · [运行时设计](hardware_cn.md) · [Dispatcher 设计](dispatcher_cn.md)

从注册节点类型及其所属接口契约开始。当前演示的 EOS 硬件节点在图格式 `0.2.0` 中位于 `eos/hardware/mock/arm`。注册表声明能力、类型化端口、设置、界限、健康时序及适配器族；图实例提供身份和选定值。不要创建第二份清单或执行器路线。

## 修改或增加节点

1. 同步更新[硬件契约](../../interfaces/hardware_cn.md)及其英文镜像，说明每个公开字段的类型、单位、用途、默认值和校验。
2. 每个节点实现一个适配器。设备通信留在 Rust HAL 或其狭窄的 CXX/C ABI 边界后；传递有界的类型化样本与命令。
3. 创建设备前校验设置与引用。显式返回健康和状态，拒绝过期或畸形观察，并用持久命令 ID 关联每个命令。
4. 所有运动经 Dispatcher 准入。在适配器写入边界实现独立实体 Stop，并验证租约运动的看门狗。
5. 添加 mock 单元测试及集成边界，覆盖成功、拒绝、故障、重启和 Stop。

## 检查与日志

运行 `cargo fmt --all`、`cargo fmt --all -- --check`、工作区 Clippy、聚焦的 HAL/Dispatcher/运行时测试、适用的仿真器或硬件集成，以及 `python3 scripts/check_doc_links.py --all`。完整输出保存到忽略的 `data/test/<run-id>/`；在所属指南记录可复现命令与结果。基准测试先使用固定工作负载，报告吞吐量、中位数、p99 和环境，再确定性能目标。

内置 EOS 目录直接由 `crates/aether-node-profile/src/registry.rs` 中当前的 `eos.*@v0.2` 节点定义构建。使用 `cargo test -p aether-node-profile` 验证注册表与边准入，使用 `cargo test -p aether-dispatcher` 验证命令准入，使用 `cargo test -p aether-runtime --lib` 验证运行时集成。

2026-11-07 的按所有者组织文件目录文档变更通过了 `python3 scripts/check_doc_links.py --all`（464 个 Markdown 文件，无失效相对链接）。完整输出位于已忽略的 `data/test/20261107-eos-paths/doc-links.log`。EOS 运行时路径由必填参数或显式参数注入；源码、便携式及系统安装的默认值由 IsOS 决定。

2026-11-08 的节点不匹配事件和侧文件位置对齐通过了 `cargo fmt --all -- --check`、`cargo test -p aether-common -p aether-runtime --bin aether-runtime --bin aether-profile`、`cargo clippy -p aether-common -p aether-runtime --all-targets -- -D warnings` 和 `python3 scripts/check_doc_links.py --all`。完整输出位于已忽略的 `data/test/20261108-eos-node-alignment/`。

包贡献的节点类型同样使用 `isos.<layer>.<name>@v0.2` 身份，并提供非空描述及完整说明的类型化端口和设置。注册表在其他图类型身份进入目录前予以拒绝。

## 按钮到模拟机械臂演示，2026-11-09

EOS 投影包含一个实时入口、技能、调度器和模拟机械臂节点。模拟机械臂注册为机械臂适配器，并具有 `mock_arm` 安全适配器。工作进程在同一次运行中保存六个关节位置。内置技能将已准入的激活转换为固定相对增量，Dispatcher 在模拟适配器更新状态之前检查单次命令和结果位置的限值。线上的字段和单位见[机械臂控制契约](../../interfaces/arm-control_cn.md)。

通过 `cargo test -p aether-node-profile`、`cargo test -p aether-runtime --bin aether-profile --bin aether-runtime --test arm_activation`、`cargo clippy -p aether-node-profile -p aether-runtime --all-targets -- -D warnings`、`cargo fmt --all -- --check` 和 `python3 scripts/check_doc_links.py --all` 验证。socket 集成测试覆盖 Run、两次激活、重复事件回放、过时修订版本拒绝及 Stop。完整命令输出保存于被忽略的 `data/test/20261109-eos-arm-demo/`。

## 模拟机械臂观测返回，2026-11-09

[机械臂监控契约](../../interfaces/arm-monitor_cn.md)定义读取模拟机械臂采样器的四个返回路径节点、一种类型化观测载荷和 `POST /v1/monitor`。工作进程默认以 100 Hz 以及每次运动后立即采样。模拟机械臂高级设置公开 10 ms 采样周期和 50 ms 新鲜度阈值；有界的图覆盖值传至工作进程。过期或无效样本阻止运动，直到新的有效样本恢复就绪。

## 独立模拟观测源，2026-11-10

新演示以零图输入的 `eos.hardware.mock.arm_observer@v0.2` 作为返回链首节点。它读取现有适配器的周期采样器；命令链结束于唯一的模拟机械臂执行器。旧的有输入 `arm_output@v0.2` 路径仍适用于已保存图。从仓库根目录运行 `cargo fmt --all -- --check`、`cargo test -p aether-node-profile --tests`、`cargo test -p aether-runtime --test arm_activation`、`cargo clippy -p aether-node-profile -p aether-runtime --all-targets -- -D warnings` 和 `python3 scripts/check_doc_links.py --all`。Profile 测试覆盖两种图路径，并拒绝观测源从命令链接收输入。所有者/工作进程 socket 测试证明激活前观测序号已经推进，运动后继续推进，并在 Stop 时结束。完整输出保存在忽略目录 `data/test/eos-observer-20261110/`。

从仓库根目录运行 `cargo fmt --all`、`cargo fmt --all -- --check`、`cargo test -p aether-node-profile --tests`、`cargo test -p aether-runtime --bin aether-profile --bin aether-runtime --test arm_activation`、`cargo clippy -p aether-node-profile -p aether-runtime --all-targets -- -D warnings` 和 `python3 scripts/check_doc_links.py --all`。单元测试覆盖初始零位、过期和无效样本、恢复、运动后立即观测及 Stop。真实所有者与工作进程 socket 测试覆盖 Run、采样推进、监控读取、运动、过期修订拒绝和 Stop。完整输出保存于被忽略的 `data/test/20261109-eos-arm-monitor/`。

## 监控健康状态与时序控制，2026-11-09

模拟机械臂目录公开可选高级设置 `sample_interval_ms`（默认 10）和 `freshness_timeout_ms`（默认 50）。Profile 限制两者范围，并要求新鲜度阈值至少为采样间隔的两倍。EOS 将有效值传给工作进程。所有者 Health 与 Status 保留工作进程的 `degraded` 或 `stale` 状态；Monitor 仍可读取，运动保持阻止。通过 `cargo test -p aether-node-profile --tests`、`cargo test -p aether-runtime --bin aether-profile --bin aether-runtime --test arm_activation`、`cargo clippy -p aether-node-profile -p aether-runtime --all-targets -- -D warnings`、`cargo fmt --all -- --check` 和 `python3 scripts/check_doc_links.py --all` 验证。完整输出位于忽略的 `data/test/20261109-eos-monitor-health/`。

## 异步演示验证（2026-11-10）

运行 `cargo fmt --all -- --check`、`cargo clippy -p aether-runtime --all-targets -- -D warnings`、`cargo test -p aether-runtime` 及 `python3 scripts/check_doc_links.py --all`，均通过。真实所有者/工作进程 socket 测试验证接收回执、结果查询、重复 ID、监控及 Stop；持久保留单元测试验证写入日志及单次运动。完整输出保存在忽略目录 `data/test/async-eos-20261110/`。
