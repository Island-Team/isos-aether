# 浏览器原语结果路由

[English](../../../../docs/eos/design/runtime/browser-results.md) · [所有者控制契约](../../interfaces/control_cn.md#r07-浏览器授权结果后续查询) · [原语绑定](../../interfaces/primitive-bindings_cn.md)

浏览器通过 IsOS 获取已保存和活动的图修订，并通过 loopback HTTP bridge 发送有界 `primitive_typed` 请求。EOS 在 Dispatcher 准备 journal 记录前验证运行范围授权、唯一活动目标、类型化负载、健康和修订。浏览器无法直接访问执行器，也无法读取其他调用方的结果。

## 提交与结果

受管理浏览器请求返回 `InProgress` 前，Dispatcher 先持久写入 `Prepared` 记录。一个有界工作线程经过现有释放门、写入 `Releasing`、调用适配器一次，并保存终态结果。受管理浏览器运动最多占用八个工作槽。相同 ID 且签名一致的请求读取已保存结果；签名变化则拒绝。终态持久写入失败会使该 ID 不可读、抑制运动并保留容量，直到封存或恢复。恢复时 `Prepared` 记录被取消且不会物理重放；`Releasing` 记录被归为不确定。

bridge 对公开长度提示和解析后的请求体施加 16 KiB 上限。私有字节数由 bridge 的紧凑 JSON 序列化生成，EOS 校验精确私有字节数。JavaScript 与 Rust 对数值的等价 JSON 表示因此可互通，同时保持负载上限。

UI 在收到关联 ID 和 EOS 状态前显示 `Sending`。只有相同图修订仍活动时，才读取待处理的 `InProgress` 结果。`Completed`、`Failed`、`Cancelled`、`Rejected` 和 `OutcomeUnknown` 是终态展示。bridge 故障、修订变化或运行时停止时展示结果不可用。IsOS Stop 始终可用；EOS 在终止直接管理的 MuJoCo 子进程前围栏运动并确认物理停止。

## 验证

使用延迟可控的受管理 mock，验证同一 ID 的 `InProgress`、授权待处理读取及后续终态读取。测试无效授权、Origin、目标、修订、请求体和重复签名，以及 journal 故障与恢复、待释放期间 Stop 和 bridge 故障。运行格式化、严格 Clippy、串行工作区测试、浏览器结果夹具及双语链接检查。将完整命令与输出保存在 `data/test/<run-id>/`。
