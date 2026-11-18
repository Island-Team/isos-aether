# v2 分发树

<!-- doc-navigation:start -->
<details>
<summary>目录：eos/core/dispatcher</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [已路由连接的调度](scheduler_cn.md)

</details>
<!-- doc-navigation:end -->

[English](../../../../docs/eos/core/dispatcher/dispatcher.md) · [硬件设计](../hardware/hardware_cn.md) · [字段接口](../../interfaces/node/hardware/mock_cn.md#hardware_mock_cn--arm)

## 地址与职责


分发层是独立于 HAL 的 Rust 模块。发现操作读取由已校验节点 profile 构成的注册表快照；运动请求指定一个节点和一个类型化模式。分发层只解析该节点一次，并在交给适配器前检查 profile 声明的能力、实测健康、所需样本新鲜度、命令期限、数值限值及安全状态。适配器是物理操作的最后把关者。导航规划、轨迹生成和逆运动学不在该层。

软件包贡献的节点类型与内置类型使用相同的 Dispatcher 准入规则。仅有软件包安全声明而缺少获准的适配器及 Stop 实现，不能取得执行器路由。


## 命令生命周期

每个命令具有调用方提供的 `command_id` 和单调时钟期限。`Dispatcher::new_durable` 独占打开有界日志目录，在调用适配器前先同步写入带版本的预留记录，再写入 `releasing` 记录。重复 ID 返回已保存结果；相同 ID 对应不同命令时拒绝。重启后，未完成记录转为 `outcome_unknown`，绝不自动再次下发。损坏或过大的记录阻止启动。内存构造函数只用于测试与进程内原型；受管 Rust 运行时使用持久构造函数。释放前日志写入失败则拒绝命令；释放后终态写入失败则保留不确定结果。

急停先锁存进程安全状态，再调用各执行器独立的停止方法。分发层不再跨可能阻塞的适配器调用持有释放锁。适配器必须在物理写入边界锁存停止，使延迟的提交无法在停止后重新执行；CXX mock 实现并测试了该规则。mock 测试证明提交阻塞时停止仍能返回，但真实控制器的停止延迟与独立速度看门狗仍须在切换前测量。读取或样本校验失败会使该节点的 Rust 健康状态降级；只有 `refresh_node` 的有效且新鲜的探测才能清除故障。健康、安全和物理停止错误分别可见。

## 与现有运行时的关系和验证


用假 HAL 注册表测试精确路径解析、接口发现和节点隔离。集成测试覆盖正常运动、过期观测、限值、过期命令、跨重启重复 ID 重放、损坏日志拒绝、健康降级与恢复、释放前取消以及 mock 释放阻塞时的急停。真实硬件停止延迟、控制器执行的速度租约到期和部署中的 ROS 2 图仍是正式切换前的验收用例。可复现命令写入[开发指南](../hardware/hardware-guide_cn.md)，完整输出放在 `data/test/<run-id>/`。

按钮演示通过内置 `arm.delta` 原语准入 `arm_joint_delta@v0.2`。在模拟适配器更新状态之前，调度器拒绝单关节超过 0.25 rad 的增量，或让结果关节超出 ±3.14 rad 的命令。见[机械臂控制](../../interfaces_cn.md#arm_control_cn)。

演示监控返回路径独立于命令响应检查最新六关节观测。样本超过配置阈值（默认 50 ms）、关节为非有限数或位置超过配置限值时撤销运动就绪状态。100 Hz 采样器可在取得新的有效样本后恢复就绪。[私有监控读取](../../interfaces_cn.md#arm_monitor_cn)保持只读，不能绕过 Dispatcher 准入。

<a id="r03-执行器受治理准入设计"></a>

## R03 执行器受治理准入设计

### 准入与持久身份

每个公开运动入口构造一份规范命令：命令 ID、IsOS 部署 `run_id`、调用方角色、服务端证明的入口、保存图活动修订、primitive/版本、精确 binding 来源、精确硬件目标、类型化参数和单独校验的单调截止时间。同一个 Dispatcher 方法在任何可能物理释放前持久预留此身份。凭据字节和传输 request ID 不进入持久签名；稳定角色/范围与入口、IsOS `run_id` 及活动修订进入签名，单次尝试的期限不进入重复签名。同一 IsOS 部署运行内 EOS 子进程重启保留运行 ID 与授权，同一 ID 和规范请求返回保留状态。新 IsOS 部署运行轮换运行 ID 和授权；旧授权失败，新授权不能把旧运行结果当作本次结果查询，复用旧 ID 仅冲突而不释放；修订、角色、binding、目标或参数变化产生冲突，绝不释放。已授权所有者在新图活动后仍可查询旧操作，但新命令不能使用旧修订。已释放的 `OUTCOME_UNKNOWN` 对该 ID 是终态，不会自动重试。旧 journal 缺少新身份字段时应封存，不能从当前图推断。

停止记录持久保存同一所有者角色与运行 ID，以及不同的 `MotionStop`/`RobotStop`/`SafetyEstop` 种类、精确设备或机器人目标、保存修订（仅已认证 estop 可为空）和恢复标记。同 ID 跨角色停止或改变恢复标记产生冲突，不能返回另一角色的旧结果。Get/Watch/Cancel 要求 journal 角色/范围及同一运行；旧运行结果不得展示为本次成功。预留后，Dispatcher 检查释放协调器、安全锁存和 inhibit、精确 binding 与 adapter kind、活动修订、角色授权、节点生命周期与新鲜健康、必需的类型化观测、有限数值与配置单位，以及剩余单调期限。调用 adapter 前持久写入 `Releasing`。排队过期或释放前 journal 写入失败时 adapter 调用数为零。终态结果与同一命令 ID 持久保存；物理释放模糊、终态落盘丢失或 `Releasing` 后崩溃成为 `OUTCOME_UNKNOWN`，并在现有显式停止恢复证明静止前抑制运动。不能把传输超时或断线视作物理成功或取消。`motion.stop`、`robot.stop`、`safety.estop` 保持 R02 中不同的 journal 类型与保留 stop 通道。


### 验证边界

[冻结用例](../../../../tests/fixtures/r03_admission_revision/cases.json)覆盖受管理修订来源、缺失与过期修订、缺失与错误授权、精确 binding 与类型化界限、假时钟健康/观测/期限、阻塞队列重检、重复与冲突身份、journal 故障与重启、并发停止及所有旧绕过路径。单元测试使用假时钟和单次调用 adapter；边界测试使用真实 Rust listener/私有 socket 与 stub adapter。EOS 进程测试构造 schema-2 快照和验证文件夹具；合并后的 IsOS 签发及启动交付已通过单独的边界重放，记录于首次使用页。物理控制器去重和所选服务子进程停止/回收时序仍属 R05/R06，R03 不能从 mock 测试宣称这些属性。

### 受治理执行器准入：开发与验证

IsOS 为选定的图修订签发运行范围授权。EOS 在 Dispatcher 释放执行器命令前验证授权、活动绑定、健康、截止时间和调用方身份。属于其他运行或修订的请求会被拒绝。journal 为每个 operation ID 保存一份结果；响应丢失或结果不确定都不允许自动重新执行物理动作。

#### 首次使用重放

1. 构建 Rust 工作区，并使用包含独立的 `isos`、`ei`、`eos` core，节点类型为 `<core>.<layer>.<group>.<node>@v0.2` 的已验证图启动 IsOS。
2. 由 IsOS 启动 EOS，确认受管理授权和活动图修订只交给选定子进程。
3. 通过公开 EOS 接口提交有效的底盘或机械臂命令。用同一个 operation ID 读取直到终态；确认只有一条 journal 记录和一次适配器释放。
4. 用无效授权、调用方、目标、修订、负载及过期截止时间重试。每个请求必须在适配器释放前失败，也不得泄露其他调用方的结果。
5. 在命令待处理期间测试取消、进程故障、journal 故障和 Stop。验证释放围栏、持久结果分类，以及[归属方停止契约](../../../isos/interfaces/server/eos_cn.md#owner_health_cn)规定的物理停止顺序。

运行 `cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`、串行工作区测试、[用例检查](../../../../tests/fixtures/r03_admission_revision/README_cn.md)和 `python3 scripts/check_doc_links.py --all`。把完整输出保存在 `data/test/<run-id>/`，并记录准确修订、命令、环境和用例到测试的映射。

<a id="eos-本地日志"></a>

## EOS 本地日志

EOS 所有者和机械臂工作进程追加到同一个本地 JSONL 文件：受管部署使用 `data/eos/log/<deployment>/eos.jsonl`，独立运行使用 `data/eos/log/local/eos.jsonl`。IsOS 读取文件更新日志面板。所有者 API 返回生命周期和激活结果；日志记录留在本地。

每条记录包含 UTC RFC 3339 时间戳、小写级别、来源及消息。属于一次运行的记录还包含运行 ID。模拟机械臂来源是 `eos.hardware.mock.arm@aether-v0.2.3`；工作进程系统事件使用 `eos.aether_runtime.worker@aether-v0.2.3`。所有者观察到工作进程丢失时只记录一次，并记录失败的 Run 或 Stop 结果。命令的终态由激活回复证明，不能只依赖日志。

为调用方保留的旧 bridge 和 config-server 程序通过 `LocalCoreLog` 写入同一个文件中的有界记录。当前演示图不启动它们。运行 `cargo test -p aether-runtime --all-targets` 验证本地日志及所有者和工作进程生命周期；完整输出保存在忽略的 `data/test/<run-id>/`。
