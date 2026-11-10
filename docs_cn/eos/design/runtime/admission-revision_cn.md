# R03 执行器受治理准入设计

[English](../../../../docs/eos/design/runtime/admission-revision.md) · [所有者契约](../../interfaces/grpc_cn.md#r03-执行器受治理准入目标契约) · [硬件契约](../../interfaces/hardware_cn.md) · [首次使用重放](admission-revision-guide_cn.md) · [冻结用例](../../../../tests/fixtures/r03_admission_revision/README_cn.md)

**R03 经合并后的 EOS 与 IsOS 所有者验收。** R03 扩展已合并的 R02 网关和现有持久 Dispatcher。不增加 protobuf 服务，不执行图连线，不实施 R04 宿主技能或 R07 浏览器结果传输，也不宣称 R05/R06 的 MuJoCo 物理隔离与停止顺序。IsOS 拥有保存图字节及活动修订；EOS 拥有已准入投影、实时健康、命令准入和 journal。

## 准入与持久身份

每个公开运动入口构造一份规范命令：命令 ID、IsOS 部署 `run_id`、调用方角色、服务端证明的入口、保存图活动修订、primitive/版本、精确 binding 来源、精确硬件目标、类型化参数和单独校验的单调截止时间。同一个 Dispatcher 方法在任何可能物理释放前持久预留此身份。凭据字节和传输 request ID 不进入持久签名；稳定角色/范围与入口、IsOS `run_id` 及活动修订进入签名，单次尝试的期限不进入重复签名。同一 IsOS 部署运行内 EOS 子进程重启保留运行 ID 与授权，同一 ID 和规范请求返回保留状态。新 IsOS 部署运行轮换运行 ID 和授权；旧授权失败，新授权不能把旧运行结果当作本次结果查询，复用旧 ID 仅冲突而不释放；修订、角色、binding、目标或参数变化产生冲突，绝不释放。已授权所有者在新图活动后仍可查询旧操作，但新命令不能使用旧修订。已释放的 `OUTCOME_UNKNOWN` 对该 ID 是终态，不会自动重试。旧 journal 缺少新身份字段时应封存，不能从当前图推断。

停止记录持久保存同一所有者角色与运行 ID，以及不同的 `MotionStop`/`RobotStop`/`SafetyEstop` 种类、精确设备或机器人目标、保存修订（仅已认证 estop 可为空）和恢复标记。同 ID 跨角色停止或改变恢复标记产生冲突，不能返回另一角色的旧结果。Get/Watch/Cancel 要求 journal 角色/范围及同一运行；旧运行结果不得展示为本次成功。预留后，Dispatcher 检查释放协调器、安全锁存和 inhibit、精确 binding 与 adapter kind、活动修订、角色授权、节点生命周期与新鲜健康、必需的类型化观测、有限数值与配置单位，以及剩余单调期限。调用 adapter 前持久写入 `Releasing`。排队过期或释放前 journal 写入失败时 adapter 调用数为零。终态结果与同一命令 ID 持久保存；物理释放模糊、终态落盘丢失或 `Releasing` 后崩溃成为 `OUTCOME_UNKNOWN`，并在现有显式停止恢复证明静止前抑制运动。不能把传输超时或断线视作物理成功或取消。`motion.stop`、`robot.stop`、`safety.estop` 保持 R02 中不同的 journal 类型与保留 stop 通道。


## 验证边界

[冻结用例](../../../../tests/fixtures/r03_admission_revision/cases.json)覆盖受管理修订来源、缺失与过期修订、缺失与错误授权、精确 binding 与类型化界限、假时钟健康/观测/期限、阻塞队列重检、重复与冲突身份、journal 故障与重启、并发停止及所有旧绕过路径。单元测试使用假时钟和单次调用 adapter；边界测试使用真实 Rust listener/私有 socket 与 stub adapter。EOS 进程测试构造 schema-2 快照和验证文件夹具；合并后的 IsOS 签发及启动交付已通过单独的边界重放，记录于首次使用页。物理控制器去重和模拟器停止/回收时序仍属 R05/R06，R03 不能从 mock 测试宣称这些属性。
