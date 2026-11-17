# Client interactions

<!-- doc-navigation:start -->
<details>
<summary>目录：isos/interfaces/api</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [客户端 API 与前端接口](README_cn.md)
  - [本地 HTTP 路由](http_cn.md)

</details>
<!-- doc-navigation:end -->

## 目录

- [按钮分组接口](#button--按钮分组接口)
- [浏览器原语控制与结果契约](#control--浏览器原语控制与结果契约)
- [节点图加载与状态展示](#graph_loading--节点图加载与状态展示)
- [当前运行日志导航——P07 UI 契约](#log_cursors--当前运行日志导航p07-ui-契约)
- [监控分组接口](#monitor--监控分组接口)
- [展示描述符边界](#presentation--展示描述符边界)
- [通用输出数据选择](#selection--通用输出数据选择)
- [视觉分组接口](#vision--视觉分组接口)
- [工作区订阅生命周期](#workspace_panels--u03-工作区面板契约)

浏览器共用传输层在请求失败时显示第一个公开的 `error.details.diagnostics[]` 路径和消息，并将 API `correlation_id` 显示为请求参考编号。类型化错误码和完整结构化详情仍可供组件处理。`409 revision_conflict` 提示刷新；其他冲突保留服务端给出的具体原因。客户端不显示任意响应正文。

<a id="button"></a>
<a id="button--按钮分组接口"></a>
## 按钮分组接口

[English](../../../../docs/isos/interfaces/api/client.md#button) · [节点类型](../node/README_cn.md)

IsosUI 通过 `POST /api/isos/v1/nodes/events` 将注册按钮节点的一次通用激活发送给 IsOS。IsOS 在转发到私有 EI 所有者之前验证本地 Host 和 Origin、活动运行、已保存修订以及经过 EI 和 EOS 的准确声明路径。浏览器不能选择机械臂、关节目标、所有者地址或执行器授权。

对于扩展的 `isos.ui.external.v1` 节点，工作台调用 `POST /api/isos/v1/nodes/external`，请求体为 `{schema_version:1,node_path,run_id,active_revision,op,port,payload}`。`op` 为 `input` 或 `read`；读取操作的载荷必须为 null。IsOS 在连接软件包子进程的私有 Unix 套接字前，检查当前运行、已保存图的精确修订值、选定的软件包节点实例、已注册端口和 JSON Schema 数据类型。读取结果也按声明的输出数据类型检查。响应包含 `{node_path,port,run_id,active_revision,payload}`。此接口不传送图连线，也不提供执行器路径。

```json
{
  "schema_version": "0.2.0",
  "source_node": "isos/ui/button/trigger",
  "output": "activation",
  "data_type": "button_activation@v0.2",
  "run_id": "run-1",
  "active_revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "payload": { "event_id": "click-1", "kind": "activate" }
}
```

所有字段必填，未知字段会被拒绝。`source_node` 有四个 ASCII 路径段，每段最多 48 字节，只使用字母、数字、`_` 或 `-`。`event_id` 为 1–64 个 ASCII 字母、数字、`_`、`-` 或 `.`；`run_id` 使用相同语法，最多 128 字节。修订为 64 个小写十六进制字符。请求体最多 2048 字节。不同点击使用新事件 ID；结果不确定的点击保留原 ID。

节点图必须有一条获准路径，从按钮经过 EI 输入、认知、安全，再到 EOS 入口、技能、调度器和模拟机械臂。IsOS 在 EI 所有者套接字上发送 `{core:"ei",op:"activate",run_id,revision,source_node,event_id,kind}`。EOS 预留命令后，成功提交返回事件 ID、命令 ID 和 `accepted` 状态。重复 ID 返回已记录状态，不会再次运动。浏览器的提交请求不等待机械臂运动完成。

公共提交成功响应将 EI 所有者结果放在 IsOS HTTP 信封中：

```json
{
  "schema_version": "0.1.0",
  "ok": true,
  "data": {
    "ok": true,
    "run_id": "run-1",
    "revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "event_id": "click-1",
    "command_id": "command-1",
    "status": "accepted"
  },
  "error": null
}
```

`POST /api/isos/v1/nodes/events/result` 读取关联结果，不会重新提交运动。准确请求是 `{schema_version:"0.2.0",source_node,run_id,active_revision,event_id}`，身份与提交相同。IsOS 验证活动运行、已保存节点图和声明路径，向 EI 查询结果，并在回复前重新检查运行。响应包含 `event_id`、`command_id` 和 `status`：`accepted`、`running`、`succeeded`、`failed`、`cancelled` 或 `outcome_unknown`。只有 `succeeded` 包含六个有限的 `joint_positions_rad` 值。IsosUI 读取此接口，直到得到终态或达到有界等待时间；后续查询继续使用相同事件 ID。

```json
{
  "schema_version": "0.2.0",
  "source_node": "isos/ui/button/trigger",
  "run_id": "run-1",
  "active_revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "event_id": "click-1"
}
```

提交字段无效返回 `400 node_event_invalid`，请求体过大返回 `413 node_event_oversized`，运行或图过期返回 `409 run_fence_stale` 或 `graph_fence_stale`，路径无效返回 `422 node_route_invalid` 或 `node_edges_invalid`。结果查询无效时按情况返回 `400 node_result_invalid`、`413 node_result_oversized`、`422 node_route_invalid` 或 `503 owner_result_unavailable`、`owner_result_invalid`。所有者传输故障或提交结果不确定时返回 `503 owner_outcome_unknown`；考虑再次提交前，使用同一事件 ID 查询结果。Stop 在工作进程退出前关闭准入。

<a id="control"></a>
<a id="control--浏览器原语控制与结果契约"></a>
## 浏览器原语控制与结果契约

[English](../../../../docs/isos/interfaces/api/client.md#control) · [设计](../../core/ui/webui_cn.md) · [首次使用](../../core/ui/webui_cn.md) · [EOS 所有者契约](../server/eos_cn.md#control_cn)


<a id="control--类型化边界"></a>
### 类型化边界

浏览器客户端只通过本地或托管构建配置的 IsOS 回环地址调用 `POST /api/isos/v1/owners/eos/control`。IsOS 将有界公共请求转发给选中的 EOS 桥。请求使用 JSON、`credentials: omit`、无 Authorization 标头和现有四秒客户端超时。浏览器不发送 `authority_grant`、私有 `primitive_result` 或原始 Dispatcher 操作。IsOS 校验 Host 与允许的 Origin；EOS 桥保留自身准入，并自行添加运行范围内的操作员授权。客户端为受支持的活动绑定提供操作员点击触发的 `primitive_typed` 发送方与关联结果读取方。浏览器 `op: "result"` 只读，桥接层将其改写为私有操作。

下表以 `base.velocity` 请求为例；机械臂命令使用活动绑定声明的对应注册身份和类型化负载。

| 发送字段 | JSON 类型／单位 | 用途、默认值与校验 |
| --- | --- | --- |
| `op` | 固定值 `primitive_typed`／— | 必填且严格匹配；无默认值。 |
| `header.data_type_id`、`data_type_version`、`codec` | 字符串／注册身份 | 此 base 部分必须分别为 `base_movement`、`1.0`、`json_v1`；无默认值。 |
| `header.primitive_name`、`primitive_version` | 字符串／原语身份 | 必须与活动绑定完全一致；本部分支持 `base.velocity@1.0.0`。 |
| `header.source_path`、`target_path` | 字符串／规范图节点路径 | 必须分别为活动绑定及其唯一图连线 base 目标；不能从可编辑目标输入获取。 |
| `header.active_revision` | 字符串／保存图 SHA-256 | 必须为当前 IsOS 保存及活动修订，64 位小写十六进制；无默认值。 |
| `header.command_id` | 字符串／命令身份 | 每次调用新生成，UTF-8 长度 1–128 字节且满足 EOS Dispatcher 语法；超时后仍保留。 |
| `header.sequence` | 无符号整数／命令顺序 | 在当前运行内为所选绑定生成的正安全整数；无默认值。 |
| `header.deadline_monotonic_ns` | 无符号整数／纳秒 | EOS `time` 响应加上有界期限，绝不使用浏览器墙钟；发送前拒绝不安全 JSON 整数或溢出。 |
| `header.payload_length_bytes` | 无符号整数／UTF-8 字节 | 解析后的载荷紧凑 JSON 序列化精确长度，范围 1–16,384；不能以 JavaScript 字符串长度代替多字节长度。 |
| `payload` | 类型化对象／SI 单位 | 严格为 `body_velocity`，有限的 `x_m_s`、`y_m_s`、`yaw_rad_s` 不超过活动硬件限制，正整数 `valid_for_ms` 不超过租约上限。 |

公开发送请求恰好包含 `op`、`header`、`payload`；桥接层完成公开校验后添加私有授权。收到匹配的 EOS `{"ok":true,"result":...}` 响应时，使用与结果读取方相同的六状态解码。ID 缺失／不符、未知状态或损坏响应属于协议错误；传输故障表示提交结果不可用。浏览器不能重试可能已被接受的运动。运动目标须为模拟设备或通过 EOS 停止边界验证的设备；逐绑定控件覆盖受支持的底盘与机械臂绑定。

浏览器在公开请求中用 `TextEncoder(JSON.stringify(payload))` 计算 `payload_length_bytes`。EOS 桥接层验证所声明长度及解析后载荷的界限，再为私有 `serde_json` 运行时封装规范化长度。这使 JavaScript 与 Rust 紧凑数字写法不同的有效小 SI 数值仍可使用。

| 客户端字段 | JSON 类型／单位 | 用途、默认值与校验 |
| --- | --- | --- |
| `op` | 固定值 `result`／— | 必填且严格匹配；无默认值。 |
| `command_id` | 字符串／命令身份 | 必须与提交的 `primitive_typed` 命令头及 EOS 首次响应关联，UTF-8 长度为 1–128 字节；无默认值。 |
| `active_revision` | 字符串／保存图 SHA-256 | 必填 64 位小写十六进制，取自当前 IsOS 活动修订；每次读取时须与保存图修订相等；无默认值。 |
| `result.command_id` | 字符串／命令身份 | 必须等于本地跟踪的命令 ID；不符属于协议错误，不是命令结果。 |
| `result.status` | 枚举／— | 必须严格为 `Completed`、`Failed`、`Cancelled`、`Rejected`、`InProgress` 或 `OutcomeUnknown`；未知状态属于协议错误。 |
| `result.reason` | 字符串或 null／UTF-8 文本 | EOS 报告的解释，最多 512 UTF-8 字节；只当文本显示，不渲染 HTML，也不用于控制决策。 |

```json
{
  "op": "result",
  "command_id": "move-1",
  "active_revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
}
```

结果读取服务在 fetch 前校验请求，只接受 `{"ok":true,"result":{"command_id":...,"status":...,"reason":...}}`，且 ID 相符、字段有界。EOS 将整个成功响应限制为 4 KiB。成功响应损坏、未知状态、ID 不符或响应过大均显示为协议错误，绝不能显示为 `Completed`。所有者固定错误仍是错误：请求无效（`400 invalid result request`）、Origin 或 Host 无效（`403`）、操作员授权不可用（`503 operator control unavailable`）、私有授权拒绝（`operator control grant required`）、修订过期（`stale active revision`）、其他运行／角色／入口／类型或不存在 ID 的统一不透明错误（`unknown command ID`）、有界结果不可用（`result unavailable`）以及 IPC 离线（`502 runtime unavailable`）。UI 不能根据 `unknown command ID` 猜测隐藏原因。

<a id="control--运行与修订围栏"></a>
### 运行与修订围栏

首次类型化发送之前，读取 IsOS 图和 `process-status`，要求：图支持编辑、无未保存草稿、`state:running`、已知当前 `run_id`、64 位十六进制保存修订与图文档修订及活动修订相同、活动绑定严格且只有一个兼容目标、EOS 就绪且目标健康。每次结果读取前重新获取 IsOS 进程状态，要求已知运行 ID 和活动修订与发送时记录相同、`state:running`，且保存修订不变；绑定或健康变化不抹除已准入的命令。IsOS 无法证明运行身份、状态不可用、新运行或新活动修订出现时，停止自动读取，把旧命令标记为**本运行无法获取结果**。保留最后一个所有者状态和命令 ID 供排查，但不带入新运行，也不改成 `Completed`。草稿修改或图只读变化禁止新发送，不能为已有命令造出终态。

命令记录键为 `(run_id, active_revision, binding_path, command_id)`：`run_id` 是 IsOS 当前部署运行精确 ID，`active_revision` 是 64 位十六进制摘要，`binding_path` 是活动图节点路径，`command_id` 每次调用只生成一次。只有当前选中的记录能更新其可见状态。页面卸载、绑定选择变化、运行／修订变化或 Stop 时取消未完成的浏览器读取，并用代际标记忽略迟到响应。旧代响应不能覆盖新命令。不能在浏览器存储中把命令 ID 或结果保存为权威。只有 EOS 所有者事件属于捕获的运行与命令，且提供长度 1–64、仅含 ASCII 字母／数字／`_`／`-` 的有效 IsOS 关联 ID，当前运行日志入口才能预填 `correlation_id`；否则单独显示命令 ID，打开无筛选的当前运行日志。绝不能假设 `command_id` 等于 `correlation_id`。

<a id="control--可见状态与-stop"></a>
### 可见状态与 Stop

`sending` 只是浏览器本地状态，不触发成功脉冲。EOS 响应 ID 匹配后改为精确所有者状态。`InProgress` 不是终态；`Completed`、`Failed`、`Cancelled`、`Rejected`、`OutcomeUnknown` 是显示终态。`Failed` 显示明确失败标签和原因。`OutcomeUnknown` 显示明确不确定标签，绝不触发运动命令重试。发送超时或桥接断开且没有关联响应时显示**提交结果不可用**，保留生成的 ID 供排查；状态／修订／运行恢复后只能进行受保护的读取，不能重发命令。后续读取遇到不透明 ID 或离线时显示**结果不可用**，保留上次已知的 `InProgress`，不改成终态。

结果读取每条命令最多一个进行中请求，一秒后开始；连续读取错误退避，上限十秒；自动轮询满六十秒或十二次（先到者）即停止。文档隐藏时暂停，恢复时先复查 IsOS 运行与修订；手动刷新也执行相同检查。UI 显示已暂停的 `InProgress` 记录和刷新操作，不能暗示完成。切换页面和 Stop 会中止读取。这些限制只适用于只读结果查询，绝不用于再次提交运动。

部署 **Stop** 即使 EOS 离线、图只读、活动修订不符、结果未知或客户端暂停，也要通过 IsOS `/api/isos/v1/process-control` 保持可见可用。点击 Stop 只是待处理意图，直到 IsOS 报告其受治理的终态或未知结果；UI 不能把命令 `Cancelled` 等同物理停止。工作台及其浮动面板始终提供 Stop 入口。

Stop 响应包含 `stopped`（已回收的直属子进程 ID）和独立的 `terminal` 对象或 null；它不是 `process-status` 快照。点击 Stop 时立即隔离未完成的 EOS 结果读取，并阻止在该运行中再次调用。收到 IsOS 响应前显示等待。只有格式正确的 `terminal.outcome:"confirmed"`、`terminal.run_id` 与点击时捕获的已知运行相同、原因为 null 且子进程结果均已确认时，才显示**停止已确认**。格式正确的 `unknown` 以告警显示固定 `reason_code`。`terminal` 为 null、缺失、格式错误或运行不符时显示**停止未获确认**；若权威状态为 `run_id_state:none` 且 `terminal:null`，则显示没有需要停止的受管运行。传输失败显示**停止响应不可用**，不会自动重试。收到响应后，另行读取新的 `GET /process-status` 更新实时运行与修订；刷新失败不能抹去已知 Stop 结果。Home、Graph 和 EOS 的 Stop 控件采用相同解释。

重复的子进程 ID 不能构成格式正确的终态。即使从未运行，`terminal:null` 且 `stopped` 非空也属于未确认。明确的 IsOS API 错误显示为**IsOS 拒绝停止**并附带所有者消息，与传输不可用分开。

`stopped` 中的每个 ID 都必须出现在 `terminal.processes` 中；确认终态若缺少对应子进程记录，仍显示未确认。

<a id="control--按绑定节点控制"></a>
### 按绑定节点控制

IsOS 拥有节点图字节、保存和活动 SHA-256 修订、运行 ID 与部署 Stop。EOS 拥有活动绑定、目标准入、适配器实时状态、日志和终态结果。IsosUI 只读取所有者状态，并仅通过 IsOS 的 `POST /api/isos/v1/owners/eos/control`，由 IsOS 转发到 EOS 准入 发送操作员点击的命令。图连线只声明绑定，不执行命令。


鼠标在节点上停留约 550 毫秒、键盘聚焦或触摸信息按钮可打开唯一的非模态信息卡。点击或拖动节点时，该节点位于相邻节点之上；原来显示在节点下方的详情卡已移除。卡片通过 body portal 浮于重叠节点上方，限制在视口内（边距 8 px，标称宽高最多 340 px）；Escape 或关闭按钮可退出。信息按钮使用 React Flow 的 `nodrag`／`nowheel`，节点拖动与连线锚点继续可用。卡片分别显示目录类型描述与实例描述、输入／输出名称、精确数据类型 ID、目录提供的 `x-unit` 单位、传输方式和端口帮助。缺失的目录描述明确标为兼容回退。仅当 IsOS 报告运行中、保存修订等于活动修订且图无未保存编辑时，EOS 硬件节点才读取 EOS 的 `status` 与 `health`；读取失败时不编造健康状态。卡片打开且页面可见时，每两秒刷新所有者状态／健康；读取失败或样本超过五秒时标为不可用或过期。草稿节点不显示推断的实时状态。

<a id="control--活动绑定与请求"></a>
#### 活动绑定与请求

EOS 工作台在页面可见时刷新归属方节点图／目录及进程状态。五秒定时器同一时间最多执行一次读取；即使后续定时器触发，较慢的读取仍可发布。新保存修订或运行 ID 出现时，在显示新状态前清除上次运行的发现结果和健康数据。操作员点击 **刷新** 会立即禁用 Invoke。若正在读取，多次点击合并为一次紧接着执行的读取，旧结果不再发布；只有后续读取的已保存图、所选目录、进程状态及 EOS 发现结果一致完成后，Invoke 才重新开放。归属方故障或修订不匹配时控件保持只读。卸载后忽略迟到回复。

发送前要求 IsOS 的 `run_id_state:known`、非空运行 ID、`state:running`、64 位十六进制保存修订与活动修订及所显示修订一致，且没有草稿修改或兼容只读状态。随后重读保存图与进程状态，验证相同的精确绑定和目标，并在读取 EOS 进程单调时间前完成最后一次 IsOS 运行／修订围栏检查。读取时间后，立即再次检查最新的本地运行、修订、绑定、Stop、编辑和取消状态，然后以新命令 ID 最多发送一次 `primitive_typed` POST。四秒截止时间从最后一次阻塞的所有者读取之后开始计算；不可用或过期的发送不会以同一 ID 重放。请求头包含 `data_type_id`（`base_movement` 或 `manipulator_movement`）、`data_type_version:1.0`、精确原语名／版本、规范绑定／目标路径、活动修订、新生成的 1–128 字节命令 ID、正序号、四秒单调截止时间、UTF-8 JSON 负载长度（1–16384 字节）和 `codec:json_v1`。负载为已验证的类型化运动对象。浏览器不传授权令牌或私有 IPC 操作。

浏览器先显示本地 `Sending`，然后只有命令 ID 相同的 EOS 响应才能确认 `Completed`、`Failed`、`Cancelled`、`Rejected`、`InProgress` 或 `OutcomeUnknown`。丢失提交响应时绝不再次提交。`InProgress` 仅使用同一公开桥接上的 `{op:"result",command_id,active_revision}`，每次读取前后检查 IsOS 运行／修订。自动读取最多十二次或一分钟，此后提供手动刷新。运行轮换、修订过期、IsOS 身份不可用、Stop、绑定切换或卸载都会隔离旧读取和迟到回复。传输错误显示不可用，绝不显示成功。即使 EOS 调用被禁用或桥接断开，IsOS Stop 按钮仍可见且可调用。点击后在等待 Stop 响应前，EOS 工作台设置按运行 ID 隔离的围栏，所有绑定卡立即禁用 Invoke。待定、未知或不可用的 Stop 与状态刷新失败都保持该运行被围栏隔离；同时取消其他绑定卡的待定结果读取、封存该运行的记录并忽略迟到终态；新观察到的运行 ID 使用独立围栏。

<a id="graph_loading"></a>
<a id="graph_loading--节点图加载与状态展示"></a>
## 节点图加载与状态展示

浏览器记住上次打开的图库文件名。打开工作台时优先选择该文件，否则选择 `demo` 或首个可用文件。若记住的文件已消失，界面会提示回退选择；若图库已空或当前文件消失，保存时要求新文件名。保存使用打开文件时的修订校验，并提示会替换同名文件。应用选择图库文件的精确已保存修订并重新生成所有者配置；保存并运行依次执行保存、应用、运行。

IsOS 拥有已保存的节点图字节、修订、目录、软件包收据、就绪状态和进程状态。浏览器只保存草稿和所有者报告的展示状态。编辑器仅在所选节点图与目录快照的 `graph_revision` 一致时渲染并允许编辑；不支持或修订不一致的快照保持只读。

打开节点图及明确点击重新载入时，等待所选节点图和目录快照。快照通过修订与编辑代数检查后即可显示画布。软件包清单、进程状态和核心就绪状态随后独立加载；其中一项延迟或失败不会把有效节点图标为不可用。迟到的可选响应不能覆盖较新的节点图请求、编辑代数或状态轮询。明确重新载入期间保留原画布；本地编辑或更新的目录请求会丢弃迟到的快照。

Graph 页面重新获得浏览器焦点或变为可见时，会刷新全局与所选类型目录及已安装软件包清单，使 CLI 安装的软件包出现，同时保留草稿图字节。节点库搜索跨所有核心和层匹配类型身份、位置、描述与精确软件包版本；选择结果后会选中该精确类型。快捷添加在所有软件包中搜索，并显示每个精确版本。旧版软件包进程类型不出现在两个新建节点菜单中；已保存的图仍可读取。

首页在打开、明确点击刷新以及修改后，独立发起节点图、软件包、设置、就绪状态、进程状态和快速系统健康请求。进程状态与 EOS/EI 归属方服务健康一旦返回就立即显示，不等待节点图、设置或就绪状态；其中一项失败也不丢弃其他结果。系统健康显示可访问的归属方服务及其报告的运行／修订；与 IsOS 所选运行不一致时明确提示。它不代表部署就绪或 Stop 证明。一个共享 WebSocket 向可见的 IsOS 视图提供后续状态变化。仅当流不可用时，首页在可见期间每 1.5 秒独立轮询进程状态和系统健康，每种请求最多保留一个在途；回退轮询不会重新检查节点图。根界面在流报告保存修订变化时刷新节点图；流不可用时每 30 秒检查一次。迟到的轮询或刷新结果不能覆盖较新的刷新或部署操作。首页的 Run 和 Restart 发送页面已显示的保存修订；IsOS 在启动归属方前检查该修订和节点图准入。Apply 检查所选节点图修订与目录权限。这些界面规则不改变 IsOS 的验证、Apply 或命令准入。

浏览器的可见视图共用一个[运行状态 WebSocket](http_cn.md#http_runtime--get-apiisosv1runtimestatusws)。标签页隐藏时关闭套接字，重新可见时按有上限的延迟重连，流不可用时改用 HTTP 状态读取。状态快照不替代命令时验证或 Stop 证明。

图输入绑定可以包含端口级 `desired_rate_hz` 范围，其正数 `min` 和 `max` 均可省略。节点检查器编辑输入端口的范围，并显示各源输出端口在注册类型中可选声明的官方 `frequency_hz`。处理节点可以为自己的输出声明不同的预期频率。省略边界表示不要求该限制；编辑器不设置连线频率。连线检查器显示两端的端口声明。IsOS 在检查或保存节点图时验证期望范围。硬件检查器将 `adapter.settings.frequency_hz` 作为最大 1000 Hz 的正 JSON 数值编辑，保存相机时保留数值类型。

<a id="log_cursors"></a>
<a id="log_cursors--当前运行日志导航p07-ui-契约"></a>
## 当前运行日志导航——P07 UI 契约

[English](../../../../docs/isos/interfaces/api/client.md#log_cursors) · [工作台设计](../../core/ui/webui_cn.md) · [首次使用](../../core/ui/webui_cn.md)


<a id="log_cursors--选择与顺序"></a>
### 选择与顺序

打开 Home 日志时先读取 IsOS 状态。可选的 `GET /api/isos/v1/logs?view=current` 响应以 `scope` 提供当前管理器会话或部署运行。任何运行准入前，`scope.kind` 为 `manager_session`，锚点等待跨所有来源的首条类型化包或 Apply 故障；Run/Restart 启动前的包故障合格，只有 Validate 的故障不合格。一旦运行准入，IsOS 将 `last_run_id` 持久保留为 `deployment_run` 范围，穿越停止、故障、PID 消失、活动修订清空和管理器重启；只有下次准入运行才替换。P07 没有清除端点。它的锚点等待跨所有来源的首条保留就绪或故障。不能从 PID、活动修订或最新日志行推断范围。服务端锚点按 IsOS 接收事件的序号排序，不能按客户端时间戳排序。`anchor.state` 为 `pending` 时，`anchor.event` 为 null 且 `lines` 为空；用绑定运行的游标继续轮询，并显示等待状态。`unavailable` 与 `log_anchor_unavailable` 表示首事件无法持久化或恢复；应显示明确的丢失提示，而非永远等待。找到后，在任何来源、级别、节点或关联筛选前显示锚点；筛选后的详情 `lines` 严格从锚点之后开始，操作员缩小列表时仍将其固定。管理器会话中的启动前失败不能被改标成旧的活动运行。正常的筛选后 `lines:[]` 响应若推进 `next_cursor` 且没有诊断，仅表示所扫描范围内没有匹配详情事件；显示无匹配状态并继续轮询。只有 `anchor.state:unavailable` 或类型化丢失诊断才显示不可用或缺口警报。不能从未筛选的页面猜造首事件。



| UI 输入或所有者字段 | JSON 类型／单位 | UI 校验与用途 |
| --- | --- | --- |
| `view` | 固定 `current`／— | P07 必需；其他值由 IsOS 报错。 |
| `source`、`level` | 来源 ID 或 `all`；级别枚举／— | 默认 `all` 和 `info`；仅筛选详情，不能用于选锚点。 |
| `node_path` | 规范保存图节点 JSON Pointer／— | 可选精确筛选，最多 512 UTF-8 字节；请求前拒绝错误或成员指针。拒绝时在 `/query/node_path` 返回 HTTP 400 `log_node_path_invalid`。 |
| `correlation_id` | ASCII ID／— | 可选精确筛选，1–64 个字母、数字、`_` 或 `-`；不是自由文本搜索。拒绝时在 `/query/correlation_id` 返回 HTTP 400 `log_correlation_id_invalid`。 |
| `limit` | 整数／事件 | 默认 100，范围 1–200；请求前限制 UI 输入。 |
| `cursor`、`next_cursor` | 不透明 ASCII 字符串／位置 | 输入游标最多 512 字节；保持相同查询并原样传回返回值。不解析，也不跨运行保存；筛选变化时重置。复用筛选不匹配的游标时返回 HTTP 400 `log_cursor_filter_mismatch`，路径为 `/query/cursor`；损坏或伪造的游标在同一路径返回 `log_cursor_invalid`。固定诊断不回显令牌或筛选值。 |
| `process-status.run_id`、`run_id_state` | 字符串或 null；枚举／运行身份 | `run_id_state` 为 `none`、`known` 或 `unavailable`；仅 known 有 1–64 ASCII 字节的末次准入运行 ID。none 允许管理器范围；unavailable 阻止当前运行声明。 |
| `anchor` | 对象 `{kind,state,event}`／— | 必需；待定／不可用的 event 为 null，找到时是完整脱敏 P06 事件。固定显示于全部筛选器上方。 |
| `lines`、`rotated` | 事件数组；布尔值／— | 至多 `limit` 条同范围且严格晚于锚点的事件；仅 `rotated` 不代表丢失。 |
| `diagnostics` | `{code,count}` 数组／丢失事件 | code 为上述四种稳定值；count 为非负整数或未知时 null。丢失或运行改变要显示持续可访问的提示。 |

<a id="log_cursors--行错误与上限"></a>
### 行、错误与上限

每条已接收记录展示 `timestamp`（UTC 毫秒）、`level`、`source`、`core`、可选的保存图 `node_path`、稳定的 `event_code`、所有者 `run_id`、可选的 `correlation_id`、脱敏 `message` 以及可用时观测到的 `pid`。仅将字段呈现为文本，不渲染 HTML 或原始子进程 stderr。可在折叠区显示有界类型化 `fields`。旧记录缺少字段时标记为不可用，不推断。即使最新事件写着 ready，IsOS 状态仍是就绪和停止的权威。

同一时刻最多一次轮询；每次最多请求 200 行，IsOS 每页最多扫描 64 KiB，保留最多 1,000 条显示行；隐藏页面或关闭 Follow 时暂停，失败后依次等待 2、5、10 秒，之后最多等待 30 秒；页面重新可见时立即核对。短暂收集器故障时保留末次游标与行，以非颜色 `role=alert` 显示错误并重试。恢复时先重新核对状态和查询身份，再继续。游标过期或保留范围丢失时，显示可访问的缺口通知（若 IsOS 提供数量／范围则展示），然后从 IsOS 给出的同运行有界位置恢复。不能静默清除缺口、把旧行重新当成新行，也不能回退到全部运行。仅在用户要求时重置来源／节点／关联详情筛选；它们不遮盖固定的首事件上下文。新运行会清除旧详情行与游标，显示命名边界，然后从新的当前页面读取自身的首事件锚点。

IsOS 在启动任何子进程前持久发布新运行 ID 和待定锚点。若 process-control 在 `/run_id` 返回 HTTP 500 `log_scope_publish_failed`，显示类型化启动前错误，不声称子进程或就绪事件。不能假定旧运行仍为当前范围：不确定的 fsync 可能让新运行持久准入，尽管没有子进程启动。重新读取 `process-status` 和 `view=current`；若 IsOS 报告幸存的新运行，则显示其边界及所有者报告的待定或不可用锚点。若身份不可用，遵循上述 HTTP 503 处理。

客户端区分空结果、无匹配事件、收集器不可用及保留缺口。呈现与无障碍行为见 [WebUI 指南](../../core/ui/webui_cn.md)。

<a id="monitor"></a>
<a id="monitor--监控分组接口"></a>
## 监控分组接口

[English](../../../../docs/isos/interfaces/api/client.md#monitor) · [节点类型](../node/README_cn.md) · [按钮事件](client_cn.md#button)

起始图通过 EOS 硬件输出、调度器监控、技能状态和接口出口节点，EI IO 输入、认知状态和治理监控节点，最终到 `isos.ui.monitor.arm@v0.2`，返回模拟机械臂的六关节观测状态。硬件输出节点是没有图输入的源节点。EOS 在本地读取同一个模拟机械臂采样器，因此图中有独立的返回链，也不会创建第二条执行器路径。

每条返回边都通过 `observation` 端口承载 `arm_monitor_state@v0.2`。载荷包含当前 `run_id`、`revision`、图中声明的观测 `source_node`（双链演示图为 `eos/hardware/mock/arm_observer`，旧图为 `eos/hardware/mock/arm`）、递增的 `sequence`、`sampled_at_monotonic_ns`、六个有序的 `joint_names`、`state`（`ready`、`degraded`、`stale` 或 `stopped`）及可为空的 `reason_code`。仅当状态为 `ready` 时，`joint_positions_rad` 才包含六个有限的弧度值；其他状态下为 null。EOS 在命令准入前检查每个 100 Hz 本地采样，EI 最多接收 20 Hz，IsosUI 最多以 5 Hz 刷新。这些速率及新鲜度限制位于图节点的高级设置中。

`GET /api/isos/v1/nodes/monitor` 使用常规 IsOS API 信封，数据为 `{state, data_type, run_id, active_revision, observation}`。部署停止时返回 `state:"stopped"`，运行标识及观测均为 null。部署运行或降级但运行标识已知时，IsOS 检查保存图的修订和完整的返回路径，然后每 200 ms 最多读取一次经过验证的 EI 快照，供所有本地 UI 读取者共享。缓存键包含部署、运行和修订；运行切换时不会返回旧快照。IsOS 在响应前再次检查运行状态。路径不完整、图已过期、需要重新读取时 owner 不可用或响应无效时均拒绝返回状态。IsosUI 在 Stop 或运行切换时清除旧值。`GET /api/isos/v1/nodes/monitor/ws` 通过只读 WebSocket 提供相同快照，`refresh_hz` 可为 1–5（默认 5），只发送变化的快照或有界错误事件，并每 15 秒发送一次 ping。浏览器用此路由进行实时监控。它与 HTTP 路由采用相同的回环 Host 和获准 Origin 准入。

当前运行使用旧图修订时，端点返回 `graph_fence_stale`（409）。IsosUI 将其说明为旧运行状态，而非编辑冲突；停止或恢复旧运行并运行已保存的节点图之前，监控保持为空。

<a id="presentation"></a>
<a id="presentation--展示描述符边界"></a>
## 展示描述符边界

本页向归属方和客户端实现者说明纯选择器使用的有类型描述符。描述符标识注册输出及一种表示形式；它不携带负载字节或端点授权。当前 HTTP 路由没有已验证的描述符注册，在输入与运行检查后返回 503。

| 字段 | 类型／单位 | 用途与验证 | 缺省值 |
| --- | --- | --- | --- |
| `node_path`、`port` | 字符串／图路径与端口 ID | 标识一个输出；都必须是有效且有界的标识。 | 必填。 |
| `data_type` | `DataType`／语义类型 | 提供选择所需的编码、媒体类型、结构及负载上限。 | 必填。 |
| `representation_id` | 字符串／标识 | 区分同一输出的不同生产者表示。 | 必填。 |
| `method` | 枚举／渲染方式 | `json`、`text`、`chart`、`image`、`canvas`、`overlay`、`audio` 之一。 | 必填。 |
| `delivery` | 枚举／传输 | `http` 或 `web_socket`；不授予访问权。 | 必填。 |
| `endpoint_id` | 字符串／不透明键 | 标识生产者路由；不是 URL 或私有授权。 | 必填。 |
| `run_id`、`graph_revision` | 字符串／运行 ID 与 SHA-256 | 将描述符限定在一个活动运行和图修订。 | 必填。 |

纯[选择器](client_cn.md#selection)检查字段形状、编码／媒体／方法匹配、负载上限及所请求的运行／修订。注册端口与类型的绑定由上游归属方准入负责；当前 HTTP 路由无法执行实时描述符选择。[HTTP 契约](http_cn.md#http_presentations)定义当前请求和错误。

[English](../../../../docs/isos/interfaces/api/client.md#presentation)

<a id="selection"></a>
<a id="selection--通用输出数据选择"></a>
## 通用输出数据选择

[English](../../../../docs/isos/interfaces/api/client.md#selection) · [呈现 HTTP](http_cn.md#http_presentations) · [发布类型](../package/README_cn.md#release)

<a id="selection--概览"></a>
### 概览

发布贡献的 `DataType` 和已注册输出端口是类型权威。发布验证会检查 ID/版本后缀；owner 注册必须先证明端口与类型绑定，才可将描述符交给选择器。IsOS 为已应用的输出选择公开呈现元数据；生产者拥有载荷字节、编码、端点授权和传送。MIME 标签或 UI 方法本身不能授权解码。不增加第二份呈现清单。

<a id="selection--类型来源"></a>
### 类型来源

[节点数据类型](../../../node/interfaces/type_cn.md#data_types)是 `id`、`version`、`purpose`、`media_type`、`codec`、`schema` 和 `max_payload_bytes` 字段的唯一权威来源。注册输出端口绑定该类型；归属方描述符再绑定节点路径、端口、表示、交付方式、不透明端点 ID、运行 ID 和图修订。字段见[呈现描述符](client_cn.md#presentation)。IsOS 只选择元数据，不取得负载字节或端点授权。

<a id="selection--表示族"></a>
### 表示族

JSON 记录和文本使用 `json` 与 `text`；标记为 `time_series` 的 JSON 时间序列使用 `chart`；JPEG/PNG 使用 `image`；有界 RGB/RGBA/depth 平面使用 `canvas`；标记为 `detections` 的 JSON 检测使用 `overlay`；Opus 或 PCM 音频使用 `audio`。Depth 需要单位，生产者帧契约还须定义无效值语义。检测需要坐标系和图像关联；音频需要采样率和声道语义。这些载荷字段属于生产者契约，IsOS 不从类型名称猜测。

<a id="selection--可用性"></a>
### 可用性

当前 IsOS 路由执行输入和 运行／修订检查后返回 503，因为选定 owner 尚未发布已验证公开描述符。精确请求和当前错误见[呈现](http_cn.md#http_presentations)。Stop、Restart 与 Apply 会使先前选择失效。

<a id="vision"></a>
<a id="vision--视觉分组接口"></a>
## 视觉分组接口

[English](../../../../docs/isos/interfaces/api/client.md#vision) · [节点类型](../node/README_cn.md)

`isos.ui.vision.monitor@v0.2` 有一个必需的 `image_frame@v0.2` 类型 `frame` 输入。该输入连接 `eos.interface.egress.vision_flow@v0.2` 的 `frame` 输出，而 EOS 传输节点的 `frame` 输入连接兼容的 EOS 相机。EOS 传输节点是标准接口节点，不是硬件适配器；相机节点负责选择图像源。Apply 和 Run 会拒绝不完整或未获准路由的路径。

`GET /api/isos/v1/vision/bindings` 返回活动 `run_id`、图 `revision`，以及每个监视器的 `{monitor_path,transfer_path,source_path,label,interval_ms}` 绑定。已停止、过期或不可用的运行会被拒绝。浏览器读取绑定后打开 `/api/isos/v1/vision/ws`，发送一条文本订阅 `{monitor_path,run_id,revision}`。IsOS 只接受活动保存图中的绑定。每条发出的二进制 WebSocket 消息为 `[u32 大端头部长度][JSON 头部][JPEG 字节]`。头部包含 `{op:"frame",run_id,revision,monitor_path,transfer_path,source_path,output_sequence,sample,width_px,height_px,encoding:"jpeg"}`；头部最多 4096 字节，JPEG 最多 20 MiB。`output_sequence` 是原生图的采集输出序列号；`sample.sequence` 是来源时钟的样本序列号，两者可以独立递增。IsOS 按所选绑定及运行栅栏检查每个数据包；图发生变化、EOS 失败、Stop 或套接字关闭时结束流。浏览器只保留最新输出序列号，并在流关闭或运行变化时清空画面。

相机按自身配置的频率采集。EOS 对精确的运行、修订、传输及来源路线准入后，通过独立且有界的工作进程数据连接推送每个新采集的 JPEG。IsOS 发送前按图中声明的交付间隔限速。软件包输出端口可选的官方 `frequency_hz` 不调度采集，也不限制交付。两个下游输入的 `desired_rate_hz.max` 中较低值在存在时限制交付；否则交付不限速。`interval_ms` 仅报告该交付上限（`null` 表示没有可表示的间隔）。低于每年一帧的上限会交付第一帧，然后关闭流。升级后的套接字必须在五秒内发送订阅。

服务器或拥有方重启期间，绑定读取失败或流关闭时，可见的视觉面板按有上限的退避间隔重试。每次重试都重新读取绑定，并在订阅前检查当前运行 ID、修订和监视节点路径完全一致。断线时清空画面，不重放控制请求。隐藏面板等到重新可见后才重试。

<a id="workspace_panels"></a>
<a id="workspace_panels--u03-工作区面板契约"></a>
## 工作区订阅生命周期

[English](../../../../docs/isos/interfaces/api/client.md#workspace_panels)

隐藏或收起的监控、视觉或日志面板停止轮询或流。重新打开时，客户端先检查活动运行和修订再继续。一个日志消费者持有该范围的游标；Stop 或运行变化使其失效。视觉显示仅使用当前运行中已接受且绑定来源的画面。面板布局和标签见 [WebUI 指南](../../core/ui/webui_cn.md)。
