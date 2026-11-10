
[English](../../../docs/ei/interfaces/safety-reset.md) · [事件](events.md) · [首次使用](../start/quick-start_cn.md)


## 仅所有者可访问的状态


| 字段 | 类型／单位 | 用途及验证 |
| --- | --- | --- |
| `identities` | alias → EOS robot ID 对象／无 | 非空受影响别名集合；键为 1–64 个 `[A-Za-z0-9_-]`，robot ID 字符串最多 128 字节；空 ID 表示未知，不能通过重置。 |
| `episode_id` | 32 位小写十六进制／锁存期 | 每次新的锁存产生新值；持久 marker、锁存和一次性授权必须精确一致。 |
| `graph_revision` | 64 位小写 SHA-256／所选图 | 锁存与授权绑定当前 IsOS 所选图；独立锁存测试使用全零哨兵值。操作检查点保存所选修订或 `standalone` 字面量。 |
| `nonce` | 32 位小写十六进制／无 | 一次性重置请求身份；下次启动消费，不重用。 |
| `operation_id` | 字符串／EOS 幂等键 | 1–64 个 `[A-Za-z0-9_-]`；在 gate 外发前生成或验证，在 Submit 前持久保存。 |
| `alias`、`primitive`、`device_id`、`mission_id` | 字符串／归属身份 | 精确恢复目标；`mission_id` 可为 null。原语和设备字符串最多 128 字节，别名和任务 ID 使用不透明 ID 语法。 |
| `eos_robot_id` | 非空字符串／EOS 身份 | 操作预留时记录；恢复查询前须与完整 EOS 发现一致。 |
| `state`、`outcome` | 封闭字符串／无 | 操作为 `pending` 或 `terminal`；终结结果为 `succeeded`、`failed`、`cancelled`、`rejected`，且须有匹配 EOS 收据。 |



仅测试使用的私有 `A03_EVIDENCE_DIR` 钩子在各工作进程退出后复制原始所有者状态，标签为 `positive-before`、`positive-after`、`negative-before`、`negative-after` 和 `negative-refused`。每个新快照目录权限为 `0700`；复制的文件以及记录哈希、字节数和权限的确定性清单均为 `0600`。精确文件集为 `operations-history.json`、所选操作的 `.index` 和 `.json`；仅后两个 negative 快照增加 `safety-history.json` 和 `safety-latch.json`。`negative-after` 在写入重置授权前复制。缺失、符号链接、错误类型、错误所有者或错误权限的源文件会导致失败；钩子绝不复制 `safety-reset.json`、grant 字节或其他所有者状态。未设置环境变量时不创建证据目录，也不影响运行时。


持久 `safety-history.json` 标记曾经锁存及当前 episode；曾锁存而锁存文件缺失时启动失败。`operations-history.json` 保留单调递增的 `reserved_count`，它先于各 ID 的 `.index`（含唯一且从 1 开始的 `sequence`）和数据文件写入；终态索引仍保留。即使其他操作仍在，缺失任一检查点或索引时启动也失败。终结检查点不可改写。一次性授权在运行锁取得后、配置／发现／任务检查点读取前消费，即使本次启动早期失败也不会留到下一次使用。托管启动即使 `mission_preload=false` 也会恢复有效任务检查点或关闭准入；不会清除先前未完成任务。

运行本机重置命令前，从仅所有者可读的 `safety-history.json` 取得 `episode_id`，并从 IsOS 已应用图取得当前所选修订。修订改变或产生新锁存期后必须重新取得操作员授权，不能重用旧请求。
