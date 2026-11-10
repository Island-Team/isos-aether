# EOS 控制准入

> 历史契约：当前按钮到模拟机械臂图未实现此接口。当前数据及命令路径参见[机械臂控制](arm-control_cn.md)；设置、Run、Status 和 Stop 参见[所有者生命周期](owner-health_cn.md)。


[English](../../../docs/eos/interfaces/control.md) · [原语绑定](primitive-bindings_cn.md) · [Rust 技能契约](skills/rust_cn.md)

当前图包含独立的 `isos`、`ei` 和 `eos` core，并使用[机械臂控制](arm-control_cn.md)中的所有者激活路线。旧回环原语绑定路线通过 `isos.eos_dispatcher.primitive_base@v0.2` 与 `isos.eos_dispatcher.primitive_manipulator@v0.2` 绑定准入运动。每个绑定经类型化边指向一个兼容的硬件节点。Dispatcher 是唯一执行器路线。Rust 技能工作进程经同一代理请求原语；图不编排动作序列。

## 回环 API


HTTP bridge 在 `127.0.0.1:16020` 监听，并通过私有 Unix 套接字把有界 JSON 转发给 EOS。请求使用 `POST /api/eos/v2/control` 和 `Content-Type: application/json`；请求体上限 64 KiB。浏览器 `Origin` 只允许 `http://127.0.0.1:<ISOSUI_PORT>` 和 `http://localhost:<ISOSUI_PORT>`（默认 16001）、`http://127.0.0.1:<ISOS_HTTP_PORT>`（默认 16000）、`https://www.os-island.com` 或 `https://os-island.com`；bridge 回应获准的 CORS/私有网络预检。bridge 的允许列表对浏览器及本地 HTTP 调用方均排除原始执行器、结果和 shutdown 路径；EOS R03 私有 socket 也拒绝上述旧路径，仅所有者可用的内部 `shutdown` 仍保留。请求必须访问回环 bridge 地址。

公开 `primitive_typed` 请求中的 `header.payload_length_bytes` 由浏览器给出，表示紧凑 UTF-8 载荷长度，必须为 1–16,384 字节的正整数。bridge 解析有界公开 JSON，要求该字段和载荷均存在，拒绝声明长度或解析后长度超出 1–16,384，再把私有请求头的长度改写为解析载荷经 `serde_json` 紧凑序列化后的字节数，随后才通过 Unix IPC 发送。公开声明是跨序列化器的有界提示，不是精确字节证明；私有请求头才是权威长度。私有 runtime 仍要求该长度与自身解析载荷的序列化长度精确一致，并在 Dispatcher 前拒绝不合法类型化命令。JavaScript 和 Rust 对某些有限 JSON 数字的表示不同，例如 `0.000001` 与 `1e-6`，因此浏览器不能由墙上时间或字符串字符数推断私有字节。公开长度缺失、非整数、越界或载荷缺失时，在读取授权前返回 HTTP 400 和 `{"ok":false,"error":"invalid typed request"}`。bridge 不改动类型化载荷值、图绑定、目标、修订、命令 ID、截止时间或授权校验。

## R07 受管理浏览器提交

**R07 限定范围实现已合并并通过资格验证；证据记于共享 TODO。** 对经过授权的受管理 `primitive_typed` 请求，只有 Dispatcher 已持久化预留该精确命令且取得一个有界执行槽位后，EOS 才能返回带关联命令 ID 的 `InProgress` `DispatchResult`。不增加公开字段或操作。bridge 现有的两秒私有 I/O 界限及浏览器四秒超时约束这次确认，而不约束后续物理调用。若校验、准入或执行在回复前结束，EOS 也可返回精确终态，浏览器必须照实显示。传输超时或确认丢失表示**提交结果不可用**，绝不允许重新发送运动命令。浏览器保留已生成的 ID，仅能在同一当前 IsOS 运行和活动修订下发起经授权的只读 `result` 查询。`InProgress` 确认只证明持久化预留，不证明已下发到执行器或物理完成。

EOS 最多接纳八个可能返回或已返回 `InProgress` 的受管理浏览器运动提交；等待释放闸门和正在调用适配器的工作线程都计入。此上限与私有 socket 客户端数及安全停止容量分离，繁忙的运动通道不能占用安全路径。对于新 ID，EOS 在预留前检查精确活动绑定、目标、类型化载荷限制、授权、运行、修订和当前截止时间，取得槽位并持久化写入 `Prepared`，随后才可确认 `InProgress`。Dispatcher 在最终释放闸门再次检查实时准入，在唯一一次适配器调用前持久化 `Releasing`，并在释放槽位前持久化一个终态 `Completed`、`Failed`、`Cancelled`、`Rejected` 或 `OutcomeUnknown`。八个槽位都被占用时，EOS 对合法新 ID **以一次原子 journal 插入**记录包含完整命令签名、受管理身份、`phase:Terminal` 和 `accepted:false` 的终态 `Rejected`；没有中间 `Prepared` 记录、工作线程或适配器调用。Dispatcher 已将这一独立的崩溃安全转换实现为 `Journal::insert_terminal_rejection`，与 `Prepared` 插入分离。若连拒绝也无法持久化，则返回 `{"ok":false,"error":"movement capacity unavailable"}`，既不确认也不下发命令。截止时间过期或准入不合法必须在物理释放前拒绝。异步路径仍进入同一个 Dispatcher；不建立第二条执行器路线，也不改变 gRPC 运动语义。

对于已预留的 ID，EOS 在计算容量前于 journal 锁下比较包含所提交单调时钟截止时间在内的完整持久化受管理身份及精确类型化参数。完全相同的请求只返回已有 `InProgress` 或终态，不启动第二个工作线程，也不再次调用适配器；身份或参数不同则返回 `Rejected`，不下发。重复请求仍须通过当前授权、运行、修订和活动绑定校验。预留后若无法启动工作线程，EOS 在同一 journal 锁下检查当前阶段：对仍为 `Prepared` 的记录，在任何确认前持久化封闭为终态 `Rejected`；若 Stop 已先将其封闭为终态 `Cancelled`，则返回该取消结果，绝不覆盖。若必要的终态写入失败，EOS 返回不可用错误、禁止后续运动，并保留 journal 供恢复。工作线程槽位由当前代际持有，直到终态持久化，或关闭已封锁后续释放并分类未决调用；分离的工作线程在存活期间仍持有旧 journal 和适配器的所有权，不能进入新代际。

IsOS 请求 Stop 时，EOS 进入 R06 的 `Closing`：关闭新的运动准入、注册停止写入方，并在等待普通运动工作线程或其 journal 锁之前启动抢占式物理停止。浏览器工作线程在 `Prepared` → `Releasing` 持久化转换之前，须在 journal 锁下再次检查关闭状态及自身记录阶段；在 `Closing` 前通过该检查的工作线程保留已捕获的 R06 代际/停止 epoch，之后不能取得新的 token。抢占式停止已排定后，EOS 可在同一 journal 锁下把仍处于 `Prepared` 的命令持久化封闭为终态 `Cancelled`；工作线程遇到该终态就不能调用适配器。不能仅因请求 Stop 就把已处于 `Releasing` 的命令报告为 `Completed`。若 MuJoCo 双部件停止获得确认，R06 代际/停止 epoch 栅栏前进并阻断旧 token 下发；已准入 mock 适配器使用锁存停止。若 MuJoCo 停止丢失或结果未知，epoch 可能没有前进，已在途的旧工作线程仍可能在子进程终止前到达旧子进程：EOS 保持准入关闭和未知回执，在 R06 期限内终止并回收该子进程。该线程仅保有旧 EOS 进程的适配器/会话及已捕获的代际，不得以新代际重新连接；新的 IsOS 运行启动独立的 EOS 进程及子进程。终止子进程是清理，不是物理停止已确认或运动结果已知的证明。阻塞的工作线程不能延迟 R06 物理停止通道或有界的子进程清理。崩溃或重启时，持久化 `Prepared` 恢复为 `Cancelled`，`Releasing` 恢复为 `OutcomeUnknown` 且运动禁用，两者都不重放。新的 IsOS 运行轮换授权，即使旧终态仍保留在 journal 中，浏览器也无法查询旧 ID。UI 仅在新的 IsOS 状态证明捕获的运行及修订仍活动时，才通过经授权的 `result` 路径跟踪 `InProgress`；Stop、重启、旧运行或过期修订终止该查询，且不得捏造完成结果。

若物理下发可能已经发生，但终态 journal 写入失败，EOS 锁存运动禁用，保留工作线程槽位直到持久化封闭或当前代际退出；物理调用仍存活时，保持旧 journal/适配器所有权，并将该 ID 的浏览器读取标为**结果不可用**，不返回过期的 `InProgress` 或未经持久化的终态猜测。此时不能准入新运动。同一运行中的后续相同 ID 请求不能启动另一线程，只能得到相同的不可用分类。仅当旧物理调用已返回、已确认的停止 epoch 将其阻断，或未知停止清理已回收其旧子进程后，EOS 才可把原始 `Releasing` 记录持久化封闭为 `OutcomeUnknown`；成功后，经授权读取返回该持久化未知终态。若封闭仍失败，在线读取保持不可用，关闭结果只来自 R06 独立的物理停止证明，绝不伪造运动成功。进程重启时，现有持久化 `Releasing` 恢复必须在就绪前写入终态 `OutcomeUnknown` 并保持运动禁用；恢复写入失败则启动失败。故障注入测试须覆盖物理下发前后终态写入失败、读取及槽位行为、迟到适配器回复，以及不重放的恢复。

若排队的工作线程在进入 `Releasing` **之前**判定截止时间过期、健康失败或已进入 `Closing`，但终态写入失败，持久化记录仍为 `Prepared`。EOS 对该 ID 封锁 `Releasing` 及所有适配器调用，在当前代际锁存运动禁用，保留槽位直到持久化封闭或代际退出；经授权结果查询及完全相同的重复提交返回 `{"ok":false,"error":"result unavailable"}`，不能返回过期的 `InProgress`，重复提交也不能启动线程。EOS 在 journal 锁下重试持久化终态：截止时间或健康拒绝为 `Rejected`，Stop 为 `Cancelled`；若发生竞争，Stop 的 `Cancelled` 优先。成功封闭后释放槽位，授权查询可读取该终态。若代际退出前仍无法封闭，正常 journal 恢复须在就绪前将持久化 `Prepared` 转为终态 `Cancelled`，且不下发；恢复持久化失败则启动失败。对截止时间、健康和 Stop 拒绝注入这种释放前终态写入故障，测试过期读取不透明性、重复请求不下发、槽位保留、持久化重试及重启恢复。

## R07 浏览器授权结果后续查询

**Q01 目标设计——尚未实现：**浏览器 `primitive_typed` 与公开 `result` 继续接受已发布的 1–128 字节 UTF-8 `command_id`。选定桥接器在核对匹配且有效的私有结果后发出脱敏的[浏览器命令事件](events_cn.md)。不以保留前缀 `h_` 开头的安全 1–64 字节 ASCII ID，原样用作 P06/P07 `correlation_id`；其他有效 ID 按已记录的确定性规则映射为不透明令牌。桥接器不拒绝先前有效的 ID，也不把任意文本复制到日志。


| 请求字段 | 类型 | 单位 | 用途、缺省与校验 |
| --- | --- | --- | --- |
| `op` | 字符串 | — | 必填且精确为 `result`；无缺省。 |
| `command_id` | 字符串 | 命令身份 | 必填，1–128 UTF-8 字节，与已提交的类型化请求头一致；无缺省。 |
| `active_revision` | 字符串 | 保存图 SHA-256 | 必填 64 位小写十六进制，须等于 EOS 当前的 IsOS 活动修订；无缺省。 |

```json
{
  "op": "result",
  "command_id": "move-1",
  "active_revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
}
```

私有 `primitive_result` 请求只有 `op`、`command_id`、`active_revision` 和 `authority_grant` 四个键。前三者沿用公开字段约束，但私有 `op` 必须等于 `primitive_result`；`authority_grant` 是仅由 bridge 提供的必填 64 位小写十六进制 bearer 字符串，浏览器无缺省值。私有 runtime 将其验证为 `OperatorControl`，核对活动修订，再通过同一 journal 锁原子核对当前运行、记录中的活动修订、角色、`Movement` 类型和 `PrivateBridge` 入口，然后返回 `DispatchResult`。现有 `gateway_operation_authorized` 只核对运行和角色，不公开入口；R07 增加专用的私有绑定结果查询，不从原语名称推断来源。不存在的 ID、其他角色或运行、gRPC 运动、旧记录以及停止 ID 均返回同一有界 `{"ok":false,"error":"unknown command ID"}`，不暴露具体原因。查询不会调用适配器或重放命令。

```json
{
  "ok": true,
  "result": {
    "command_id": "move-1",
    "status": "InProgress",
    "reason": null
  }
}
```

成功响应沿用 `DispatchResult` 形状：`command_id` 是相同的有界身份，`status` 精确为 `Completed`、`Failed`、`Cancelled`、`Rejected`、`InProgress` 或 `OutcomeUnknown`，`reason` 为 null 或 UTF-8 字符串。读取路径拒绝超过 512 UTF-8 字节的原因，也拒绝完整序列化 `{"ok":true,"result":...}` 响应超过 4 KiB；不会截断或改写持久 journal 记录。两者均返回 `{"ok":false,"error":"result unavailable"}`。bridge 还以 HTTP `502` 和同一固定错误拒绝超过 4 KiB 的私有结果响应。`InProgress` 仍非终态；`Failed` 是终态失败；`OutcomeUnknown` 不能转为成功。新的 IsOS 运行轮换授权后，旧命令不可查询。UI 只在保存修订与活动修订一致时轮询；bridge 故障或 runtime 停止须显示不可用，部署 Stop 仍走现有 IsOS 受治理路径。R06 另行确认物理停止，之后才展示 MuJoCo 运动。

公开结果路径对畸形或额外公开键返回 HTTP `400` 和 `{"ok":false,"error":"invalid result request"}`；对伪造的 `authority_grant`、无效 Host 或禁止 Origin 返回 HTTP `403` 和现有 bridge 错误。bridge 授权缺失或不可读取时，在 IPC 前返回 HTTP `503` 和 `{"ok":false,"error":"operator control unavailable"}`。EOS 拒绝私有授权时返回 HTTP `200` 和 `{"ok":false,"error":"operator control grant required"}`；过期修订返回 HTTP `200` 和 `{"ok":false,"error":"stale active revision"}`；不透明 ID 返回 HTTP `200` 和 `{"ok":false,"error":"unknown command ID"}`。私有 IPC 传输失败时，bridge 返回 HTTP `502` 和 `{"ok":false,"error":"runtime unavailable"}`。这些结果路径错误均是有界固定字符串，不回显私有授权或 journal 数据。
