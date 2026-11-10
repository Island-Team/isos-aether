# 托管例行关闭与持久安全状态

[归属方契约](../../../../docs/eos/interfaces/simulator-stop.md) · Dispatcher · M5 验证

## 问题与边界


实现应区分归属方请求的**托管例行关闭**与真正的 `safety.estop`。不新增公开重置操作、图字段、授权凭据、执行器入口或协议标识。真正的急停锁存保持持久；Stop、Restart、普通 `robot.stop` 或新的模拟器代际都不能清除它。崩溃、结果不明的运动释放、物理停止或子进程回收继续以拒绝运行为默认结果。

## 状态转换

1. 在停止执行器前，EOS 关闭运动和证据接纳，并设置 Dispatcher 内存抑制。托管例行路径记录已接纳的运动目标。日志增加绑定机器人的 `1.1.0` 版 `shutdown_pending` 标记及所选 `shutdown_targets`；待处理为真当且仅当目标非空。读取器先严格按原有体形状及校验和验证 `1.0.0` 控制记录，再赋予假/空默认值；仅在下次成功修改控制状态时升级体形状与校验和。重新打开时，待处理标记的目标须匹配已接纳的机器人和配置。该标记与现有 `robot_inhibited` 恢复状态及 `estop_latched` 分离；新 EOS 进程看到未解决标记时禁止运动。不得覆盖既有原因或把它视为本次关闭所创建。
2. 它在 R06 截止时间内启动现有的单请求 MuJoCo 伴随套接字停止协调器，然后才等待日志持久化。并发的公开 `robot.stop` 或 `safety.estop` 可共享物理请求，但保留各自持久身份与安全效果。例行路径必须在宣称物理停止成功前，以原子方式持久设置自己的关闭标记和目标集合。写入停滞或失败时，伴随停止仍须执行，所选终态为未知，不得声称标记已持久化。写入前 EOS 崩溃时，IsOS 的所选未知停止栅栏阻止替换。
3. 只有匹配的代际和更新的 epoch 同时证明底座和机械臂停止，EOS 才继续清空并回收所选 MuJoCo 进程组、移除后端与控制套接字、关闭宿主技能和证据服务，并检查全部既有 R06 回执条件与总期限。公开入口先于停止接纳结束而关闭；关闭前接纳的 `safety.estop` 必须持久锁存并完成，之后的调用被拒绝。期限内仍未解决的公开停止使所选回执为未知。任何部分证据都不足以形成确认回执。内部关闭的各部分结果由所选原始回执证明；只有公开停止操作拥有独立的各部分持久日志记录。
4. 仅在上述条件均成立且没有尚在执行的公开停止操作时，EOS 才可利用已证明的更新物理停止 epoch，持久栅栏终态结果不明的运动并保留其机器人抑制，再在同一日志锁下解除**本次关闭标记及其目标列表**。非终态或旧格式未解决运动仍阻止解除。写入失败时回执为未知。此操作绝不修改 `robot_inhibited` 或 `estop_latched`；两者均可独立保持为真。当前进程直至退出仍保持关闭和内存抑制，绝不恢复运动。随后只发布一次所选 stdout 原始回执；IsOS 仍须核验严格回执和 EOS 成功退出，才能启动新运行实例。
5. 任何条件未知时，关闭标记保持持久；迟到的适配器结果不得清除它。若真正的 `safety.estop` 在关闭前或关闭期间锁存，即使例行物理停止和子进程回收都已确认，锁存仍为真。新 EOS 运行实例继续把运动标为不可用，直到另行规定并授权安全恢复。


## 归属方实现的限定范围证据（2026-11-02）


从此工作树根目录运行以下可复现命令：

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace -- --test-threads=1
cargo build --workspace
python3 scripts/check_doc_links.py docs/eos/interfaces/simulator-stop.md docs/eos/design/process/mujoco-operations.md docs_cn/eos/interfaces/simulator-stop_cn.md docs_cn/eos/design/process/mujoco-operations_cn.md
git diff --check
```

均已通过；完整输出保存于 `data/test/20261102-managed-shutdown/`。此处不作吞吐量或延迟声称。
