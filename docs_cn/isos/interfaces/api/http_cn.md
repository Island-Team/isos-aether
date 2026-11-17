<a id="http"></a>
<a id="http--本地-http-路由"></a>
# 本地 HTTP 路由

<!-- doc-navigation:start -->
<details>
<summary>目录：isos/interfaces/api</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [客户端 API 与前端接口](README_cn.md)
  - [Client interactions](client_cn.md)

</details>
<!-- doc-navigation:end -->

## 目录

- [目录 HTTP API](#http_catalog--目录-http-api)
- [Core API v1](#http_cores--core-api-v1)
- [本地 HTTP 封套 v1](#http_envelope--本地-http-封套-v1)
- [图库 API v1](#http_graph_pool--图库-api-v1)
- [Graph API v1](#http_graph--graph-api-v1)
- [所有者控制投影](#http_owners--所有者控制投影)
- [软件包 HTTP API](#http_packages--软件包-http-api)
- [展示 HTTP 路由](#http_presentations--展示-http-路由)
- [运行时 API v1](#http_runtime--运行时-api-v1)
- [版本与健康 API](#http_version--版本与健康-api)


本页面向 IsOS 服务及本地客户端实现者。路由以 `/api/isos/v1/` 开始；请求经过本地 Host/Origin 准入后由 `app` 解码，图、软件包和运行时归属模块作出决策。[外部节点事件请求](client_cn.md#button)随节点契约记录。

- [封套](http_cn.md#http_envelope)定义回环请求准入及成功与错误响应字段，包括诊断代码、阶段和图路径。
- [版本](http_cn.md#http_version)返回 API 身份，使客户端选择受支持接口。
- [图](http_cn.md#http_graph)说明草稿验证、保存修订和 Apply；保存实例模式见[节点](../../../node/interfaces/graph_cn.md#instance)。
- [图库](http_cn.md#http_graph_pool)保存与活动部署独立的命名草稿。
- [目录](http_cn.md#http_catalog)公开注册类型与选择器选项，不更改保存的图。
- [软件包](http_cn.md#http_packages)说明围绕精确已验证锁定的安装、链接、检查和移除。
- [核心](http_cn.md#http_cores)向本地客户端公开归属方发现与就绪。
- [运行时](http_cn.md#http_runtime)说明带运行／修订围栏的 Run、Restart、Stop、状态及日志。
- [所有者控制投影](http_cn.md#http_owners)通过 IsOS 转发有界 EOS 公共操作和视觉流，EOS 仍负责准入。
- [图 Vision 监视器](client_cn.md#vision)定义 IsOS UI 显示的图绑定相机流。
- [呈现](http_cn.md#http_presentations)公开选中的输出元数据；[UI 选择](client_cn.md#selection)拥有通用显示规则。

[English](../../../../docs/isos/interfaces/api/http.md)

<a id="http_catalog"></a>
<a id="http_catalog--目录-http-api"></a>
## 目录 HTTP API

[English](../../../../docs/isos/interfaces/api/http.md#http_catalog) · [封套](http_cn.md#http_envelope) · [图](../node/README_cn.md)

目录路由公开注册节点类型、选中类型、节点选项及节点集预览。结果来自经过验证的通用 `node_contributions` 和选中所有者目录；软件包 ID 不改变解析或优先级。所有路由都需要回环 Host/Origin 准入。

贡献类型的目录条目包含其精确的 `package` `{id, version}`。不同已安装发行版贡献相同类型 ID 和位置时，目录中可以出现多个条目。图实例通过顶层 `package` 锁定选定一个定义；选择器、设置和边准入使用相同身份。未锁定或锁定不匹配的贡献实例会被拒绝，不会随意分配给某个发行版。当前 EOS 运行时注册表以类型 ID 为键；若同一个归属方的图选中两个贡献相同 EOS 类型 ID 的发行版，Apply 会拒绝该图。

| 路由 | 结果 | 状态影响 |
| --- | --- | --- |
| `GET /api/isos/v1/catalog/node-types` | 注册类型及可用性。 | 无。 |
| `GET /api/isos/v1/catalog/node-types/selected` | 已应用图引用的类型。 | 无。 |
| `POST /api/isos/v1/catalog/node-options` | 所提供草稿的声明设置、稳定资源 ID 及 EOS/EI 归属方上下文选择项。 | 无。 |
| `GET /api/isos/v1/catalog/node-sets` | 经过验证的声明节点集。 | 无。 |
| `POST /api/isos/v1/catalog/node-sets/preview` | 候选图组合；不发布。 | 无。 |

节点集预览保留已有节点。新增节点的路径或 EOS 硬件实例 ID 已被占用时，IsOS 为新增实例选择首个空闲的 `_2`、`_3` 等后缀，并将节点集内的引用改写为最终路径。响应的 `added_paths` 返回最终路径。预览仍验证完整候选图；不支持的所有者类型、适配器、路由和混合执行模式会被拒绝，也不会修复已有的无效节点。预览被拒绝时返回带有有界原因的 `node_set_preview_rejected`，而不是含糊的验证错误。

需要正文的请求使用 `schema_version:"0.1.0"`。格式错误的模式返回 400；未知精确锁定、类型、资源或所有者返回有界准入诊断。过期草稿修订返回 409。字段身份与验证细节见[图](../../../node/interfaces/graph_cn.md#instance)、[选项](../../../node/interfaces/type_cn.md#selectors)和[资源](../package/README_cn.md#resources)。

对于软件包相机，node-options 还从该准确版本且经过验证的软件包相机节点集配置提供 `adapter.settings.service_instance` 和 `adapter.settings.device_id` 选项。服务列表不固定为 RoboCasa。只有服务的软件包配置和本机前提条件通过验证时，服务才可用。选择服务会清除原设备选择；选择设备会应用其声明的坐标系与适配器配置。新添加的相机没有隐式设备。已选值若不在经过验证的软件包中，仍显示为不可用；Apply 和 Run 会拒绝未声明的相机来源。

归属方选择项分组保留 `node_path`、`setting`、草稿的 `selected` 值，以及带可用性和原因字段的有界 `options`。目录 sidecar 选择在每次请求中对每个精确软件包版本锁定只验证一次；当 EOS 或 EI 在草稿中不提供选择器时，跳过该归属方的选项预览。必要的归属方预览仍验证当前草稿。调试计时事件 `catalog_queue_wait` 和 `node_options_stage` 以有界毫秒记录排队、软件包验证、目录构建、归属方预览及总耗时，不包含软件包身份或负载。IsOS 在返回前检查归属范围和分组身份。工作台把响应绑定到当前草稿；归属方请求失败时，选择器保持不可用。草稿类型目录与选项使用同一批本地选定的 EOS/EI setup 适配器，即使持久化归属方仍运行较旧的已准入世代。预览可能过期，因此 Apply 会向实时归属方重新验证；被拒绝的 Apply 无法启动该草稿。

<a id="http_cores"></a>
<a id="http_cores--core-api-v1"></a>
## Core API v1

[English](../../../../docs/isos/interfaces/api/http.md#http_cores) · [owner 边界](../manager_cn.md)


<a id="http_cores--get-apiisosv1coresreadiness"></a>
### GET /api/isos/v1/cores/readiness

返回 `data.cores`、图与 IsOS/UI 配置就绪。它使用选定的软件包和 EI 所有者 stage 目录验证已保存图，包括每个已注册 `isos.<layer>.<name>@v0.2` 节点的位置。它描述选定安装路径与兼容性；实时就绪请查看部署状态和所有者事件。

已启用核心尚无选中生成配置时，返回 `graph_configured: false` 并提示 Apply 保存图。先前已 Apply 的图库文件以新字节保存后，`configuration_diagnostic.code` 为 `pending_apply`，Run 在再次 Apply 前继续受限。若选中的程序在 Apply 后发生变化，`configuration_diagnostic.code` 为 `generated_launch_input_mismatch`，`ready` 为 false，操作提示再次 Apply。无效的选中快照会报告其诊断，不会被误报为未保存的图。已停止但就绪的核心提示运行保存图。存在归属方层却没有对应进程节点时，会明确报告缺少进程。真正空白的图使 EOS 和 EI 均处于未启用状态，不产生错误。

对于仅报告健康的三核心图，EOS owner 的 `ready` 状态与 EI owner 的 `running` 状态都会使 `running: true`。此字段反映部署状态报告的 owner 工作进程；`ready` 字段另行描述已保存配置能否运行。若 owner 失败，部署状态变为 degraded，工作台会在浏览器日志记录受影响 owner 的状态。

<a id="http_cores--get-apiisosv1coressetup-status"></a>
### GET /api/isos/v1/cores/setup-status

返回 `data.ready`、`issues`、`graph_revision`、backend 选项及进程摘要。响应示例：`{"schema_version":"0.1.0","ok":true,"data":{"ready":false,"issues":["selected owner setup is unavailable"],"graph_revision":null,"backends":[],"process":null},"error":null}`。

<a id="http_cores--get-apiisosv1coressetuprolecatalog"></a>
### GET /api/isos/v1/cores/setup/{role}/catalog

返回已保存图对应的选定 EOS 或 EI 归属方 setup 目录，不发布配置。路由只接受 `eos` 或 `ei`；归属方不可用时返回 503。

<a id="http_cores--post-apiisosv1coressetuprolevalidate"></a>
### POST /api/isos/v1/cores/setup/{role}/validate

请求 `{"schema_version":"0.1.0","graph":{"version":"0.2.1","robot":{"id":"robot","name":"Robot"},"cores":{"isos":{"layers":{}}}},"revision":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}`。IsOS 将草稿及修订传给所选 owner，不发布配置。owner 响应含数字 `schema_version:3`、`ok`、`diagnostics`、`files` 和可选 `identity` 或 `catalog`。

<a id="http_cores--post-apiisosv1coressetuprolerender"></a>
### POST /api/isos/v1/cores/setup/{role}/render

使用与 validate 相同的请求形状，只预览有界相对生成文件，不发布。缺图返回 400，未知 role 或操作返回 404，超时、畸形响应或 setup 身份未声明 `setup_schema_version:3` 返回 503。

`schema_version` 为字符串/—且必须为 `0.1.0`；`role` 是封闭字符串/—，防止选择任意可执行文件；`graph` 为已注册 profile 对象/—；`revision` 是可选 64 位小写 SHA-256 字符串，供已有保存字节围栏的调用者使用。外部 setup wire 使用模式 3 和有界 stdin/stdout；产品版本不代表 setup 兼容。

<a id="http_cores--失败示例与字段契约"></a>
### 失败示例与字段契约

对于 `POST /api/isos/v1/cores/setup/eos/validate`，以下是结构化失败示例：

```json
{
  "schema_version": "0.1.0",
  "ok": false,
  "data": null,
  "error": {
    "code": "owner_unavailable",
    "message": "required owner is unavailable",
    "details": null,
    "correlation_id": "req-42"
  }
}
```


<a id="http_cores--验证"></a>
### 就绪与事件准入

就绪检查使用准入时选定的 EI 阶段目录。带节点路径的 EI 或 EOS 事件必须指向已保存图中归属该所有者且已获准的节点。同一修订的 Apply 可重新生成缺失的启动输入；后续就绪响应反映新生成的状态。

<a id="http_envelope"></a>
<a id="http_envelope--本地-http-封套-v1"></a>
## 本地 HTTP 封套 v1

[English](../../../../docs/isos/interfaces/api/http.md#http_envelope) · [接口目录](../README_cn.md)

<a id="http_envelope--概览"></a>
### 概览

回环 API 在 `/api/isos/v1/` 下提供八个域。每个处理器返回 JSON 封装。各域页面定义路由载荷；本页定义通用传输规则。API schema 独立于 IsOS 产品、图、包和 owner 协议版本。

<a id="http_envelope--请求准入"></a>
### 请求准入

本页面向 IsOS 回环 API 的实现者。当前监听器绑定 `127.0.0.1`，端口由 `ISOS_HTTP_PORT` 指定（默认 `16000`）。路由执行前，传输层检查 `Host` 是否与配置的回环主机及端口完全一致。提供的 `Origin` 必须是本地 IsOS 来源、配置的 IsosUI 来源或允许的托管 UI 来源。跨站 `sec-fetch-site` 请求缺少 `Origin` 时会被拒绝。当前本地请求不携带归属方 bearer 令牌。

CORS 预检需要 `Origin`；允许的请求方法为 `GET`、`POST`、`PUT`，允许的请求头只有 `Content-Type`。Host、Origin 或预检被拒绝时，在图或运行时变更前返回 403。每条路由仍需验证负载、运行围栏和图权限。[节点事件契约](client_cn.md#button)给出首个 UI 控制请求。

当预检来自获准的本地或托管 IsosUI 来源且包含 `Access-Control-Request-Private-Network: true` 时，API 返回 `Access-Control-Allow-Private-Network: true`。其他来源不会获得私有网络准入。

<a id="http_envelope--接口"></a>
### 接口

成功响应含 `schema_version:"0.1.0"`、`ok:true`、域特定 `data` 及 `error:null`。以下为系统路由实现的示例：

```json
{
  "schema_version": "0.1.0",
  "ok": true,
  "data": {
    "api_schema_version": "0.1.0",
    "isos_version": "0.1.0"
  },
  "error": null
}
```

失败响应含 `schema_version:"0.1.0"`、`ok:false`、`data:null` 和结构化错误。以下 `correlation_id` 仅为示例；服务器为每次失败生成标识。

```json
{
  "schema_version": "0.1.0",
  "ok": false,
  "data": null,
  "error": {
    "code": "invalid_request",
    "message": "invalid request",
    "details": null,
    "correlation_id": "req-42"
  }
}
```

| 字段 | 类型 / 单位 | 用途与验证 |
| --- | --- | --- |
| `schema_version` | 字符串 / 版本 | 必需的 wire 区分字段，必须为 `0.1.0`；产品版本不能替代。 |
| `ok` | 布尔值 / 无 | 仅操作成功时为 true。 |
| `data` | JSON 值或 null / 由域定义 | 域结果；失败时为 null。 |
| `error` | 对象或 null / 无 | 成功时为 null；失败时为结构化错误。 |
| `error.code` | 字符串 / 无 | 稳定的机器可读类别或域诊断。 |
| `error.message` | 字符串 / 无 | 安全的人类可读消息；不可作为协议字段解析。 |
| `error.details` | JSON 值或 null / 由域定义 | 可选的有界诊断；null 表示未提供。 |
| `error.correlation_id` | 字符串 / 无 | 服务器生成的标识，用于关联净化后的日志。 |

畸形 JSON 和无效输入使用 400，Host/Origin 拒绝使用 403，资源缺失使用 404，版本冲突使用 409，准入失败使用 422，owner 不可用使用 503。域可以返回更具体的稳定 `error.code`；客户端按代码而非消息分支。要求 `schema_version` 的 JSON 写操作在发布前拒绝其他值。声明闭合请求结构的路由会拒绝未知字段；客户端不可假定所有路由的未知字段策略相同。

内部操作失败的 HTTP 响应和服务器 `http_request_failed` 事件使用同一个 `correlation_id`，方便操作员查找原因，而无需在浏览器暴露内部细节。若 EOS 或 EI 仍持有先前的运行，Run 返回 HTTP 409、`error.code:"run_conflict"`，并在 `/owners/eos` 或 `/owners/ei` 提供诊断；若所有者提供了运行 ID 和修订，该诊断包含有界的 `run_id` 和 `revision` 字符串。确认该运行的 Stop 后再重试。这与图修订冲突使用不同的代码。

<a id="http_envelope--路由"></a>
### 路由

[版本](http_cn.md#http_version)、[图](http_cn.md#http_graph)、[目录](http_cn.md#http_catalog)、[包](http_cn.md#http_packages)、[core](http_cn.md#http_cores)、[运行时](http_cn.md#http_runtime) 与[呈现](http_cn.md#http_presentations)定义各自载荷和副作用。[节点事件契约](client_cn.md#button)定义第八个域。路由器是方法/路径对的真实来源。

<a id="http_graph_pool"></a>
<a id="http_graph_pool--图库-api-v1"></a>
## 图库 API v1

[English](../../../../docs/isos/interfaces/api/http.md#http_graph_pool) · [活动图 API](http_cn.md#http_graph)

图库是本地 IsOS 配置目录中的图文件工作区。部署绑定只记录选定的图库名称；图字节仅存在于图库。所有路由使用与图 API 相同的回环 Host 和 Origin 检查。保存已绑定文件会改变其保存修订，因此 Run 前必须通过 `PUT /api/isos/v1/graph` 刷新生成配置。保存不会启动或停止进程。

<a id="http_graph_pool--get-apiisosv1graph-pool"></a>
### GET /api/isos/v1/graph-pool

返回按名称排序的 `data.graphs` 数组，条目包含 `name`、准确字节 SHA-256 `revision` 和 `version`。图库最多接受 128 份草稿。不安全或超大条目会被拒绝。

空图库返回：

```json
{
  "schema_version": "0.1.0",
  "ok": true,
  "data": {
    "graphs": []
  },
  "error": null
}
```

<a id="http_graph_pool--get-apiisosv1graph-poolname"></a>
### GET /api/isos/v1/graph-pool/{name}

返回单份草稿的 `data:{name,revision,graph}`。名称包含 1–48 个 ASCII 字母、数字、连字符或下划线。名称不存在时返回 404。

<a id="http_graph_pool--put-apiisosv1graph-poolname"></a>
### PUT /api/isos/v1/graph-pool/{name}

保存 `0.2.1` 图文件。先从 `GET /api/isos/v1/graph/new` 返回的按钮路径与模拟机械臂观测路径在 EI 上下文汇合的十七节点演示图开始，再发送 `{"schema_version":"0.1.0","revision":"","graph":<草稿>}` 创建新名称。替换已有文件时提交 GET 返回的准确修订；编辑器会提示同名保存将替换该文件。修订变化返回 409。编辑期间文件可以不完整；Apply 要求三个核心，并验证完整图。图的大小上限为 1 MiB。

Web 编辑器从图库打开一个命名文件，并合并经修订检查的归属方选定目录与全局目录，使仅在归属方目录中的节点类型仍显示设置和端口。画布位置和上次打开的图库名称保存在浏览器本地。该文件缺失时，编辑器在保存时要求输入新名称。

<a id="http_graph"></a>
<a id="http_graph--graph-api-v1"></a>
## Graph API v1

[English](../../../../docs/isos/interfaces/api/http.md#http_graph) · [节点接口](../node/README_cn.md)

IsOS 管理已保存图及其精确字节 SHA-256 修订。所有 `/api/isos/v1/*` 路由都执行回环 Host 与 Origin 准入。JSON 修改使用 `schema_version: "0.1.0"`。成功响应为 `{ "schema_version": "0.1.0", "ok": true, "data": ..., "error": null }`；失败响应设置 `ok: false`，返回稳定错误代码、安全消息及关联 ID。过期修订返回 409，图准入失败返回 422。

`GET /api/isos/v1/graph` 从绑定的图库文件读取 `{pool_name, revision, graph, read_only, diagnostics, warnings, missing_file}`，不修改其字节。文件缺失时返回可编辑的三核心起始草稿、空修订及 `missing_file:true`；编辑器必须先以图库名称保存，才能 Apply。`GET /api/isos/v1/graph/new` 返回按钮路径与观测路径在 EI 上下文汇合的 17 节点演示图，但不保存。

`POST /api/isos/v1/graph/validate` 接受 `{schema_version, graph}`，返回 `{valid, diagnostics}`。`PUT /api/isos/v1/graph` 接受 `{schema_version, pool_name, revision}`，并返回已应用的 `{revision, graph, pool_name, warnings}`。修订必须等于命名图库文件的精确字节摘要。IsOS 验证三个核心、注册的所有者类型、静态连接、软件包锁定及所有者投影，发布生成配置，并把部署绑定到该图库文件。文件缺失返回 404；修订过期返回 409。旧调用方仍可提交 `{schema_version, revision, graph}`；此方式会更新当前绑定的图库文件，编辑器不应使用。

在图库中保存 `demo` 后，Apply 使用返回的修订。下面的摘要仅为示例；调用方从图库 GET 响应复制实际值。

```json
{
  "schema_version": "0.1.0",
  "pool_name": "demo",
  "revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
}
```

`POST /api/isos/v1/graph/connection` 预览一条草稿连接。草稿包含该输入引用后，请求携带 `{schema_version, graph, source, output, target, input}`。`source` 与 `target` 是规范的四段节点路径；`output` 与 `input` 是端口名。响应为包含准确 `source`、`output`、`target`、`input`、路由类别、通道及 QoS 字段的 `{allowed:true, edge}`，或 `{allowed:false, diagnostic}`。预览期间其他必需输入和固定绑定目标可以尚未连接。选定目录不可用时返回 503，API 版本错误返回 400。该读取不保存草稿，Apply 会重新执行完整准入。

当前图格式为 `0.2.1`，包含 `isos`、`ei`、`eos` 核心，类型 ID 为 `<core>.<layer>.<group>.<node>@v0.2`。仓库中的[混合示例](../../../../examples/mixed-mujoco.nodes.json)有 17 个节点。它保留机械臂观测和相机视觉路径，但让模拟机械臂控制路径缺少激活源，用于验证不完整链警告；该路径无法向机械臂发送命令。Apply 该示例需要已验证的 `island/mujoco-sim@0.2.4` 软件包及其配置。`/graph/new` 返回的无软件包 17 节点起始图具有连通的按钮到机械臂路径。仓库演示图另有 `island/mujoco-sim@0.2.6` 相机与视觉路径。混合示例不生成 IsOS 子进程规格。工作进程实时健康通过部署状态读取；图读取仅代表配置。

`GET /api/isos/v1/graph/convert?revision=<sha256>` 仅在修订值匹配绑定图的原始字节时，将 `0.2.1` 图转换为未保存的 `0.2.1` 草稿。转换只更改 `version`，保留节点 ID、输入、设置和软件包精确固定版本。使用相同修订值通过图库 PUT 保存；并发修改返回 409。
工作台可只读打开旧版图库文件并预览相同转换。保存操作检查该文件的修订值，随后应用操作验证并绑定转换后的图。

`POST /api/isos/v1/graph/validate` 除 `valid`、`diagnostics` 和已准入的 `edges` 外还返回 `warnings`。缺少注册的必需输入时，系统在输入指针处给出 `edge_required_input` 警告，并包含目标节点与输入名；图仍然有效且可编辑。已连接边仍须通过完整的类型与路由准入。警告不允许沿断开的路径交付运行时数据。

<a id="http_owners"></a>
<a id="http_owners--所有者控制投影"></a>
## 所有者控制投影

[English](../../../../docs/isos/interfaces/api/http.md#http_owners) · [HTTP 路由](http_cn.md) · [EOS 控制契约](../server/eos_cn.md#control_cn)

<a id="http_owners--概述"></a>
### 概述

IsOS 为 Web UI 提供选中 EOS 公共控制桥的有界投影。IsOS 检查活动运行及保存图修订，从已验证图解析桥的环回地址，并且仅转发公共操作。EOS 仍负责命令准入和执行器安全。该路由不能选择任意所有者端点或提供私有授权。

<a id="http_owners--身份验证"></a>
### 身份验证

调用方必须通过 IsOS 环回 Host 和获准 Origin 检查。请求不携带令牌或所有者秘密。IsOS 只接受运行中、运行 ID 已知、保存图字节匹配且桥进程运行中的部署。所有者桥另行执行自身准入。

<a id="http_owners--接口"></a>
### 接口

请求和响应使用 JSON。IsOS [封套](http_cn.md#http_envelope)的 schema 为 `0.1.0`；其中 `data` 是有界 EOS 公共响应，保留 EOS 的 `ok` 和操作字段。请求体最多 20,480 字节，所有者响应最多 65,536 字节。操作只能是 `discover`、`status`、`health`、`observe`、`time`、`result`、`primitive`、`primitive_typed` 或 `manual`。顶层 `grant`、`token` 和 `endpoint` 字段会被拒绝。操作专属字段与验证由 [EOS 控制契约](../server/eos_cn.md#control_cn)定义。

<a id="http_owners--路由"></a>
### 路由

<a id="http_owners--post-apiisosv1ownerseoscontrol"></a>
#### POST /api/isos/v1/owners/eos/control

IsOS Web UI 发送包含 EOS 公共操作的 JSON 对象。例如 `{"op":"time"}` 在 EOS 接受时返回 `{"schema_version":"0.1.0","ok":true,"data":{"ok":true,"monotonic_ns":123},"error":null}`。状态码 `400` 表示 JSON 格式错误或操作未列出；`403` 表示 Host 或 Origin 被拒绝；`409` 表示图修订过期；`413` 表示请求过大；`422` 表示 EOS 拒绝；`502` 表示所有者响应无效或过大；`503` 表示运行、管理器、端点或桥不可用。失败使用相同封套，例如 `{"schema_version":"0.1.0","ok":false,"data":null,"error":{"code":"owner_bridge_unavailable","message":"EOS bridge is unavailable","details":null,"correlation_id":"req-1"}}`。

<a id="http_packages"></a>
<a id="http_packages--软件包-http-api"></a>
## 软件包 HTTP API

[English](../../../../docs/isos/interfaces/api/http.md#http_packages) · [封套](http_cn.md#http_envelope) · [通用软件包契约](../package/README_cn.md)

所有路由要求回环 Host/Origin 准入。请求和响应使用 `schema_version:"0.1.0"`。安装、链接和预览不更改保存的图或活动运行。软件包 ID 是数据，不是解析器开关。

| 方法和路由 | 输入 | 成功 | 失败 |
| --- | --- | --- | --- |
| `GET /api/isos/v1/packages` | 无正文 | 精确锁定和验证状态的 `data.installed` 行。 | 403 Host/Origin；503 存储错误。 |
| `POST /api/isos/v1/packages/preview-download` | 精确 `id`、`version`。 | 选定工件的 URL、大小、SHA-256、发布者/来源、信任状态及批准摘要；不下载工件。 | 400 发行版无效或不兼容。 |
| `POST /api/isos/v1/packages/install` | 精确 `id`、`version`、预览返回的 `approval`。 | 已验证收据、路径、更新的清单。 | 400 缺少批准或批准已失效；409 锁定重复；422 契约不支持或工件不安全。 |
| `POST /api/isos/v1/packages/link` | 精确 `id`、`version`、绝对 `path`，可选绝对 `manifest`。 | 已验证链接收据和清单。 | 400 格式错误；409 锁定重复；422 不受支持或载荷变化。 |
| `POST /api/isos/v1/packages/preview-link` | 绝对软件包 `path`，可选 `manifest`。 | `id`、`version`、`ready`、有界 `issues`；不发布。 | 400 格式错误；422 软件包不安全。 |
| `GET /api/isos/v1/packages/config?id=<id>&version=<version>` | 查询参数中的已安装精确锁定。 | `data` 包含此部署和锁定的 `descriptor`、`toml`、解析后的 `values` 与 `revision`。 | 422 描述符或配置缺失、无效。 |
| `POST /api/isos/v1/packages/config/validate` | `schema_version`、精确 `id` 和 `version`，以及 `toml` 或 `values` 二者之一。 | `data.valid` 与有界 `data.diagnostics`；不保存。 | 400 编辑格式错误；422 配置缺失。 |
| `PUT /api/isos/v1/packages/config?id=<id>&version=<version>` | 相同编辑并附当前 `revision`；查询与正文身份须一致。 | 保存后的 `revision` 和 `toml`；配置变更后须在 Run 前 Apply。 | 400 格式错误或身份不匹配；409 修订过期或配置无效。 |

```json
{
  "schema_version": "0.1.0",
  "id": "vendor/sample-a",
  "version": "1.2.3",
  "path": "/absolute/bundle",
  "manifest": "/absolute/bundle/release.json"
}
```

链接成功返回 `{"schema_version":"0.1.0","ok":true,"data":{"verified":true,"source_kind":"local_link"},"error":null}`，另有契约定义的收据和清单字段。缺少必要注册声明的贡献无法通过准入。移除软件包拒绝保存或活动的精确锁定引用。发行包、收据、资源和诊断的字段以[软件包接口](../package/README_cn.md)为准。

批准摘要绑定软件包 ID、版本、来源提交与 URL，以及选定工件的 URL、大小和 SHA-256。安装处理器重新获取当前目录元数据，在下载前拒绝已变更字节对应的旧批准。CLI 安装展示相同预览，并要求终端输入 `yes`；脚本使用 `--approve-download <摘要>`。本地源码构建和本地归档是独立的显式路径。

<a id="http_presentations"></a>
<a id="http_presentations--展示-http-路由"></a>
## 展示 HTTP 路由

本契约面向请求活动图某个输出表示形式的本地客户端。路由已实现，但当前没有归属方注册已验证的公开描述符，因此有效的当前请求返回 503。[数据类型](client_cn.md#selection)和[描述符](client_cn.md#presentation)契约定义内部选择边界。

<a id="http_presentations--认证与请求"></a>
### 认证与请求

`POST /api/isos/v1/presentations/select` 要求回环 Host/Origin 准入。它接受 `schema_version: "0.1.0"` 的 JSON 主体；未知字段和畸形 JSON 均失败。共享[信封](http_cn.md#http_envelope)定义错误和关联 ID。

| 字段 | 类型／单位 | 用途与验证 | 缺省值 |
| --- | --- | --- | --- |
| `schema_version` | 字符串／版本 | 必须为 `0.1.0`；数字值以 `unsupported_schema_version` 拒绝。 | 必填。 |
| `node_path` | 字符串／图路径 | 规范的四段输出归属路径；有界并需验证。 | 必填。 |
| `port` | 字符串／输出端口 ID | 标识一个输出；有界并需验证。 | 必填。 |
| `accept` | 非空数组／渲染方法 | 客户端支持的 `json`、`text`、`chart`、`image`、`canvas`、`overlay` 或 `audio`；顺序表示偏好。 | 必填。 |
| `run_id` | 字符串／运行身份 | 必须匹配活动运行。 | 必填。 |
| `graph_revision` | 字符串／SHA-256 | 必须匹配活动修订。 | 必填。 |

```json
{
  "schema_version": "0.1.0",
  "node_path": "isos/eos_interface/gate/front",
  "port": "preview",
  "accept": [
    "image",
    "canvas"
  ],
  "run_id": "r7-cde",
  "graph_revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
}
```

<a id="http_presentations--当前结果"></a>
### 当前结果

畸形选择返回 400 `invalid_request`；模式版本错误返回 400 `unsupported_schema_version`。过期的运行或修订返回 409 `revision_conflict`。当前请求如果没有注册描述符，则返回 503 `owner_unavailable`：

```json
{
  "schema_version": "0.1.0",
  "ok": false,
  "data": null,
  "error": {
    "code": "owner_unavailable",
    "message": "presentation owner has not registered a public delivery descriptor",
    "details": null,
    "correlation_id": "req-43"
  }
}
```

当前路由不返回成功信封或端点 ID。Stop、Restart 和 Apply 会使先前运行／修订的选择失效。生产者保留字节与端点授权。

[English](../../../../docs/isos/interfaces/api/http.md#http_presentations)

<a id="http_runtime"></a>
<a id="http_runtime--运行时-api-v1"></a>
## 运行时 API v1

[English](../../../../docs/isos/interfaces/api/http.md#http_runtime) · [停止证明](../core_cn.md#core_dependent_stop) · [日志游标](../manager_cn.md#manager_log_cursors)

**IsOS 侧已实现。** IsOS 负责部署命令准入、状态和有界日志读取。所有路由需要回环 Host/Origin 准入。统一 HTTP 封装为 `{schema_version:"0.1.0",ok,data,error}`；错误包含稳定 `code`、安全 `message` 与 `correlation_id`。命令等待管理器响应；没有 owner 证明时，停止结果不得称为已确认。

<a id="http_runtime--get-apiisosv1runtimeprocess-status"></a>
### GET /api/isos/v1/runtime/process-status

返回管理器状态，包括保存及活动修订、进程状态、run ID 和保留的终态结果。响应形状示例：`{"schema_version":"0.1.0","ok":true,"data":{"state":"stopped","run_id":null,"processes":[],"packages":[]},"error":null}`。`packages` 是精确 `{id,version,path}` 图锁定数组；空数组表示未选择锁定。其他健康字段见[状态契约](../core_cn.md#core_dependent_stop)。

Run 或 Restart 重建图锁定的本地源码包并启动 owner 时，管理器仍可响应状态读取。此时状态返回 `state:"starting"`、`startup_phase`（`checking_source`、`building_source`、`validating_graph` 或 `starting_owners`），且 `run_id` 与 `active_revision` 为 null；不会把先前运行的身份显示成正在启动的身份。共享日志流会记录源码构建开始、成功、取消或失败，以及后续的图验证和 owner 启动。源码构建期间收到 Stop 后，管理器取消并回收构建进程，再返回已确认的停止结果；操作完成前状态为 `state:"stopping"`。EOS 或 EI Run 之前仍必须重建并验证源码包。同一部署同时进行的只读状态轮询共用一个待完成的管理器请求。命令完成后，owner 部分启动或不可用时返回含进程详情的 `state:"degraded"`。管理器已停止或无响应时返回 503。客户端使用 `run_id` 和 `active_revision` 约束每次读取，并在命令响应后刷新。

<a id="http_runtime--get-apiisosv1runtimestatusws"></a>
### GET /api/isos/v1/runtime/status/ws

浏览器将此只读路由升级为 WebSocket。升级时沿用 HTTP API 的回环 Host 和获准 Origin 准入；无需查询参数、客户端消息或凭据。首条事件立即发送。每条 `snapshot` 包含[进程状态](#http_runtime--get-apiisosv1runtimeprocess-status)和[系统健康](#http_version--get-apiisosv1systemhealth)的原始 `data` 形状，不带 HTTP 封装：

```json
{
  "type": "snapshot",
  "sequence": 1,
  "process_status": { "schema_version": "0.1.0", "state": "stopped", "run_id": null, "processes": [] },
  "system_health": { "service": "isos", "api_schema_version": "0.1.0", "state": "ready", "owners": [] }
}
```

IsOS 最多每 200 毫秒探测一次，只有任一投影变化时才发送新事件。`sequence` 是每条连接从一开始的正整数，不是持久游标。投影失败时服务端发送下一序号的 `type:"error"`，其中 `error:{"code":"status_unavailable","unavailable":["process_status"]}`；`unavailable` 数组可包含 `process_status`、`system_health` 或两者。恢复后发送完整快照。每 15 秒发送一次 WebSocket ping。关闭连接即停止探测；客户端重连并使用 HTTP 状态接口回退。快照报告状态，不证明节点图准入、就绪或 Stop 完成。

<a id="http_runtime--post-apiisosv1runtimecontrol"></a>
### POST /api/isos/v1/runtime/control

请求 `{"schema_version":"0.1.0","operation":"stop","revision":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}`。`operation` 是 `run`、`start`、`stop` 或 `restart`；`revision` 是可选的准确保存字节 SHA-256 围栏。成功 `data` 是管理器操作结果。过期修订返回 409；owner 失败或停止不确定返回结构化故障，不能声称成功。此路由不能绕过 EOS 执行器准入。

<a id="http_runtime--get-apiisosv1runtimelogs"></a>
### GET /api/isos/v1/runtime/logs

通过 `source`、`cursor`、`limit`、`level` 和可选当前运行 `view` 查询有界脱敏 JSONL。请求示例：`GET /api/isos/v1/runtime/logs?source=all&limit=100&level=info`；`data` 含 `lines`、`next_cursor` 和 `rotated`。EOS 和 EI 来源从 IsOS 的持久所有者日志池读取，因此新连接的 UI 可分页读取保留的启动与运行记录。每次至多 200 条与 64 KiB；轮转游标明确报告。当前运行锚点及间隙诊断见[日志游标](../manager_cn.md#manager_log_cursors)。

### GET /api/isos/v1/runtime/logs/ws

浏览器将此只读路由升级为 WebSocket，以跟踪当前运行日志。路由接受与当前日志读取相同的 `source`、`level`、`node_path`、`correlation_id`、`cursor` 和 `limit` 查询字段。`page` 消息包含与 HTTP `data` 字段相同的当前日志页，包括不透明的 `next_cursor`、范围、锚点和缺口诊断。服务端推进游标，在日志行、范围或锚点变化时发送页面；最多每 200 毫秒检查一次，每 15 秒发送一次 ping。失败时发送 `{"type":"error","status":503,"error":{"code":"log_scope_unavailable"}}`。连接升级沿用回环 Host 和获准 Origin 准入。日志面板隐藏时客户端关闭连接，重连时从最后一个游标继续。HTTP 仍用于有界历史读取和不支持 WebSocket 的环境。

`schema_version` 为字符串/—，变更时必须为 `0.1.0`；`operation` 为字符串/—且必填；`revision` 为可选字符串/SHA-256，仅调用者接受当前保存修订时可省；`cursor` 为不透明字符串/—，客户端不解析；`level` 为五种共享严重级别之一。这些字段区分保存编辑与活动运行，并限制日志读取。停止、就绪及错误可通过管理器结果和部署事件流观察。

<a id="http_runtime--失败示例与字段契约"></a>
### 失败示例与字段契约

对于 `POST /api/isos/v1/runtime/control`，以下是结构化失败示例：

```json
{
  "schema_version": "0.1.0",
  "ok": false,
  "data": null,
  "error": {
    "code": "invalid_request",
    "message": "operation must be run, start, stop, or restart",
    "details": null,
    "correlation_id": "req-42"
  }
}
```

`operation` 必须是 `run`、`start`、`stop` 或 `restart`；`run` 与 `start` 选择同一管理器启动操作。`revision` 是可选精确已保存字节栅栏。`cursor` 不透明，`limit` 是有上限的记录数。控制成功响应将管理器操作数据放入共享封装。 示例中的关联 ID 在每次失败时生成；通用封装字段和状态映射见 [HTTP](http_cn.md#http_envelope)。

<a id="http_version"></a>
<a id="http_version--版本与健康-api"></a>
## 版本与健康 API

[English](../../../../docs/isos/interfaces/api/http.md#http_version)

**IsOS 侧已实现。** IsOS 负责本地兼容身份和所有者连接概况。两个路由都需要回环 Host/Origin 准入，使用共享 schema-0.1.0 结构化 HTTP 封装，且都不启动其他 core。

<a id="http_version--get-apiisosv1systemversion"></a>
### GET /api/isos/v1/system/version

返回独立的 API schema、IsOS 包版本及内置的本地版本支持映射。响应示例：`{"schema_version":"0.1.0","ok":true,"data":{"api_schema_version":"0.1.0","isos_version":"0.2.3","version_support":{"schema":"isos.version-support@v2","product_version":"0.2.3","target_graph_format":"0.2.1","isos_http_schema":"0.1.0","package_contract":"isos.package@v1","package_schema":"0.2.1"}},"error":null}`。版本映射由 `config/version-support.toml` 编入程序，不访问 GitHub。软件包清单和精确发行元数据由 IsOS 软件包管理器提供，不来自此构建映射。客户端拒绝不支持的 `api_schema_version`；产品版本不能替代通信契约兼容检查。Host/Origin 被拒绝时返回安全的结构化 403 错误。

| 字段 | 类型／单位 | 用途与验证 | 缺省／默认 |
| --- | --- | --- | --- |
| `api_schema_version` | 字符串／契约版本 | 当前 IsOS HTTP 封装 schema，精确为 `0.1.0`。 | 必需。 |
| `isos_version` | 字符串／产品 SemVer | 已安装的 IsOS 源包版本；与 HTTP 独立。 | 必需。 |
| `version_support.schema` | 字符串／身份 | 内置映射契约，`isos.version-support@v2`。 | 必需。 |
| `version_support.product_version` | 字符串／产品 SemVer | 必须与编译的 IsOS 包版本一致。 | 必需。 |
| `version_support.target_graph_format` | 字符串／图格式 | 此构建的目标图格式，`0.2.1`。 | 必需。 |
| `version_support.isos_http_schema` | 字符串／契约版本 | 必须与响应的 HTTP schema 一致。 | 必需。 |
| `version_support.package_contract`、`.package_schema` | 字符串／软件包契约 | 当前软件包读取器接受的精确清单身份与 schema。 | 必需。 |


<a id="http_version--失败示例与字段契约"></a>
### 失败示例与字段契约

对于 `GET /api/isos/v1/system/version`，以下是结构化失败示例：

```json
{
  "schema_version": "0.1.0",
  "ok": false,
  "data": null,
  "error": {
    "code": "unauthorized",
    "message": "unauthorized",
    "details": null,
    "correlation_id": "req-42"
  }
}
```

此路由无请求体或查询字段。示例中的关联 ID 在每次失败时生成；通用封装字段和状态映射见 [HTTP](http_cn.md#http_envelope)。

<a id="http_version--get-apiisosv1systemhealth"></a>
### GET /api/isos/v1/system/health

返回 IsOS 的 `state:"ready"` 和 EOS、EI 的有界 `owners` 数组。每个可用条目包含来自所有者私有 `GET /v1/health` 的 `core`、`state`，以及可为 null 的 `run_id` 和 `revision`；不可用条目包含 `core`、`state:"unavailable"` 和原因。空闲所有者的运行身份为 null。IsOS 在请求目录或其他内部配置前公布已选所有者的身份。这一快速概况不代表部署就绪或 Stop 证明；带运行身份约束的状态应查看运行时进程状态。缓慢或缺失的所有者显示为不可用，IsOS HTTP 服务仍保持响应。
