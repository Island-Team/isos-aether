# IsOS–EI server interface

<!-- doc-navigation:start -->
<details>
<summary>目录：isos/interfaces/server</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [IsOS 所有者服务端接口](README_cn.md)
  - [IsOS–EOS server interface](eos_cn.md)

</details>
<!-- doc-navigation:end -->

<a id="events_cn"></a>
<a id="events_cn--ei-本地诊断"></a>
## EI 本地诊断

[English](../../../../docs/isos/interfaces/server/ei.md#events) · [日志设计](../../../ei/core/runtime/runtime.md) · [首次使用](../../start/quick-start_cn.md)

EI 同时写入易读的 `ei.log` 配套文件，并向 `data/ei/log/<deployment>/ei.jsonl` 逐行追加 JSON 对象（独立运行使用 `local`）。`EI_LOG_DIR` 可选择绝对路径下的私有目录。IsOS 在本地读取此文件来填充日志面板。EI 所有者接口仅传递健康与生命周期回复，不提供日志传输路由。

每条记录包含 `timestamp`（RFC 3339 UTC）、`level`（`debug`、`info`、`warn` 或 `error`）、`source` 与 `message`。节点来源格式为 `<core>.<layer>.<group>.<node>@aether-v0.2.3`；系统代码使用点分文件路径，例如 `ei.owner_service.py@aether-v0.2.3`。可选字段包括 `event_code`、`run_id`、`correlation_id` 及有界且已清理的 `fields`。JSONL 记录自身不含显示前缀。EI 在写入前清理秘密、URL、提示、媒体及原始负载。

所有者在同一文件中记录工作进程启动、意外退出及 Stop。日志记录只是诊断证据，不是命令，也不能证明物理执行成功。日志写入失败不会改变所有者生命周期回复。

<a id="node_cn"></a>
<a id="node_cn--ei-激活节点"></a>
## EI 激活节点

[English](../../../../docs/isos/interfaces/server/ei.md#node) · [配置](ei_cn.md#setup_cn) · [架构](../../../ei/core/architecture.md)

节点实例可以省略 `description` 或将其设为空字符串；EI 将其作为可选展示文本保留。

四个 Unix 套接字边界阶段注册 `ei.owner.stage.v1` 外部实现，并在 EI 所有者中执行。意图转发阶段和机械臂状态阶段注册 `native_plugin`，使用共享的 `native/libei_demo_stage.so` 实现 `isos_node_v1`；EI 所有者在选定运行期间创建、启动、输入、检查、停止并销毁每个实例。图 `0.2.1` 注册三个激活数据传递节点和三个机械臂监测数据传递节点：`ei.io.input.trigger@v0.2` 位于 `io/input`，`ei.cognition.intent.forwarder@v0.2` 位于 `cognition/intent`，`ei.governance.safety.gate@v0.2` 位于 `governance/safety`。每个节点都有名为 `activation` 的输入和输出，传递 `button_activation@v0.2`，交付模式为 `event`，数据类别为 `event`。节点设置为空。旧健康状态节点和仅声明型节点不再接纳。

输入节点通过 `unix_socket` 接收 IsOS 所有者桥接，契约为 `{owner:isos,id:button-activation,version:1}`。其输出及认知节点的输入、输出使用 `in_process`，契约为 `{owner:ei,id:activation-local,version:1}`。安全节点接收此本地契约，并通过 `unix_socket` 和 `{owner:ei,id:activation-to-eos,version:1}` 向外发送。图输入引用必须构成 IsOS 按钮 → EI 输入 → 认知 → 安全的有序路径。IsOS 验证穿过 EOS 的完整路径。

EI 原样传递 `{event_id,kind:"activate"}`，不选择运动。所有者通过已注册的输入与转发实例交付数据，并在发送给 EOS 前检查安全阶段输出；受管 EI 工作进程提供进程就绪状态。安全阶段在转发前检查选定运行、修订、工作进程存活、图路径和 EOS 所有者端点；它不建立执行器通道。只有 EOS 接纳机械臂命令。

<a id="node_cn--只读机械臂监测回传"></a>
### 只读机械臂监测回传

回传链注册 `ei.io.input.arm_status@v0.2`（`io/input`）、`ei.cognition.state.arm_status@v0.2`（`cognition/state`）及 `ei.governance.monitor.arm_status@v0.2`（`governance/monitor`）。每个节点的 `observation` 输入和输出传递 `arm_monitor_state@v0.2`，交付模式为 `event`，数据类别为 `observation`。监测节点可在高级设置中指定 `forward_rate_hz`（默认 20，范围 1–20）；输入和状态节点设置为空。EOS egress → EI 输入的 Unix 套接字契约为 `{owner:eos,id:arm-monitor-to-ei,version:1}`。EI 输入 → 状态 → 监测的进程内契约为 `{owner:ei,id:arm-monitor-local,version:1}`。EI 监测 → IsOS UI 的 Unix 套接字契约为 `{owner:ei,id:arm-monitor-to-isos,version:1}`。

监测设置描述符声明 `data_type: "number"`，与 IsOS 节点目录的设置模式一致。

EOS 采样源在独立观察适配器中为 `eos/hardware/mock/arm_observer`，在现有已应用图中为 `eos/hardware/mock/arm`。EI 只接收自己的图投影，因此接纳这两个精确的 EOS 来源路径；运行、修订和有序关节名称约束仍然生效。

EI 所有者在运行所属的后台任务中每秒最多按 `forward_rate_hz` 读取 EOS 私有 `POST /v1/monitor` 端点，与 IsOS 读取者无关。它要求选定运行和修订、存活的工作进程、完整的 EOS egress → EI 输入 → 状态 → 监测声明路径，以及不倒退的 EOS 序号。Run 从已应用 EOS 机械臂节点取得 `endpoints.arm_monitor.joint_names`；EI 拒绝关节名称和顺序不同的观测。验证 EOS 响应后，所有者原样传递其中的 `observation` 对象。`ready` 携带六个有限的弧度关节位置；`degraded`、`stale` 和 `stopped` 携带空位置及明确原因。已停止或过期运行不能读取新数据。IsOS UI 最多以 5 Hz 获取最终 EI 观测；回传链绝不调用运动准入。

<a id="node_cn--软件包贡献的-ei-节点"></a>
### 软件包贡献的 EI 节点

通过 IsOS 验证的软件包可声明 EI 节点类型，使用现有 `@v0.2` 类型标识、精确的 `ei/<layer>/<group>` 放置位置、带类型的端口与设置，并提供 `implementation`。EI 目录只读取设置请求中指定的软件包存储，拒绝重复类型标识，并公布软件包显式声明的 `port_routes` 和 `bridge_adapters`。端口类型相同本身不能证明路由可用。IsOS 在调用 EI 前验证发行元数据和完整存储；EI 再次核对所选发行版身份和本地库字节。

在 Linux 上，`native_plugin` 使用 `abi: "isos_node_v1"` 以及在 `node_contributions.libraries` 中列出 SHA-256 的软件包相对 `.so` 文件。EI 要求导出的 `isos_node_v1` 表报告 ABI 1、完整的表大小和非空的生命周期及数据回调。Run 创建并启动实例、检查健康状态；Stop 要求 `stop` 返回零，随后才销毁实例。`external` 使用版本 `1` 的接口及匹配的 `node_contributions.processes` 条目。EI 将可执行文件作为直接子进程启动，并设置 `ISOS_NODE_SOCKET`、`ISOS_RUN_ID` 和 `ISOS_REVISION`；该进程在私有 Unix 套接字上提供逐行 JSON 请求。请求使用 `{version:1,op:"start"|"input"|"health"|"stop"}`；输入附带 `data`，成功回复使用 `{version:1,ok:true}`。输入回复带有 JSON 对象构成的 `outputs` 数组；健康回复要求 `ready:true`；Stop 回复要求 `quiesced:true`。请求和回复上限各为 1 MiB，超时为五秒。

EI 所有者的私有 `POST /v1/node-input` 接受 `schema_version:1`、`core:"ei"`、`op:"node-input"`、精确的 `run_id` 和 `revision`、选定的 EI 节点路径及 JSON `data` 对象。它返回节点产生的 `outputs`，或拒绝过期运行、不可用节点、失败输入及未就绪节点。此仅所有者可访问的数据路径不会转发执行器命令；固定的 EI 安全阶段和 EOS 调度器仍是执行动作的必要环节。自定义 EI 节点之间声明的本地路由通过匹配输出的 `port` 与目标输入，同步交付数据。自定义 EI↔EOS Unix 套接字路由只有在两个所有者都声明精确的软件包路由，并且已应用图声明匹配的实例边时才会被接纳。EI 所有者通过私有 `POST /v1/input` 接收 EOS 数据，并将 EI 输出转发至 EOS 的 `POST /v1/input`。每个请求携带 `schema_version:1`、`core`、`op:"input"`、`run_id`、`revision`、目标 `node` 和 `port`、来源 `source_node` 和 `source_port`，以及有界 JSON `payload`。EI 在交付前核对精确的来源连接，返回带运行约束的 `{node,output:{port,payload}}` 输出。内置的激活至执行器路由继续经过 EI `/v1/activate` 和 EOS 调度器准入。

缺少必需输入属于 IsOS 图警告。EI 接受所选投影，但断开的激活路径无法通过所有者激活路径检查，也不会向 EOS 转发。断开的监控路径不会启动监控轮询线程。

<a id="reader_features_cn"></a>
<a id="reader_features_cn--ei-读取器身份"></a>
## EI 读取器身份

[English](../../../../docs/isos/interfaces/server/ei.md#reader_features) · [Setup 交换](ei_cn.md#setup_cn) · [阶段管线](../../../ei/core/runtime/pipeline.md)

选定的 `ei-config` 身份报告产品版本 `0.2.3`、图格式 `0.2.1`、节点类型版本 `0.2`、`supported_graph_versions:["0.2.1"]`、`supported_node_type_versions:["0.2"]`、排序后的可在图中选择的阶段类型，以及 setup 模式 `3`。仅供直接调用的认知类型列于目录，但不列于 `supported_node_types`，因为普通图设置会拒绝这些类型。其特性包含 `setup-v3`、`catalog`、`custom-node-v1`、`graph-0.2-ei-projection`、`health-only-owner-v1`、`activation-owner-v1` 和 `arm-monitor-owner-v1`。

选定的 `ei` 运行时探针报告 `managed-config-v1`、`eos-gateway-client-v2`、`ei-evidence-scheduler-v1`。IsOS 在 Apply、Run 和 Restart 前比较所需特性及选定可执行文件字节。探针只读，不能授予进程或执行器权限。

EI 在 `/graph/version` 拒绝不支持的图 envelope，并以有界诊断拒绝未知阶段类型、无效位置或路线以及畸形配置。EOS 服务版本作为独立线协议契约检查。

<a id="safety_reset_cn"></a>

[English](../../../../docs/isos/interfaces/server/ei.md#safety_reset) · [事件](ei_cn.md#events_cn) · [首次使用](../../start/quick-start_cn.md)


<a id="safety_reset_cn--仅所有者可访问的状态"></a>
### 仅所有者可访问的状态


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

<a id="setup_cn"></a>
<a id="setup_cn--ei-setup-交换"></a>
## EI setup 交换

[English](../../../../docs/isos/interfaces/server/ei.md#setup) · [读取器身份](ei_cn.md#reader_features_cn) · [阶段管线](../../../ei/core/runtime/pipeline.md)

IsOS 使用一份有界 JSON 请求调用选定的 `ei-config` 进程，操作为 `identity`、`catalog`、`options`、`validate` 或 `render`。托管 setup envelope 使用数字 `schema_version: 3` 和 `core: "ei"`。EI 接受图格式 `0.2.1` 的独立 `cores.ei` 投影；EI 核心根据注册的位置、设置、端口和实现验证六个内置阶段节点及软件包贡献的 EI 节点声明。旧健康状态节点和仅声明型节点将被拒绝。IsOS 管理保存的修订、软件包验证及返回文件的发布。

`values` 包含 IsOS 可编辑的 EI listener 与 gateway 连接。`mode` 必须为 `isos`；`server` 和 `gateway` 使用各自文档定义的严格字段。setup 错误包含稳定 JSON Pointer `path`、`code` 和有界 `message`，且不生成文件。setup 操作不启动 EI 运行时，也不连接 EOS。

```json
{
  "schema_version": 3,
  "op": "identity",
  "core": "ei"
}
```

身份报告源码产品版本 `0.2.3`、图格式 `0.2.1`、节点类型版本 `0.2`、精确支持集合、setup 模式版本 `3` 及选定特性。EI 所需的 EOS Gateway、Capability 与 Skill 服务版本是独立的线协议身份。

有效 `validate` 请求的响应不含生成文件。节点被拒绝时，EI 返回指向准确字段的 JSON Pointer 和稳定代码；消息仅供诊断。

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
      "path": "/graph/cores/ei/layers/cognition/groups/context/nodes/bad/node_type",
      "code": "unsupported_node_type_version",
      "message": "unsupported EI type 'ei.cognition.context.retired@v0.2'"
    }
  ],
  "files": []
}
```

对于被拒绝的 EI 节点，单次 setup 命令和所有者套接字返回相同的类型化诊断。`path` 指向被拒绝的图字段，例如 `/graph/cores/ei/layers/cognition/groups/intent/nodes/bad/node_type`；`code` 区分 `unsupported_node_type_version` 与 `invalid_node_placement`。IsOS 可以在 setup 诊断中记录此有界响应，而无需依赖未准入的所有者 stderr。

<a id="setup_cn--持久化所有者控制套接字"></a>
### 持久化所有者控制套接字

`options` 返回空的 `setting_options` 列表，因为目前节点设置是静态的。`validate` 和 `render` 验证 EI 激活节点声明与解析后的端点输入。

`ei-config owner --socket <绝对路径>` 在 Unix 套接字上运行持久的 EI 所有者服务。套接字文件权限为 `0600`，父目录必须预先存在。`GET /v1/health` 不需要运行请求体，返回 `schema_version:1`、`core:"ei"`、`setup_schema_version:3`、工作进程状态 `idle`、`running` 或 `exited`，以及可为 null 的 `run_id` 和 `revision`。空闲时两个身份字段均为 null；选定运行后，即使工作进程退出，仍保留精确的有界 ASCII 运行 ID 和已应用图的小写 64 位十六进制修订。IsOS 先读取这一简短状态快照，再请求目录或其他内部配置。只有所选身份与保留的运行回执匹配，IsOS 才重新附着；不匹配属于冲突，且 Health 本身并非 Stop 证明。`POST /v1/identity`、`/v1/catalog`、`/v1/options`、`/v1/validate` 和 `/v1/render` 使用 JSON 信封：`schema_version:1`、`core:"ei"`，且 `op` 与路径一致。其他字段沿用上述设置交换，响应仍使用设置 schema 版本 3。请求、头部和响应的上限分别为 1 MiB、8 KiB 和 4 MiB；每个连接的超时为十秒。

`GET /v1/logs?after=<十进制字节偏移>` 使用同一个私有套接字。响应为 `{schema_version:1,core:"ei",next_cursor,entries}`；`entries` 包含原始本地 `ei.jsonl` 记录，`next_cursor` 是非负源字节偏移。每次最多读取 64 KiB，游标仅越过完整且以换行结尾的记录。文件不存在时返回游标为 `0` 的空页；偏移超过文件长度时从 `0` 重新开始。无效偏移返回 HTTP 400 `invalid_log_cursor`；文件读取失败返回 HTTP 503 `owner_logs_unavailable`。页面包含全部警告、错误，以及选定的所有者和健康工作进程概况信息，不包含常规低层信息。IsOS 保存有界且持久的副本，供后来连接的工作台读取，并在重新附着后从此所有者游标继续读取。日志是诊断数据，不能作为工作进程健康或 Stop 证明。

生产环境的 `POST /v1/run` 接受传输版本 1 的信封，包含 `core:"ei"`、`op:"run"`、有界 `run_id`、小写 64 位十六进制 `revision`、所有者投影后的 `graph`、`endpoints`、已验证的 `packages`，以及 `launch:{sha256,args,env}`。EI 重新验证图并核对 `ei` 工作程序的 `sha256`，随后每个所有者在独立进程组中最多启动一个工作进程。仅报告健康状态的工作进程不占用完整运行时的安全状态锁；所有者仍约束选定运行和 Stop。`POST /v1/status` 和 `/v1/stop` 要求精确匹配运行身份。Stop 向受管工作进程发信号并回收，然后返回 `outcome:"stopped"` 与 `worker_exited:true`。IsOS 客户端退出不会终止所有者服务；Stop 后可接受新运行，同时拒绝旧运行身份。`/v1/apply` 返回 501。EI 将诊断写入本地 JSONL 文件。

EI 所有者从信封注入 `ISOS_RUN_ID`，并拒绝 `launch.env` 中冲突的值。已完成的运行身份在服务整个生命周期内不可重复使用。工作进程意外退出时，Stop 回收它后才可启动另一运行。

`POST /v1/activate` 接受版本 1 的激活封套，包括运行 ID、修订、来源节点、事件 ID 和 `kind:"activate"`。所有者检查运行、工作进程和 EI 的有序图路径，再转发至 Run 提供的 `endpoints.eos_owner.path`。成功响应包含 EOS 命令 ID、`status:"accepted"`，以及 EI 运行、修订和事件 ID；物理执行异步完成。`POST /v1/result` 使用相同的版本和运行约束，携带 `op:"result"` 及事件 ID，转发 EOS 按该事件关联的 `accepted`、`running`、`succeeded`、`failed`、`cancelled` 或 `outcome_unknown` 结果；仅成功结果携带六个模拟关节位置。旧运行返回 409；无效路径返回 422；工作进程不可用或 EOS 结果不确定返回 503。Stop 在结束工作进程前关闭接纳。

`POST /v1/monitor` 接受数字版本 1、`core:"ei"`、`op:"monitor"`、运行 ID 和修订。成功响应在 EI 运行与修订封套下携带未经修改的 EOS `observation`。包含监测节点的图在 Run 时必须提供 `endpoints.arm_monitor.joint_names`，即已应用 EOS 机械臂节点的六个有序名称。监测节点的 `forward_rate_hz` 默认为 20，允许 1–20 Hz；运行所属的后台任务按不高于该速率轮询 EOS，与 IsOS 读取者无关。读取返回最新的已验证快照，刷新后三个间隔与一秒两者取较长值，在该时限内有效。旧运行返回 409；无效图路径返回 422；工作进程或 EOS 所有者丢失、关节名称不符、采样无效、序号倒退或缓存过期返回 503。Stop 清除任务和缓存，然后才允许新运行。

对于软件包贡献的节点，每个已验证 `packages` 引用提供 `id`、`version`、指向 IsOS 软件包存储的 `path`，以及可选的链接软件包文件根目录 `payload_root`。EI 从 `path` 读取 `release.json`，在提供 `payload_root` 时从该目录解析 `.so` 库和外部入口文件，并拒绝路径越界或已变化的本地库摘要。目录返回六个内置阶段类型、受支持的软件包类型、显式 `port_routes` 和 `bridge_adapters`。只有具有已实现传输的本地自定义 EI 节点路由才会公布；自定义 EI↔EOS Unix 套接字路由要求匹配的所有者声明和图边。[节点接口](ei_cn.md#node_cn)定义私有输入与生命周期消息。

Run 中的 `ei_to_eos_edges` 端点投影包含由 IsOS 接纳的 `{source,output,target,input}` 条目。EI 只将存活的自定义来源节点绑定到这些条目，并通过 EOS 所有者套接字转发匹配的输出。内置的安全至执行器路由不经过此通用路径。
