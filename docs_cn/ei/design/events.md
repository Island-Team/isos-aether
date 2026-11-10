
[English](../../../docs/ei/design/events.md) · [归属接口](../interfaces/events.md) · [夹具](../../../ei/tests/fixtures/jsonl_events/README_cn.md)


## 映射和顺序



归属方本地最多 256 键的单调表只对重复的非关键诊断在 60 秒内抑制，发出有界计数。启动、就绪、状态变化、首次警告／故障、锁存、未知结果、终结和安全停止失败绕过抑制。当前 `_RepeatSuppressor` 对本地文件使用 4,096 项策略；封闭的生命周期发射器保留独立的类型字段与抑制规则。表淘汰和关闭时刷新安全计数，不阻塞停止。共享写失败增加有界归属计数；该失败不改变 EOS 致动状态。

## A03 持久恢复与共享策略修订


`OperationLedger` 在 EOS Submit 前原子预留 ID，传输不确定时保留待决条目。启动后只用同 ID 调用 `GetOperation`；匹配的终态收据结算检查点，缺失或不匹配的收据保持未知并阻止安全重置。`SafetyState` 在 estop 外发前将受影响 alias 和 EOS robot ID 集合写入仅所有者可读的原子文件并 fsync 目录。本机停止进程 CLI 使用与运行时同一把排他锁，写一次性授权；下次启动消费授权并两次检查所有 alias 后才持久解锁。


A03 独立实现审查修正：`safety-history.json` 先于锁存 payload 写入，曾锁存而 payload 缺失时关闭准入；`operations-history.json` 和每个操作索引先于操作 payload 写入，缺失文件会被检测。锁存和一次性授权都绑定 episode ID 与所选图 revision；操作预留还保存 EOS robot ID 与图 revision，终态不可重写。运行锁取得后立即消费授权，配置、发现或任务检查点失败也不会留下可重用授权。托管默认启动不会在恢复前清除未完成任务。

