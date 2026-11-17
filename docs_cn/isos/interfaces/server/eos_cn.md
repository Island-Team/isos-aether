# IsOS–EOS server interface

<!-- doc-navigation:start -->
<details>
<summary>目录：isos/interfaces/server</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [IsOS 所有者服务端接口](README_cn.md)
  - [IsOS–EI server interface](ei_cn.md)

</details>
<!-- doc-navigation:end -->

## 目录

- [EOS 配置投影](#configuration_cn--eos-配置参考)
- [EOS 控制准入](#control_cn--eos-控制准入)
- [EOS 本地 JSONL 日志契约](#events_cn--eos-本地-jsonl-日志契约)
- [EOS gRPC 与所有者 IPC](#grpc_cn--eos-grpc-与所有者-ipc)
- [EOS 软件包节点贡献](#node_contributions_cn--eos-软件包节点贡献)
- [EOS 所有者生命周期](#owner_health_cn--eos-所有者生命周期)
- [软件包观测运行时](#package_observer_cn--软件包观测运行时)
- [所有者软件包进程交接](#package_process_cn--所有者软件包进程交接)
- [EOS 读取器身份](#reader_features_cn--eos-读取器身份)
- [EOS 设置交换](#setup_v1_cn--eos-设置交换)


<a id="configuration_cn"></a>
<a id="configuration_cn--eos-配置参考"></a>
## EOS 配置投影

[English](../../../../docs/isos/interfaces/server/eos.md#configuration) · [IsOS 图](../../../node/interfaces/graph_cn.md#instance) · [EOS 生命周期](eos_cn.md#owner_health_cn)

IsOS 仅向 EOS 提供已保存图中选定的 `cores.eos` 投影及精确的已验证软件包固定版本。EOS 只读投影，不修改它。每个已注册 EOS 类型声明位置、设置、端口、能力和生命周期；实例值与类型化边选择设备和路线。IsOS 在启动前拒绝未知固定版本或资源；EOS 在报告就绪前验证自身路线及适配器语义。

原语输出恰好连接到一个兼容的执行器输入。Dispatcher 在释放前检查活动修订、目标、能力、健康、界限、期限和 Stop 状态。相机路线只读。生成文件及进程启动见 [EOS 设计](../../../eos/core/architecture_cn.md)。

<a id="control_cn"></a>
<a id="control_cn--eos-控制准入"></a>
## EOS 控制准入

> 历史契约：当前按钮到模拟机械臂图未实现此接口。当前数据及命令路径参见[机械臂控制](../../../eos/interfaces_cn.md#arm_control_cn)；设置、Run、Status 和 Stop 参见[所有者生命周期](eos_cn.md#owner_health_cn)。


[English](../../../../docs/isos/interfaces/server/eos.md#control) · [原语绑定](../../../eos/interfaces_cn.md#primitive_bindings_cn) · [Rust 技能契约](../../../eos/interfaces_cn.md#skills_rust_cn)

当前图包含独立的 `isos`、`ei` 和 `eos` core，并使用[机械臂控制](../../../eos/interfaces_cn.md#arm_control_cn)中的所有者激活路线。旧回环原语绑定路线通过 `isos.eos_dispatcher.primitive_base@v0.2` 与 `isos.eos_dispatcher.primitive_manipulator@v0.2` 绑定准入运动。每个绑定经类型化边指向一个兼容的硬件节点。Dispatcher 是唯一执行器路线。Rust 技能工作进程经同一代理请求原语；图不编排动作序列。

<a id="control_cn--回环-api"></a>
### 回环 API


HTTP bridge 在 `127.0.0.1:16020` 监听，并通过私有 Unix 套接字把有界 JSON 转发给 EOS。请求使用 `POST /api/eos/v2/control` 和 `Content-Type: application/json`；请求体上限 64 KiB。浏览器 `Origin` 只允许 `http://127.0.0.1:<ISOSUI_PORT>` 和 `http://localhost:<ISOSUI_PORT>`（默认 16001）、`http://127.0.0.1:<ISOS_HTTP_PORT>`（默认 16000）、`https://www.os-island.com` 或 `https://os-island.com`；bridge 回应获准的 CORS/私有网络预检。bridge 的允许列表对浏览器及本地 HTTP 调用方均排除原始执行器、结果和 shutdown 路径；EOS R03 私有 socket 也拒绝上述旧路径，仅所有者可用的内部 `shutdown` 仍保留。请求必须访问回环 bridge 地址。

公开 `primitive_typed` 请求中的 `header.payload_length_bytes` 由浏览器给出，表示紧凑 UTF-8 载荷长度，必须为 1–16,384 字节的正整数。bridge 解析有界公开 JSON，要求该字段和载荷均存在，拒绝声明长度或解析后长度超出 1–16,384，再把私有请求头的长度改写为解析载荷经 `serde_json` 紧凑序列化后的字节数，随后才通过 Unix IPC 发送。公开声明是跨序列化器的有界提示，不是精确字节证明；私有请求头才是权威长度。私有 runtime 仍要求该长度与自身解析载荷的序列化长度精确一致，并在 Dispatcher 前拒绝不合法类型化命令。JavaScript 和 Rust 对某些有限 JSON 数字的表示不同，例如 `0.000001` 与 `1e-6`，因此浏览器不能由墙上时间或字符串字符数推断私有字节。公开长度缺失、非整数、越界或载荷缺失时，在读取授权前返回 HTTP 400 和 `{"ok":false,"error":"invalid typed request"}`。bridge 不改动类型化载荷值、图绑定、目标、修订、命令 ID、截止时间或授权校验。

<a id="control_cn--r07-受管理浏览器提交"></a>
### R07 受管理浏览器提交

**R07 限定范围实现已合并并通过资格验证；证据记于共享 TODO。** 对经过授权的受管理 `primitive_typed` 请求，只有 Dispatcher 已持久化预留该精确命令且取得一个有界执行槽位后，EOS 才能返回带关联命令 ID 的 `InProgress` `DispatchResult`。不增加公开字段或操作。bridge 现有的两秒私有 I/O 界限及浏览器四秒超时约束这次确认，而不约束后续物理调用。若校验、准入或执行在回复前结束，EOS 也可返回精确终态，浏览器必须照实显示。传输超时或确认丢失表示**提交结果不可用**，绝不允许重新发送运动命令。浏览器保留已生成的 ID，仅能在同一当前 IsOS 运行和活动修订下发起经授权的只读 `result` 查询。`InProgress` 确认只证明持久化预留，不证明已下发到执行器或物理完成。

EOS 最多接纳八个可能返回或已返回 `InProgress` 的受管理浏览器运动提交；等待释放闸门和正在调用适配器的工作线程都计入。此上限与私有 socket 客户端数及安全停止容量分离，繁忙的运动通道不能占用安全路径。对于新 ID，EOS 在预留前检查精确活动绑定、目标、类型化载荷限制、授权、运行、修订和当前截止时间，取得槽位并持久化写入 `Prepared`，随后才可确认 `InProgress`。Dispatcher 在最终释放闸门再次检查实时准入，在唯一一次适配器调用前持久化 `Releasing`，并在释放槽位前持久化一个终态 `Completed`、`Failed`、`Cancelled`、`Rejected` 或 `OutcomeUnknown`。八个槽位都被占用时，EOS 对合法新 ID **以一次原子 journal 插入**记录包含完整命令签名、受管理身份、`phase:Terminal` 和 `accepted:false` 的终态 `Rejected`；没有中间 `Prepared` 记录、工作线程或适配器调用。Dispatcher 已将这一独立的崩溃安全转换实现为 `Journal::insert_terminal_rejection`，与 `Prepared` 插入分离。若连拒绝也无法持久化，则返回 `{"ok":false,"error":"movement capacity unavailable"}`，既不确认也不下发命令。截止时间过期或准入不合法必须在物理释放前拒绝。异步路径仍进入同一个 Dispatcher；不建立第二条执行器路线，也不改变 gRPC 运动语义。

对于已预留的 ID，EOS 在计算容量前于 journal 锁下比较包含所提交单调时钟截止时间在内的完整持久化受管理身份及精确类型化参数。完全相同的请求只返回已有 `InProgress` 或终态，不启动第二个工作线程，也不再次调用适配器；身份或参数不同则返回 `Rejected`，不下发。重复请求仍须通过当前授权、运行、修订和活动绑定校验。预留后若无法启动工作线程，EOS 在同一 journal 锁下检查当前阶段：对仍为 `Prepared` 的记录，在任何确认前持久化封闭为终态 `Rejected`；若 Stop 已先将其封闭为终态 `Cancelled`，则返回该取消结果，绝不覆盖。若必要的终态写入失败，EOS 返回不可用错误、禁止后续运动，并保留 journal 供恢复。工作线程槽位由当前代际持有，直到终态持久化，或关闭已封锁后续释放并分类未决调用；分离的工作线程在存活期间仍持有旧 journal 和适配器的所有权，不能进入新代际。


若物理下发可能已经发生，但终态 journal 写入失败，EOS 锁存运动禁用，保留工作线程槽位直到持久化封闭或当前代际退出；物理调用仍存活时，保持旧 journal/适配器所有权，并将该 ID 的浏览器读取标为**结果不可用**，不返回过期的 `InProgress` 或未经持久化的终态猜测。此时不能准入新运动。同一运行中的后续相同 ID 请求不能启动另一线程，只能得到相同的不可用分类。仅当旧物理调用已返回、已确认的停止 epoch 将其阻断，或未知停止清理已回收其旧子进程后，EOS 才可把原始 `Releasing` 记录持久化封闭为 `OutcomeUnknown`；成功后，经授权读取返回该持久化未知终态。若封闭仍失败，在线读取保持不可用，关闭结果只来自 R06 独立的物理停止证明，绝不伪造运动成功。进程重启时，现有持久化 `Releasing` 恢复必须在就绪前写入终态 `OutcomeUnknown` 并保持运动禁用；恢复写入失败则启动失败。故障注入测试须覆盖物理下发前后终态写入失败、读取及槽位行为、迟到适配器回复，以及不重放的恢复。

若排队的工作线程在进入 `Releasing` **之前**判定截止时间过期、健康失败或已进入 `Closing`，但终态写入失败，持久化记录仍为 `Prepared`。EOS 对该 ID 封锁 `Releasing` 及所有适配器调用，在当前代际锁存运动禁用，保留槽位直到持久化封闭或代际退出；经授权结果查询及完全相同的重复提交返回 `{"ok":false,"error":"result unavailable"}`，不能返回过期的 `InProgress`，重复提交也不能启动线程。EOS 在 journal 锁下重试持久化终态：截止时间或健康拒绝为 `Rejected`，Stop 为 `Cancelled`；若发生竞争，Stop 的 `Cancelled` 优先。成功封闭后释放槽位，授权查询可读取该终态。若代际退出前仍无法封闭，正常 journal 恢复须在就绪前将持久化 `Prepared` 转为终态 `Cancelled`，且不下发；恢复持久化失败则启动失败。对截止时间、健康和 Stop 拒绝注入这种释放前终态写入故障，测试过期读取不透明性、重复请求不下发、槽位保留、持久化重试及重启恢复。

<a id="control_cn--r07-浏览器授权结果后续查询"></a>
### R07 浏览器授权结果后续查询

**Q01 目标设计——尚未实现：**浏览器 `primitive_typed` 与公开 `result` 继续接受已发布的 1–128 字节 UTF-8 `command_id`。选定桥接器在核对匹配且有效的私有结果后发出脱敏的[浏览器命令事件](eos_cn.md#events_cn)。不以保留前缀 `h_` 开头的安全 1–64 字节 ASCII ID，原样用作 P06/P07 `correlation_id`；其他有效 ID 按已记录的确定性规则映射为不透明令牌。桥接器不拒绝先前有效的 ID，也不把任意文本复制到日志。


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


公开结果路径对畸形或额外公开键返回 HTTP `400` 和 `{"ok":false,"error":"invalid result request"}`；对伪造的 `authority_grant`、无效 Host 或禁止 Origin 返回 HTTP `403` 和现有 bridge 错误。bridge 授权缺失或不可读取时，在 IPC 前返回 HTTP `503` 和 `{"ok":false,"error":"operator control unavailable"}`。EOS 拒绝私有授权时返回 HTTP `200` 和 `{"ok":false,"error":"operator control grant required"}`；过期修订返回 HTTP `200` 和 `{"ok":false,"error":"stale active revision"}`；不透明 ID 返回 HTTP `200` 和 `{"ok":false,"error":"unknown command ID"}`。私有 IPC 传输失败时，bridge 返回 HTTP `502` 和 `{"ok":false,"error":"runtime unavailable"}`。这些结果路径错误均是有界固定字符串，不回显私有授权或 journal 数据。

<a id="events_cn"></a>
<a id="events_cn--eos-本地-jsonl-日志契约"></a>
## EOS 本地 JSONL 日志契约

[English](../../../../docs/isos/interfaces/server/eos.md#events) · [EOS 接口](../../../eos/interfaces_cn.md#readme_cn) · [所有者生命周期](eos_cn.md#owner_health_cn)

EOS 为每个部署写入一个本地 JSONL 文件和易读的 `eos.log` 配套文件：`data/eos/log/<deployment>/eos.jsonl`。独立运行时使用 `data/eos/log/local/eos.jsonl`。IsOS 向持久所有者提供绝对路径 `EOS_LOG_DIR`，并读取该文件更新日志面板。日志不通过所有者的 Run、Status、激活或 Stop 回复传递。当前图启动所有者和一个机械臂工作进程；工作进程将已准入的增量应用到模拟机械臂。

每行 UTF-8 文本是一个 JSON 对象，包含以下必需字段：

| 字段 | 值 |
| --- | --- |
| `timestamp` | UTC RFC 3339 时间戳 |
| `level` | 小写的 `debug`、`info`、`warn` 或 `error` |
| `source` | 节点使用 `eos.<层>.<组>.<节点>@aether-v0.2.3`；系统进程使用 `eos.<源文件>@aether-v0.2.3` |
| `message` | 人可读的诊断文本；换行必须经过 JSON 转义 |

属于一次运行的记录包含 `run_id`。模拟来源是 `eos.hardware.mock.arm@aether-v0.2.3`。所有者和工作进程的系统来源分别是 `eos.aether_profile.owner@aether-v0.2.3` 与 `eos.aether_runtime.worker@aether-v0.2.3`。Run 或 Stop 失败时，`message` 包含所有者错误码。健康或状态首次观察到工作进程丢失时，所有者写入一条错误。日志用于诊断；就绪、命令结果和 Stop 确认由所有者生命周期与激活回复决定。

保留的 bridge/config 程序通过 `LocalCoreLog` 将有界诊断写入同一个文件。它们不写第二个 `events.jsonl` 文件。当前图不启动这些程序。

<a id="grpc_cn"></a>
<a id="grpc_cn--eos-grpc-与所有者-ipc"></a>
## EOS gRPC 与所有者 IPC

[English](../../../../docs/isos/interfaces/server/eos.md#grpc) · [EOS 接口](../../../eos/interfaces_cn.md#readme_cn)

当前所有者交换使用私有 Unix 套接字。设置、Run、Status 和 Stop 参见[所有者生命周期](eos_cn.md#owner_health_cn)；当前数据路线参见[机械臂控制](../../../eos/interfaces_cn.md#arm_control_cn)和[机械臂监测](../../../eos/interfaces_cn.md#arm_monitor_cn)。早期公开 gRPC 接口属于[EOS 架构](../../../eos/core/roadmap_cn.md#目标架构尚未实现)中的目标设计。

<a id="grpc_cn--r02-冻结的-rust-子集"></a>
### R02 冻结的 Rust 子集

早期 R02 gRPC 子集不在当前演示图中；其归档设计见[运行时网关设计](../../../eos/core/gateway/grpc-gateway_cn.md)。

<a id="grpc_cn--r03-执行器受治理准入目标契约"></a>
### R03 执行器受治理准入目标契约

当前激活通过受运行身份约束的所有者交换和分发器进入 EOS。早期 R03 传输设计见[准入设计](../../../eos/core/dispatcher/dispatcher_cn.md#r03-执行器受治理准入设计)。

<a id="node_contributions_cn"></a>
<a id="node_contributions_cn--eos-软件包节点贡献"></a>
## EOS 软件包节点贡献

> 图 `0.2.1` 在 EOS 设置和 Run 时使用此注册表。两个内置演示技能节点和贡献的原生节点使用下述 Linux ABI；模拟机械臂执行器仍由 EOS 适配器实现。


[English](../../../../docs/isos/interfaces/server/eos.md#node_contributions) · [统一硬件 profile](../../../eos/interfaces/node/hardware/mock_cn.md#hardware_mock_cn--arm) · [原语绑定](../../../eos/interfaces_cn.md#primitive_bindings_cn)

软件包 schema `0.2.0` 和 `0.2.1` 的 `node_contributions.libraries` 将软件包内相对 `.so` 路径绑定至 SHA-256 摘要。EOS 在已验证的软件包载荷内解析该路径（包括已验证的本地链接），检查路径边界和文件内容，并拒绝重复的类型或库标识。EOS 节点类型必须声明 `native_plugin`、`isos_node_v1` 或 `isos_node_v2` ABI 和已声明的库，或声明带接口、版本 `1` 和生命周期适配器的 `external` 实现。所有者设置使用 schema `3`，目录会发布已接受的 `port_routes` 与 `bridge_adapters`。

Linux [v2 ABI](../../../../contract/node/isos_node.h) 将 `observe`、`command` 与 `stop` 分开，并通过宿主回调要求有界、带运行围栏的停止证明。创建 JSON 含 `run_id`、`revision` 和选定实例；服务实例选择 v2，其私有 `adapter.settings.socket_path` 注入该实例。EOS 所有者校验声明的命令端口及载荷，持久预留命令 ID，然后使用私有运行令牌转发至工作进程。重复 ID 仅在节点、端口和载荷相同的情况下返回已记录结果；不确定的交付会阻止后续命令。Stop 先关闭准入，再调用每个 v2 Stop 回调。仅当证明重复运行 ID 和修订版且 `outcome:"stopped"` 时，Stop 才成功。旧软件包版本继续使用 v1 ABI。macOS 和 Windows 加载器为后续工作。

Run 时，IsOS 按精确图节点路径提供 `endpoints.package_services`。每项包含精确软件包 `{id,version}`、`service_instance`、绝对私有 `socket_path`、非空 `service_generation` 和 SHA-256 `config_digest`。EOS 先核对已保存节点的软件包固定版本和服务实例，再将端点注入工作进程实例。图作者写入的端点或代次会被拒绝。仅供所有者使用的 `POST /v1/command` 接受 `{schema_version:1,core:"eos",op:"command",run_id,revision,node,port,command_id,payload}`，只处理已声明的服务执行器端口；它校验类型化载荷、核对底盘坐标系与机械臂关节维度，再通过该运行的 Dispatcher 转发。`aether-runtime --probe-adapter <绝对 .so 路径>` 报告实际找到的 ABI；同时存在两个符号时优先报告 `isos_node_v2`。

原生 ABI 由 [isos_node_v1.h](../../../../contract/node/isos_node_v1.h) 定义。原生工作进程依次调用 create、start、input、health、stop 和 destroy，使用运行 ID 与图修订版约束输入，并在卸载前要求成功的 Stop。原生图工作进程最多载入 64 个贡献的软件节点实例，沿已注册的进程内边传递 `{port,payload}` 输出；每次输入最多触发 256 次传递。它在交付前根据已校验的输出声明检查每个输出端口和类型化载荷。所有者通过 `/v1/input` 接收带 `source_node`、`source_port`、`node`、`port` 和 `payload` 的输入，校验图中精确的输入连接和类型化载荷，并且只接受已声明的 EI 或 IsOS 来源节点；EOS 本地工作进程连线由所有者管理。对于已声明的 EOS→EI 边，所有者以相同的运行 ID 和修订版、源和目标路径将输出转发至 EI 所有者的 `/v1/input`。服务绑定的原生执行器通过该运行的 Dispatcher；未绑定的执行器会被拒绝。两个演示技能类型载入与可执行文件同目录的 `libaether_runtime.so`；模拟机械臂工作进程通过现有 Dispatcher 和硬件适配器处理其输出。完整演示链可以在同一所有者下与原生节点同时运行。EOS 拒绝不兼容的回调表或内容已变化的库。

对于经 Dispatcher 准入的 v2 执行器 `op:"command"`，工作进程只接受目标节点发出的一个私有 `{port:"command_result",payload:{accepted:<布尔值>,...}}`，并连同该精确节点路径返回给所有者。此结果无需图输出声明，也绝不沿图连线转发。缺失、重复、端口错误或载荷格式错误的结果会使命令失败。`command_result` 是保留端口：图输出声明和使用该端口的连线均被拒绝，普通图输入或观测也不能发出它。

所有者向原生工作进程提供私有的、已验证的 `output_types` 列表，其中每项为 `{node,port,data_type}`，并提供被引用的贡献数据类型的完整 `data_types` 描述符。在将输出加入本地连线队列或返回之前，工作进程要求输出端口与声明精确匹配，并按该数据类型的大小上限和 schema 校验载荷。无效输出会使本次输入失败，下游节点不会被调用。

设置验证与 Run 使用同一执行准入检查。EOS 从已校验的投影和已注册的 `port_routes` 编译路线计划：每条可执行的 EOS 本地连线保留精确的来源节点、输出、目标节点、输入和数据类型。仅声明的连线不进入可执行路线计划。完整的八节点模拟机械臂链仍归 Dispatcher 工作进程，贡献的原生节点与服务适配器由原生工作进程承载，`vision_flow` 由所有者桥接。只有演示链完整且每条新增可执行本地连线均有已注册路线时，混合图才通过准入。模拟工作进程只向其他宿主提供 `arm_observer/observation`，没有接收原生节点或所有者桥接输入的通用入口。未注册路线、固定演示 Dispatcher 链以外的命令连线、不支持的跨宿主传输或演示链输入或输出、不完整演示链、缺少实现，或尚未集成 Dispatcher 的贡献执行器，会在 Apply 或 Run 前收到 `unsupported_execution` 诊断。软件包服务节点是原生工作进程的依赖，并非独立的图路由宿主。

[跨后端设计](../../../eos/core/roadmap_cn.md#跨后端图执行)规定工作进程交换与生命周期行为。所有者以同一运行身份启动两个工作进程、要求二者就绪，并沿精确准入连线将模拟观察者的类型化输出交付给原生工作进程。当前 v2 服务 ABI 支持观察与命令回调，但没有类型化输入回调，因此 v2 服务可以向 v1 软件消费者提供观察，不能自行消费类型化软件输入。现有按节点索引的 v2 软件包服务交接与旧版 EOS 单进程交接仍各自独立。


IsOS 安装或链接 Aether 软件包并管理已应用的图。软件包发布版本可以贡献已注册的节点类型和节点集；IsOS 验证发布版本，为 EOS 写出派生的只读节点类型侧文件。EOS 读取图与侧文件，不修改它们。本 JSON 接口是 EOS 专用的节点 IDL，描述身份、位置、设置和类型化端口；硬件采样与命令在 EOS 内仍使用 Rust 类型。下方示例使用当前已注册节点身份和类型化端口。

<a id="node_contributions_cn--已验证的节点类型侧文件"></a>
### 已验证的节点类型侧文件

IsOS 通过可重复的 `--node-types <absolute-path>` 参数把已验证的侧文件传给 `aether-profile` 和 `aether-server`；持久化 EOS 所有者在设置和 Run 时从已验证的软件包引用重新载入所选侧文件。工作进程只接收所选库路径。每个侧文件最多 1 MiB。EOS 先载入安全声明，再载入对应类型；重复或格式错误的定义会被拒绝，并使用内置类型与贡献类型的合并注册表校验图。枚举值使用 `snake_case`，未知字段解析失败。

以下侧文件形状演示一个通用 EOS 软件包；完整类型模式见下文。

```json
{
  "package": {"id": "vendor/camera-driver", "version": "1.2.3"},
  "safety_adapters": [],
  "data_types": [],
  "libraries": [{"path":"lib/camera.so","sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}],
  "adapter_families": [],
  "node_types": [{
    "id": "eos.hardware.camera.driver@v0.2",
    "core": "eos",
    "layer": "hardware",
    "group": "camera",
    "kind": "camera",
    "lifecycle": "adapter",
    "description": "Vendor camera driver",
    "capabilities": [],
    "settings": [],
    "inputs": [],
    "outputs": [],
    "implementation": {"kind": "native_plugin", "abi": "isos_node_v1", "library": "lib/camera.so"}
  }]
}
```

`package` 是精确的 `{id, version}` 引用。ID 由 `/` 分隔的两个有效地址段组成；版本包含三个非空数字部分。贡献类型的实例必须将相同值写入 `node.package`；内置节点省略该字段。注册检查声明的停止能力，但 EOS 只有在类型和软件包对应的适配器及停止实现已编译进运行时时才准入物理执行器。

`node_types` 是[硬件契约](../../../eos/interfaces/node/hardware/mock_cn.md#hardware_mock_cn--arm)中的 `NodeType` 定义数组；`safety_adapters` 是 `SafetyAdapterType` 数组。Rust 与 JSON 字段相同。`core`、`layer` 和 `group` 是必填的位置段；图不能从 group 推断硬件种类。`kind` 值为 `camera`、`base`、`manipulator`、`gripper`、`torso` 和 `software`；生命周期值为 `internal`、`process` 和 `adapter`。IsOS 软件包节点集使用这些类型 ID 和符号图引用；已应用图只包含具体路径。

可选的 `data_types` 是 `DataTypeDescriptor` 数组。每项包含 `id`（字符串，唯一且带版本，例如 `vendor.camera.depth@v0.2`）、`version`（字符串，`0.2`，与 ID 后缀一致）、`purpose`（非空用途说明字符串）、`media_type`（非空 MIME 字符串）、`codec`（非空运行时编解码器 ID）、`schema`（描述逻辑字段的 JSON 对象，使用 `description`、`x-unit` 和 JSON Schema 校验关键字）、`max_payload_bytes`（正整数，字节）。内置 ID 继续使用 snake case，例如 `camera_frame` 和 `vision_frame`。贡献端口引用的数据类型必须拥有已编译进 EOS 的编解码器。内置编解码器 ID 为 `json_v1` 和 `vision_frame_jpeg_v1`。`aether-profile catalog` 和配置服务器的 `GET /api/eos/v2/data-types` 暴露合并目录。

连线的来源与输入必须使用完全相同的已注册 `data_type` ID。schema 的 `x-unit` 注释描述该类型中载荷字段的单位，用于显示和类型内部的校验；不同类型 ID 即使单位相同也不能互换。图连线不做隐式单位转换。贡献描述符不能以不同单位重定义已注册 ID；重复 ID 在注册时拒绝。IsOS 拥有指向具体连线的 Apply 诊断；EOS 拥有启动后的实时健康。[硬件连线契约](../../../eos/interfaces/node/hardware/mock_cn.md#hardware_mock_cn--arm)规定来源 QoS、路由分类、传输和安全限制。软件包贡献端口不能仅通过设置 `data_class: "command"` 或匹配内置运动类型取得命令路径：硬件命令输入只接受已注册的 Dispatcher 原语绑定来源。

可选的 `adapter_families` 数组声明适配器族。每项必填 `id`（有效族名段）、`device_kinds`（非空的 `camera`、`base`、`manipulator`、`gripper` 或 `torso` 列表）、`transport`（`in_process`、`unix_socket`、`websocket` 或 `ros2`）和 `description`（非空说明）。可选 `settings_schema` 为 JSON 对象；可选 `supported_primitives` 是已注册 `<名称>@<版本>` 字符串数组。侧文件外层的 `package` 提供精确来源。软件包作者不能设置 `compiled`；EOS 根据实际已编译的工厂计算该字段，并在目录中暴露 `compiled` 与 `reason`。通用硬件实例把软件包引用写在 `node.adapter.package`，与内置节点类型独立。

每个输入或输出端口必填 `name`（图内名称字符串）、`data_type`（已注册 ID 字符串）和 `required`（布尔值）。可选 `description` 说明用途；`frequency_hz` 是正且有限的赫兹数；`transport` 可为 `in_process`、`unix_socket`、`websocket` 或 `ros2`；`data_class` 可为 `image`、`observation`、`command`、`event`、`state` 或 `opaque`；`cardinality` 可为 `one` 或 `many`（默认为 `one`）。连线校验使用基数、传输和数据类别；相机软件包作者可以在相机帧输出端口声明官方频率。v1 或 v2 原生 Vision 来源必须在图实例的 `node.adapter.settings.frequency_hz` 设为 `(0, 1000]` Hz，才能调度采集并写入来源池；官方输出频率本身不会调度采集。省略时，EOS 继续传递样本，直到后续节点声明官方频率才进行比较；EOS 不从仅表示上限的 `observation.max_rate_hz` 推断该频率。对于输出端口，可选 `frequency_hz` 是注册的官方频率，供安全和诊断使用，并非交付上限；处理节点即使改变了输入频率，也独立声明自己的输出频率。图节点输入可用 `{"sources": <来源对象或数组>, "desired_rate_hz": {"min": <Hz>, "max": <Hz>}}` 代替旧的直接来源对象或数组。期望范围的两个边界均可省略；提供时必须是有限正数，且 `min <= max`。最小值表示偏好，不能生成来源帧；IsOS 根据 `max` 推导连线交付上限。省略边界不增加限速。每个 `@v0.2` 类型的端口都声明传输与数据类别。运行时数据身份由精确的已注册 `data_type` ID 决定。JPEG 编解码和只读传输见 [Vision 接口](../../../eos/interfaces/node/interface/egress_cn.md#interface_egress_cn--vision)。

<a id="node_contributions_cn--类型化数据身份"></a>
### 类型化数据身份

EOS 可用 `NodeDataEnvelope<T>` 包装进程内硬件值。信封包含 `source_node`（规范图路径）、`port`（已注册输出名）、`data_type`（`PortDataType`）、`contract_version`（`2.0.0`）和类型化的 `payload`。当输出端口声明的类型与 `T::DATA_TYPE` 不一致时，`NodeDataEnvelope::output` 拒绝该值。信封用于按 IsOS 图识别数据，不授予命令准入，也不建立第二条执行器路径。

`TypedDataHub` 将这类信封发布到已注册输出键。`subscribe<T>` 校验节点输出端口并返回类型化游标；`poll_latest` 只返回更新的值。每个输出仅保留一个值，慢速订阅者跳过中间值，不会累积队列。丢弃游标即断开该读取者。已编译的 `DataTransformer<Input, Output>` 声明编解码器 ID 并接收类型化选项；`CameraJpegTransformer` 使用 `vision_frame_jpeg_v1` 将 `camera_frame` 转成 `vision_frame`。

无效侧文件 JSON、重复类型 ID、缺少安全适配器、软件包不匹配、未知类型、位置不匹配和不兼容的端口边都在运行时构造前校验失败。缺少已编译的执行器实现在 EOS socket 绑定前导致适配器构造失败。

<a id="node_contributions_cn--类型描述"></a>
### 类型描述

新贡献的 `@v0.2` 节点类型必须包含非空字符串 `description`，解释节点功能、输入、输出及操作员可见效果；EOS 目录显示它。内置类型同样拥有该字段。新类型描述为空、类型 ID 重复、软件包固定版本不匹配或编解码器不支持时，在 Apply 前拒绝注册。外层 `package` 和适配器族 `description` 保持既有含义。

<a id="node_contributions_cn--目标设置选择器"></a>
### 目标设置选择器

`Setting.selector` 是新类型定义中的可选 JSON 对象，说明 IsOS 从何处取得选项，不包含可执行代码或任意文件路径。现有 `name`、`data_type`、`required`、`description`、`unit`、边界和静态选项规则见[硬件契约](../../../eos/interfaces/node/hardware/mock_cn.md#hardware_mock_cn--arm)。

| 字段 | JSON 类型与单位 | 用途、默认值、校验与兼容性 |
| --- | --- | --- |
| `provider` | 字符串枚举，— | 有 `selector` 时必填：`package`、`registry` 或 `graph`；没有选择器时使用普通编辑器。 |
| `resource` | 非空字符串 ID，— | 必填选项族，例如 `scene`、`device`、`adapter_family` 或 `node`；只能由选定 provider 解析。未知族拒绝注册。 |
| `package_from` | 可选字符串枚举，— | `provider=package` 时必填：`node.package` 或 `node.adapter.package`；其他 provider 禁用。它选择图草稿中的精确已验证固定版本。 |

```json
{
  "name": "scene",
  "data_type": "string",
  "required": true,
  "description": "Verified package resource ID",
  "selector": {
    "provider": "package",
    "resource": "scene",
    "package_from": "node.package"
  }
}
```

<a id="node_contributions_cn--结构化设置元数据"></a>
### 结构化设置元数据

已注册的 `object_array` 设置声明 `items_schema` 和条目数量边界。注册表校验结构与大小；IsOS 校验软件包固定版本和可执行文件路径。EOS 进程节点的 `settings.skill_executables` 选择经验证的 Rust `.skill` 文件。每项指定精确软件包和相对可执行文件。[Rust 技能契约](../../../eos/interfaces_cn.md#skills_rust_cn)规定选择交接与工作进程边界。

<a id="owner_health_cn"></a>
<a id="owner_health_cn--eos-所有者生命周期"></a>
## EOS 所有者生命周期

[English](../../../../docs/isos/interfaces/server/eos.md#owner_health) · [EOS 接口](../../../eos/interfaces_cn.md#readme_cn) · [机械臂控制](../../../eos/interfaces_cn.md#arm_control_cn)

持久运行的 `aether-profile owner --socket <absolute-path>` 进程负责 EOS 设置及其直接子进程 `aether-runtime`。IsOS 通过私有 Unix socket 连接。IsOS 进程退出后所有者可继续运行。所有者与工作进程都在给定的绝对路径 `EOS_LOG_DIR` 下写入本地 JSONL 记录；所有者发现工作进程丢失时写入一条错误记录。IsOS 首先读取简短的 `GET /v1/health` 响应并核对运行身份，然后才请求设置目录或其他内部配置。

<a id="owner_health_cn--设置和图投影"></a>
### 设置和图投影

所有者传输使用模式版本 `1`。设置请求通过 `POST /v1/identity`、`/v1/catalog`、`/v1/options`、`/v1/validate` 和 `/v1/render` 发送，包含 `core:"eos"` 及匹配的 `op`；设置响应使用模式版本 `3`。图投影格式为 `0.2.1`，只含一个 `cores.eos` 条目。目录包含[机械臂控制](../../../eos/interfaces_cn.md#arm_control_cn)所述的八个实时演示节点和三个载荷类型。已移除的健康封装节点与 `topology_link` 因类型未知而被拒绝。

<a id="owner_health_cn--runstatus-和-stop"></a>
### Run、Status 和 Stop

`POST /v1/run` 要求 `{schema_version:1,core:"eos",op:"run",run_id,revision,graph,launch}`。`run_id` 是有界 ASCII 标识符，`revision` 是 64 字符十六进制摘要。启动 SHA-256 必须与同目录的 `aether-runtime` 可执行文件一致；参数和环境对象为空。所有者启动 `aether-runtime --serve <private-socket> --run-id <id> --revision <digest> --joint-limit <rad> --source-node <path> --joint-names <csv> --sample-interval-ms <ms> --freshness-timeout-ms <ms>` 并等待就绪。运行身份冲突时拒绝。

对于已准入、同时包含完整模拟机械臂链和原生或软件包节点的图，Run 在同一运行 ID 和修订版下启动原生图工作进程与独立的模拟机械臂工作进程。两个工作进程均报告就绪后 Run 才成功。激活、结果查询和监控发往模拟工作进程；原生输入、观测、视觉和软件包适配器调用发往原生工作进程。所有者每 20 毫秒通过带运行隔离的 `observe_output` 轮询模拟观测源，并仅沿已准入的精确观测边向原生工作进程传送最新的类型化样本。Monitor 和显式观测读取也可能触发交付；重复的样本序号会被跳过。这是最新样本交付，并非无损流：模拟样本序号跳跃时，Status 与健康检查的 `observation_samples_skipped` 统计漏过的样本数，所有者会对首次漏样和每增加 100 个漏样写入带运行身份的警告日志。调用方不能通过 `/v1/input` 向这些由所有者管理的边注入载荷。跨工作进程交付最多产生 256 个输出；原生图路由还分别将累计排队字节数和输出字节数限制在 24 MiB。过时的响应、无效载荷或交付失败会将运行标记为失败，直至 Stop。

`POST /v1/status`、`POST /v1/monitor` 和 `POST /v1/stop` 需要相同的身份字段。Monitor 不驱动执行器，仅返回最新的运行隔离观测；见[机械臂监控](../../../eos/interfaces_cn.md#arm_monitor_cn)。Status 和 GET health 保留工作进程实际的 `ready`、`degraded` 或 `stale` 状态；子进程丢失后 Status 报告 `mock_hardware:"failed"`。降级或过期的工作进程仍可供只读监控，而运动准入保持关闭。Stop 关闭准入、请求子进程退出、等待并回收。若无法确认退出或 socket 清理，Stop 返回 `ok:false` 并保留运行身份以供重试。所有者崩溃时，子进程收到父进程死亡信号。超过 1 MiB 的请求、无效 HTTP 和不支持的操作均被拒绝。

`GET /v1/health` 不需要运行请求体，返回 `schema_version:1`、`core:"eos"`、`setup_schema_version:3`、`state`，以及可为 null 的 `run_id` 和 `revision`。空闲时两个身份字段均为 null；选定运行后，即使工作进程退出或健康状态下降，仍保留精确身份。`run_id` 是有界 ASCII 标识符，`revision` 是已应用图的小写 64 位十六进制摘要。只有两个非空值都与 IsOS 保留的运行回执匹配，IsOS 才能重新附着；意外身份属于冲突，不能据此替换或停止该运行。Health 只是快速的所有者及身份快照；工作进程细节和 Stop 证明仍需带身份约束的 Status 和 Stop。

<a id="owner_health_cn--所有者诊断分页"></a>
### 所有者诊断分页

IsOS 在健康探测之后及运行期间，通过同一个私有套接字读取 `GET /v1/logs?after=<十进制字节偏移>`。响应为 `{schema_version:1,core:"eos",next_cursor,entries}`。`next_cursor` 是所有者追加写入的 `eos.jsonl` 中的非负字节偏移，`entries` 包含原始 JSONL 对象。每次最多读取 64 KiB 源字节，游标仅越过完整且以换行结尾的记录。文件不存在时返回游标为 `0` 的空页；偏移超过当前文件长度时从 `0` 重新开始。无效偏移返回 HTTP 400 和 `invalid_log_cursor`，文件不可读返回 HTTP 503 和 `owner_logs_unavailable`。

页面包含全部警告和错误，以及选定的所有者、Run 与健康工作进程概况信息；常规低层信息不进入页面。IsOS 保存有界、持久的副本，供后来连接的工作台读取；IsOS 重新附着后仍可从所有者文件重放。日志页是诊断数据，不能代替带身份约束的 Status 或 Stop 证明。

混合运行中的 Status 和健康检查要求两个工作子进程都可连接，并通过 `mock_hardware` 报告模拟工作进程状态；任一工作进程丢失均将运行报告为失败。Stop 先关闭 Dispatcher 准入，再取得模拟机械臂 Stop 证明和原生工作进程 Stop 证明，之后才停止选定的软件包服务子进程。缺失证明时返回 `stop_uncertain`，并保留运行隔离状态以供重试。

由原生图工作进程承载的软件包节点之间的精确类型化边在该工作进程内执行。所有者保留 IsOS 选定的软件包固定版本和服务交接；软件包适配器不能通过图边打开新的执行器路由。

对于每个已选 v2 服务节点，Run 根据运行 ID、修订版、软件包固定版本和服务实例推导预期服务代次，然后向已验证交接 socket 上所选 `device_id` 发送 IsOS 分帧 `health` 请求。就绪要求响应接受该请求、重复该代次并声明 `strict-socket-v1` 特性。socket 缺失或响应过期时，Run 返回 `service_not_ready` 或 `service_generation_mismatch`。Status 和 GET health 在合计 500 毫秒期限内重复探测，并对共用同一服务设备的节点去重；服务丢失或原生观测路线失败会将运行锁定为 `failed`，直至 Stop。该检查适用于每个已选服务节点，包括带服务交接的软件节点。

<a id="package_observer_cn"></a>
<a id="package_observer_cn--软件包观测运行时"></a>
## 软件包观测运行时

[English](../../../../docs/isos/interfaces/server/eos.md#package_observer) · [EOS 接口](../../../eos/interfaces_cn.md#readme_cn) · [通用硬件](../../../eos/interfaces_cn.md#general_hardware_cn)

本页定义较早的 v1 仅观测软件包进程路径。当前 v2 服务命令和 Stop 边界见[节点贡献](eos_cn.md#node_contributions_cn)及[通用硬件](../../../eos/interfaces_cn.md#general_hardware_cn)。EOS 可以把已验证、仅用于观测的软件包进程作为直接子进程运行。IsOS 提供一个精确的进程交接和选中的适配器共享库。EOS 再次核对摘要，绑定私有 Unix socket，仅展开声明的参数占位符，并在把适配器载入原生节点工作进程前启动该进程。图必须包含选中的进程节点和使用同一适配器族的相机节点。在这一 v1 路径中，底盘和机械臂软件包运动不可用；v2 服务路径有独立的 Dispatcher 与 Stop 证明。

私有所有者路由 `POST /v1/observe` 接受 schema 1、核心 `eos`、操作 `observe`、当前运行 ID 与修订，以及已声明的相机节点路径。原生适配器收到 `{"op":"observe"}`。EOS 仅接受已声明端口且符合注册数据 schema 的输出，并转发已声明的 EOS 到 EI 边。未知节点、过期运行、文件变化、无效端口或载荷均关闭准入。`aether-runtime --probe-adapter <绝对路径 .so>` 在独立进程中检查 `isos_node_v1` 表和必需回调；它不会启动硬件。

EOS 将进程、适配器族和相机节点绑定到相同的精确软件包 ID 与版本。Run 时再次按安装回执核对每个选中 sidecar。工作进程就绪检查失败时回滚两个子进程；清理结果不确定则返回 `stop_uncertain`。相机载荷上限为 20 MiB，原生输出上限为 24 MiB，所有者响应上限为 20 MiB。

Stop 首先要求所有原生适配器的 `stop()` 成功且工作进程退出，然后终止并回收直接软件包子进程组。任一证明失败时 EOS 报告 `stop_uncertain`。仅观测适配器的 `stop()` 证明采样已经静止；执行器软件包在获准运行前还需要独立的所有者专属实体 Stop 回执。

内置相机类型允许 `rgb8` 和 `jpeg` 能力。软件包实例声明其实际发出的编码；EOS 在所有者边界检查 `image_frame@v0.2` 载荷 schema。

<a id="package_process_cn"></a>
<a id="package_process_cn--所有者软件包进程交接"></a>
## 所有者软件包进程交接

[English](../../../../docs/isos/interfaces/server/eos.md#package_process) · [接口目录](../../../eos/interfaces_cn.md#readme_cn) · [IsOS 进程契约](../../../../docs/isos/interfaces/package/README.md#process) · [JSONL 事件](eos_cn.md#events_cn)

软件包进程是位于同一核心 `process` 层的注册 `<core>.process.<group>.<node>@v0.2` 类型。实例选择精确 `node.package` 锁定。IsOS 在生成绑定运行的 `isos.package-process@v1` 交接之前，验证发行包、包内可执行文件、资源描述符与选中资源 ID。选中的所有者须声明并实现所命名的生命周期适配器。仅有可执行文件不授予启动权限。

交接包含所属核心、节点路径与类型、精确软件包锁定、已验证入口与资源路径、生命周期适配器、运行 ID、保存的图修订，以及发行包和载荷摘要。所有者重新验证输入，监管自己的直接子进程，且只在实时健康得到证明后报告就绪。未知字段、变化的字节、不支持的适配器与跨所有者位置在启动前被拒绝。通用通信形状由 [IsOS 交接契约](../../../../docs/isos/interfaces/core.md#core_handoff)定义。

EOS 在终止 EOS 自有子进程前关闭执行器准入并取得 Stop 证明。IsOS 只在选中所有者的证明与直接子进程退出一致时接受确认 Stop。缺失或不确定的证明保持未知终态。软件包子进程标准输出与错误输出仅供诊断；有界的所有者事件承载权威运行身份。

<a id="reader_features_cn"></a>
<a id="reader_features_cn--eos-读取器身份"></a>
## EOS 读取器身份

[English](../../../../docs/isos/interfaces/server/eos.md#reader_features) · [设置](eos_cn.md#setup_v1_cn) · [所有者生命周期](eos_cn.md#owner_health_cn)

IsOS 在 Apply 前探测选定的 EOS 可执行文件。`aether-profile --capabilities` 报告源码包版本 `0.2.3`、图格式 `0.2.1`、节点类型版本 `0.2` 和四个内置 EOS 节点类型：

- `eos.interface.ingress.trigger@v0.2`
- `eos.skill.demo.joint_delta@v0.2`
- `eos.dispatcher.arm.delta@v0.2`
- `eos.hardware.mock.arm@v0.2`

`binary` 为 `aether-profile`；功能列表为 `catalog`、`setup-v3`、`arm-activation-owner` 和 `validate-file`。目录还提供 `button_activation@v0.2` 与 `arm_joint_delta@v0.2` 数据类型。实时健康状态和激活结果来自[所有者生命周期](eos_cn.md#owner_health_cn)及[机械臂控制](../../../eos/interfaces_cn.md#arm_control_cn)。

`aether-profile validate --file <graph> [--node-types <已验证侧文件>]...` 读取仅包含 EOS 核心的 `0.2.1` JSON 图投影（不超过 1 MiB），依据合并后的注册表验证，并执行与设置及 Run 相同的可执行模式准入检查。只有图可以执行时才无输出地成功退出。解析、类型、拓扑或运行时不可用错误会在 stderr 给出诊断并以失败状态退出；此命令不启动工作进程。需要侧文件时，IsOS 传入可重复的已验证侧文件路径。

<a id="setup_v1_cn"></a>
<a id="setup_v1_cn--eos-设置交换"></a>
## EOS 设置交换

[English](../../../../docs/isos/interfaces/server/eos.md#setup_v1) · [所有者生命周期](eos_cn.md#owner_health_cn) · [EOS 接口](../../../eos/interfaces_cn.md#readme_cn)

IsOS 通过仅供所有者使用的 Unix 套接字调用持久的 EOS 所有者。传输信封使用 `schema_version: 1`、`core: "eos"` 和与 `POST /v1/<op>` 匹配的 `op`。设置响应使用模式版本 `3`；传输、设置、图和节点类型版本相互独立。一次性的 `aether-profile setup --json` 以模式版本 `3` 接受相同的设置内容。

<a id="setup_v1_cn--操作"></a>
### 操作

`catalog.direct_only_types` 列出有直接测试处理器但尚无图工作进程集成的 EOS 类型。这些类型不列于 `identity.supported_node_types`。图选中其中之一时，`validate` 返回 `unsupported_execution`，原因以 `isolated_node_graph_runtime_unavailable` 开头。[隔离节点库契约](../../../eos/interfaces_cn.md#isolated_library_cn)定义直接 Rust API，以及接入运行时仍需的所有者服务。

`identity` 返回产品及支持的图和节点类型版本。`catalog` 返回内置演示类型和选定的软件包贡献。`options` 读取尚未完成的 EOS 草稿，并为每个已注册硬件节点返回一个图范围内的 `adapter.type` 选项组，包括软件包贡献的节点。每个选项必须匹配节点准确的软件包版本固定值和设备类别；只有 EOS 实现已编译或其经过验证的库可加载时，选项才可用。尚未选择适配器时 `selected` 为 null；选项结果不会使草稿可以运行。`validate` 检查仅含 EOS 的图投影并返回诊断，不生成文件。`render` 还返回已验证的 `eos/graph.json` 文件，供 IsOS 再次检查保存的修订后原子发布。一次性命令与持久所有者均只载入请求中精确的 IsOS 已验证 `packages` 引用所命名的侧文件，然后才准入节点类型；身份、载荷根目录或贡献不匹配会使设置失败。设置不会启动工作进程。

```json
{
  "schema_version": 1,
  "core": "eos",
  "op": "validate",
  "revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "graph": {
    "version": "0.2.1",
    "robot": {"id": "robot", "name": "Robot"},
    "cores": {"eos": {"layers": {"interface": {"groups": {"ingress": {"nodes": {}}}}}}}
  },
  "endpoints": {},
  "packages": []
}
```

示例说明信封结构；有效图要求每个出现的分组至少有一个注册的 EOS 节点。投影只包含 `cores.eos`；其声明的跨 core 输入可引用经 IsOS 准入的 EI 来源。无效图版本、缺少 EOS 层、未知类型、错误设置和不兼容的本地连接返回 `ok:false`、有界诊断且不生成文件。请求和响应大小分别限制为 1 MiB 和 4 MiB。[所有者生命周期契约](eos_cn.md#owner_health_cn)定义 Run、Status、Stop 和健康状态。

有效 `validate` 请求返回空文件列表。请求格式错误时，诊断指出被拒绝的字段和代码；EOS 不会渲染部分图。

```json
{
  "schema_version": 3,
  "ok": true,
  "diagnostics": [],
  "files": []
}
```

```json
{
  "schema_version": 3,
  "ok": false,
  "diagnostics": [
    {
      "path": "request",
      "code": "invalid_json",
      "message": "setup request is not JSON"
    }
  ],
  "files": []
}
```

缺少已注册的必需输入由 IsOS 报告为警告。EOS 仍会验证已连接的边，并在 Run 前要求受支持的模拟机械臂可执行拓扑。输入断开时，模拟工作进程仍可运行，但所有者会在声明的命令链完整连接前拒绝激活，并在观测链完整连接前拒绝监控读取。缺少连接不会产生隐式执行器路由。
