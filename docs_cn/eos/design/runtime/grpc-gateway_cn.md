# R02 Rust gRPC 网关设计

[English](../../../../docs/eos/design/runtime/grpc-gateway.md) · [所有者契约](../../interfaces/grpc_cn.md#r02-冻结的-rust-子集) · [首次使用重放](grpc-gateway-guide_cn.md) · [机器用例](../../../../tests/fixtures/r02_grpc_gateway/README_cn.md)


## 组合与发布

在 `aether-runtime` 增加有界 `tonic`/`prost` 服务模块，从现有 protobuf 文件生成 Rust stub。锁定生成器版本并验证 Google API 导入；以 descriptor set 对比既有 wire 身份，防止构建悄然修改字段号。私有 `aether-runtime` 进程仍由 IsOS 选择。CLI 增加默认 16200 的回环 gRPC 端口及可选、受所有者权限保护的 Unix 路径，两者均区别于现有私有 JSON 控制 socket。没有明确部署策略时拒绝通配绑定；重复或不安全 socket 路径在就绪前失败。

启动顺序：profile/注册表和生成配置锁定验证 → 排他持久 journal → adapter 和直接 MuJoCo 子进程 → 不可变 Gateway 投影 → gRPC 绑定 → 聚合 health `SERVING` 及既有所有者 ready 事件。当前实现遇到不完整准入会在绑定前失败，不会发布准入前 listener 或 ready 事件。若将来先绑定，health 必须保持 `NOT_SERVING`，所有必需方法返回 `UNAVAILABLE`。投影保存机器人身份、设备路径映射、精确控制 binding、资源注册、能力目录和 `started_at`，在 listener 发布前只构建一次；R02 没有进程内图刷新或 `Arc` 交换。`GetStatus` 叠加实时健康，不混合不同代际身份。失败的候选代际不能替换当前完整投影或发布半个目录；实时健康可独立降级。停机前先把 health 设为 `NOT_SERVING`，拒绝新写入并排空有界 stream。私有控制 socket 和浏览器桥仍是同一 Runtime/Dispatcher 的调用方，不另建执行器路线。


本进程只读已 Apply 图；新图代际需要进程重启，因此四个启动发现调用不会跨越进程内图替换。实时健康和能力可用性可改变，但 binding 身份不变。若以后支持热刷新，必须完整发布代际并提供客户端重试边界，不能在多个独立 RPC 间返回混合目录。

已实现的 Rust 投影为每个已准入投影使用一个后台工作线程采样 HAL 健康及生命周期状态。采样器同时最多有一次 adapter 探测，因此阻塞的所有者 socket 不会占用 tonic 工作线程，也不会随读取负载复制探测。投影构建最多等待 200 ms 获得初始完整样本；样本缺失时报告不可用，但已准入身份及 listener 保持完整。发现、状态、能力列表与相机预检查只读取保留样本。每次探测在调用健康和状态接口之前记时；缓慢的状态响应不能刷新旧健康。失败、缺失或超过 `min(profile stale_after_ms, 1000 ms)` 的样本使可用性降级；新鲜探测恢复可用性。完整新鲜投影以 `eos-sample-age-ms` 给出最旧节点样本的年龄，缺失或过期时省略该字段。ReadResource 和 CaptureFrames 使用保留产品或帧的年龄。停机阻止采样器之后再发布，无需等待阻塞 adapter 调用；Dispatcher 在写入释放前仍重新检查物理准入。gRPC 工作许可为十六个运动、一个受控停止及一个紧急停止分别保留独立路线；运动工作线程饱和不能占用两条停止路线。

## 读取与有界 stream


`CaptureFrames` 一次解析已准入相机设备 ID，校验机器人名称和实时健康，从 EOS 自有两帧缓存读取 1 或 2 个不同的新鲜帧。使用已有 `VisionFrame::encode_jpeg` 或已校验 JPEG preview；检查 SOI/EOI、尺寸、时间戳及不同采集身份。响应 ≤16 MiB，每张 JPEG ≤8 MiB。缓存等待及编码均遵守截止时间和取消；超时不能留下无界任务。缓存不可用返回 `UNAVAILABLE`，响应过大返回 `RESOURCE_EXHAUSTED`。

## 操作与持久身份




同一 journal 保存已同步的 RobotStop inhibit 控制状态；它在 Releasing 前设置，所有运动释放（包括并发 Submit）都要检查。只有不可变快照中每个部件及当前全部已准入可停止部件都确认终态停止后，才能持久清除。已知部分失败、不确定释放、崩溃或清除写入失败均保持 inhibit。重启后部分扇出记录变为 `OUTCOME_UNKNOWN`，原 ID 只可查询。新的、按本地策略授权的 RobotStop ID 可通过快照尚未解决的旧目标与当前已准入可停止节点的并集恢复；确认所有目标后才清除。旧目标无法验证时保持 inhibit，直到显式授权维护提供等价逐目标停止证据。紧急 estop latch 独立存在，不能通过此恢复清除。inhibit 活动时，完整 gRPC 接口可为安全发现和停止恢复报告 health `SERVING`；`GetStatus.status` 与机器人 health 为 `degraded`，运动能力不可用，运动 Submit 在释放前拒绝。Stop 能力仍可用。这里的服务可用不代表物理运动就绪。

将释放协调器实现为 Dispatcher 所有、写方优先的运动许可屏障，与 journal 互斥锁和紧急停止 latch 分离。运动先取得许可，在屏障下检查 inhibit/截止时间，再调用 `begin_release` 持久写入 `Releasing`，直到 adapter 调用及其终态或物理边界隔离完成才释放许可。RobotStop 原子关闭新运动许可后持久设置 inhibit；inhibit 同步完成后等待所有旧许可结束，包括已写入 `Releasing` 但在 adapter 前暂停的运动，随后才扇出。等待或调用 adapter 时不持有 journal 互斥锁。调用返回 `Accepted`、`Ambiguous` 或可能已物理释放后的错误，即使线程结束也留下未解决代次。停止路线必须获得 adapter 证明的静止且无待执行命令，或在物理写入边界隔离该代次，才可宣称终态停止；否则在有界截止时间后成为 `OUTCOME_UNKNOWN`，inhibit 持久保留，不能凭一次已停观测清除。崩溃后从 journal 重建未解决的 `Releasing` 运动，在宣布运动可用或清除任何 RobotStop inhibit 前建立 adapter 代次隔离或等价证明。独立的 `SafetyEstop` 路线仍可抢先执行，不等待该屏障。确定性测试在运动同步写入 `Releasing` 后、调用 adapter 前暂停，启动 RobotStop，确认停止扇出与清除均不能越过该运动，再释放运动并确认有序停止；另一分支使运动结果模糊，核对 inhibit 与 unknown 恢复。

在 Prepared 前校验最多 64 个目标的快照，并按每个部件 256 字节原因的上限预留有界结果空间；实际原因在返回时脱敏并截断。若 Releasing 前序列化超限，返回 `RESOURCE_EXHAUSTED`，adapter 调用数为零且不改变 inhibit。若可能已经调用后写入或结果序列化失败，保留 `OUTCOME_UNKNOWN` 和 inhibit，不重试该 ID，且只保留已持久确认的逐部件结果。并发运动在扇出中、释放前注入溢出与可能释放后注入溢出须分别测试。

通过一个纯投影将 `DispatchResult` 转为 v2 Operation 和规范 `primitive.ack`，包括匹配的 ID、primitive/version、accepted、status、sequence 和有界 result。不可从现有未区分的运动结果编造 `FAILED` 或 `TIMED_OUT`；R02 stop 命令有自己的已确认 `FAILED` 结果，R03 可增加可辨的运动原因。`WatchOperation` 先发当前记录，再最多发一个终态。watch 断开不取消或重试命令；恢复使用相同 ID `GetOperation`。`CancelOperation` 经 Dispatcher 定向取消，不能停止同设备的后续命令。显式 stop 能力和关闭路径负责钳制运动。现有同步 Dispatcher 可立即返回终态，watch 契约仍支持未来 pending adapter。

