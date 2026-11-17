# 浏览器原语结果路由

<!-- doc-navigation:start -->
<details>
<summary>目录：eos/core/gateway</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [R02 Rust gRPC 网关设计](grpc-gateway_cn.md)

</details>
<!-- doc-navigation:end -->

[English](../../../../docs/eos/core/gateway/browser-results.md) · [所有者控制契约](../../../isos/interfaces/server/eos_cn.md#control_cn--r07-浏览器授权结果后续查询) · [原语绑定](../../interfaces_cn.md#primitive_bindings_cn)

浏览器通过 IsOS 获取已保存和活动的图修订，并通过 loopback HTTP bridge 发送有界 `primitive_typed` 请求。EOS 在 Dispatcher 准备 journal 记录前验证运行范围授权、唯一活动目标、类型化负载、健康和修订。浏览器无法直接访问执行器，也无法读取其他调用方的结果。

## 提交与结果

受管理浏览器请求返回 `InProgress` 前，Dispatcher 先持久写入 `Prepared` 记录。一个有界工作线程经过现有释放门、写入 `Releasing`、调用适配器一次，并保存终态结果。受管理浏览器运动最多占用八个工作槽。相同 ID 且签名一致的请求读取已保存结果；签名变化则拒绝。终态持久写入失败会使该 ID 不可读、抑制运动并保留容量，直到封存或恢复。恢复时 `Prepared` 记录被取消且不会物理重放；`Releasing` 记录被归为不确定。

bridge 对公开长度提示和解析后的请求体施加 16 KiB 上限。私有字节数由 bridge 的紧凑 JSON 序列化生成，EOS 校验精确私有字节数。JavaScript 与 Rust 对数值的等价 JSON 表示因此可互通，同时保持负载上限。


## 验证

使用延迟可控的受管理 mock，验证同一 ID 的 `InProgress`、授权待处理读取及后续终态读取。测试无效授权、Origin、目标、修订、请求体和重复签名，以及 journal 故障与恢复、待释放期间 Stop 和 bridge 故障。运行格式化、严格 Clippy、串行工作区测试、浏览器结果夹具及双语链接检查。将完整命令与输出保存在 `data/test/<run-id>/`。

<a id="检查浏览器原语结果"></a>

## 检查浏览器原语结果

IsOS 报告部署运行后，检查保存与活动修订以及 EOS 实时就绪。在 EOS 面板选择一个**活动原语绑定实例**。面板显示注册类型和实例描述、唯一图目标、运动类型与单位，以及所有者报告的目标健康。草稿绑定或相同显示名不能自行授权命令。经回环 bridge 获取 EOS 单调时间，携活动保存字节修订及新的命令 ID 提交类型化 `primitive_typed` 载荷；EOS 返回该 ID 和状态前，本地保持 `sending`。浏览器不能提供私有授权或独立硬件目标。

`InProgress` 确认必须发生在 `Prepared` journal 记录持久化之后，也可能早于适配器释放。它只证明 EOS 已保留命令 ID；EOS 在确认前检查类型化运动限制。若提交回复丢失，保留该 ID 并使用受保护的只读查询；绝不再次发送运动命令。EOS 同时最多接纳八条此类浏览器提交；有效的新 ID 遇到满载时获得持久化的 `Rejected` 结果。结果待定时仍可通过 IsOS 使用 Stop。

响应为 `InProgress` 时，仅在 IsOS 仍报告该修订活动时有界轮询公开 `result` 后续查询；bridge 将其转换为受保护的私有 `primitive_result`。显示 EOS 精确状态：`Completed`、`Failed`、`Cancelled`、`Rejected`、`InProgress` 或 `OutcomeUnknown`。`Failed` 保持终态失败。修订过期、绑定或载荷无效、Origin 被禁止、bridge 故障、旧运行结果或缺少操作员授权，都须显示明确失败或不可用；不能显示为 `Completed`。通过 IsOS Logs 找当前运行的首条就绪或故障事件，再仅按命令 ID 的关联值筛选 bridge 诊断，不增加节点筛选；该事件的 `node_path` 为 null。Vision 面板通过独立只读 WebSocket 读取帧，并拒绝过期的订阅修订。

**Q01 EOS 生产端已实现；IsOS 接纳及所选版本回放待完成：**bridge 在收到有效私有响应后，把脱敏结果事件写入当前运行。已发布的浏览器命令 ID 仍接受 1–128 字节 UTF-8。IsosUI 生成的 `ui-<UUID>` ID 原样写入 `correlation_id`；其他有效 ID 按[事件契约](../../../isos/interfaces/server/eos_cn.md#events_cn)映射成确定性令牌。普通轮转段仍在保留期时，查询 `GET /api/isos/v1/logs?view=current&source=bridge&correlation_id=<token>`。页面不受筛选的锚点是首条就绪或故障事件，不是命令结果。缺失日志行或收集缺口不改变 EOS Dispatcher 日志结果。

<a id="q01-浏览器命令事件设计"></a>

## Q01 浏览器命令事件设计

现有 P06 写入器发生非阻塞错误后会永久停止写入。同一桥接进程不重试丢失的诊断行，也不改变命令响应；所选关联日志缺失时，完整 Q01 验证失败。


**EOS 生产端已实现；IsOS 接纳及 Q01 回放待完成。**桥接器已将类型化浏览器控制和经授权的结果后续查询转发至私有运行时 IPC，验证成功结果的身份和状态，并发出脱敏事件。当前远端版本的 IsOS 封闭 P06 桥接角色仍只接收生命周期事件，因此 Q01 回放尚无法在 P07 当前运行日志中找到 UI 命令 ID。

桥接器保持已发布的 128 字节 UTF-8 浏览器 ID 接受规则。安全 ID 原样用作 P06 关联值；其他有效 ID 使用确定性不透明令牌，因此既保留同一运行的后续查询，也不记录任意文本。收到私有响应后，仅当 `ok` 为 true、结果 ID 与请求 ID 相同、状态属于已发布的 Dispatcher 枚举时，才发出结果事件。不能从 HTTP 200 或请求正文推断状态。有界状态缓存抑制重复的 `InProgress` 轮询及终态读取，同时保留提交结果和首次观察到的终态转换。共享的非阻塞事件写入器对客户线程的有界写入进行串行化；写入失败是诊断损失，不改变命令结果，也不能成为重放运动的理由。[归属方事件契约](../../../isos/interfaces/server/eos_cn.md#events_cn)定义令牌映射、全部字段和安全响应边界。

边界测试使用私有运行时桩，覆盖立即终态、`InProgress` 后 `Completed`、错误 ID 或状态、缺失授权、同一运行中 128 字节或非 ASCII ID、重复后续查询、持续轮询、缓存逐出、并发调用及事件写入失败。所选版本集成验证对照原始桥接事件、IsOS 规范化日志行、P07 页面、UI 显示 ID 和 EOS 单条日志结果。日志证明脱敏归属事件的关联和接纳；Dispatcher 日志仍是结果权威。

### EOS 生产端验证，2026-11-03

在本主题工作树运行 `cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test -p aether-common -p aether-http-bridge -- --test-threads=1`、`cargo test --workspace -- --test-threads=1`、`cargo build --workspace`、`python3 scripts/check_doc_links.py --all` 和 `python3 tests/fixtures/jsonl_events/check_events.py`，全部通过。完整输出保存在本地 `data/test/20261103-q01-browser-events/`。桥接器测试覆盖 UTF-8 命令在私有提交与后续查询间的流转、安全 ID 原样关联、缓存并发与逐出、矛盾响应拒绝和写入失败。该证据只验证 EOS 生产端；IsOS 日志接纳与 Q01 回放仍未完成。

独立审查发现一个并发顺序边界：终态后续查询可能先于原提交线程记录其响应。缓存现在分别跟踪首次提交与首次终态观察，即使顺序倒置也各发出一次。确定性的倒序测试固定这一边界。写入失败测试统计写入次数，并验证首次诊断写入失败后，两次成功的私有结果仍返回 HTTP 200。复测输出位于 `data/test/20261103-q01-browser-events-rereview/`。
