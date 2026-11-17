<a id="core"></a>
<a id="core--核心归属方契约"></a>
# 核心归属方契约

<!-- doc-navigation:start -->
<details>
<summary>目录：isos/interfaces</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [节点、软件包、核心与 IsosUI 接口](README_cn.md)
  - [共享契约目录](contract-catalog_cn.md)
  - [归属方实现说明](manager_cn.md)
- 下级目录：
  - [api/](api/README_cn.md)
  - [node/](node/README_cn.md)
  - [package/](package/README_cn.md)
  - [server/](server/README_cn.md)

</details>
<!-- doc-navigation:end -->

这些页面供接受 IsOS 已准入节点的核心使用。核心拥有执行、生命周期适配器、健康状态和物理安全；IsOS 在交接前验证精确软件包字节、注册位置和运行围栏。以下交换承载有界身份与结果，不承载软件包特有场景或设备语义。

- [设置](core_cn.md#core_setup)告知核心就绪前必须读取哪个已验证图和配置。
- [交接](core_cn.md#core_handoff)将声明的进程、可执行文件、资源、归属方、运行 ID 和修订绑定到一次归属方接受。
- [事件](core_cn.md#core_events)将归属方状态报告给 IsOS，并携带足够身份以拒绝过期观察。
- [控制授权](core_cn.md#core_control_grants)传递归属方范围内的控制许可；图边本身不能授予驱动权。
- [依赖停止](core_cn.md#core_dependent_stop)要求终止证据，IsOS 才会将停止视为完成或允许替换。

私有管理器 IPC 和服务配置位于[开发](manager_cn.md)。

[English](../../../docs/isos/interfaces/core.md)

<a id="core_control_grants"></a>
<a id="core_control_grants--控制授权边界"></a>
## 控制授权边界

[English](../../../docs/isos/interfaces/core.md#core_control_grants) · [设计](../core/README_cn.md)

已实现的 IsOS 授权包由每次运行的私有文件组成。两个控制授权文件是随机且互不相同的秘密，仅通过保留的所有者环境路径交付。私有 `verifier.json` 具有下列精确字段。IsOS 检查仅所有者可用的目录/文件权限、无符号链接祖先、选中运行与修订及授权摘要。停止和替换撤销这些文件。EOS 负责执行器命令准入；软件包贡献不能建立另一条路径。

| `verifier.json` 字段 | 类型 / 单位 | 用途与验证 | 缺省值 |
| --- | --- | --- | --- |
| `schema_version` | 整数 / 版本 | 当前私有验证文件值 `1`。 | 必填。 |
| `run_id` | 字符串 / 运行身份 | 精确授权包代次。 | 必填。 |
| `active_revision` | 小写 64 位十六进制 / SHA-256 | 绑定保存的图字节。 | 必填。 |
| `operator_control_sha256` | 小写 64 位十六进制 / 授权字节 | 第二份私有授权的摘要。 | 必填。 |

<a id="core_dependent_stop"></a>
<a id="core_dependent_stop--依赖停止契约"></a>
## 依赖停止契约

[English](../../../docs/isos/interfaces/core.md#core_dependent_stop) · [设计](../core/README_cn.md)

当前 IsOS 从选中的所有者管道读取严格且以换行结束的终止停止收据。收据使用模式 `1`。IsOS 将收据绑定到选中 `run_id`、所有者 PID、活动修订、已准入的包进程节点及预期部分。所有者证明其后代和物理停止；IsOS 独立证明自己的直接进程组退出。证明缺失、格式错误、过期、延迟或矛盾时产生 `unknown`。

| 收据字段 | 类型 / 单位 | 用途与验证 | 缺省值 |
| --- | --- | --- | --- |
| `schema_version`, `type` | 整数 `1`；字符串 `eos_terminal_stop` / 身份 | 当前精确接受的所有者收据格式。 | 必填。 |
| `run_id`, `active_revision` | 字符串 / 运行与 SHA-256 | 必须匹配选中运行和图修订。 | 必填。 |
| `eos_pid`, `backend_node` | 整数；图路径 / 所有者与节点 | 必须匹配已准入所有者进程及后端节点。 | 必填。 |
| `backend_pid`, `backend_pgid` | 整数或 null / 进程 ID | 绑定已准入后代进程组。 | 字段必填；值可为 null。 |
| `generation_id`, `stop_epoch` | 字符串或 null / 所有者代次 | 所有者提供的停止关联。 | 字段必填；值可为 null。 |
| `admission_closed`, `backend_group_drained`, `backend_reaped`, `sockets_removed` | 布尔值 / 停止事实 | 报告 `stopped` 前检查所有必需事实。 | 必填。 |
| `parts` | 数组 / 选中的物理结果 | 必须精确覆盖预期已准入部分。 | 必填。 |
| `outcome`, `reason_code` | 字符串及可选字符串 / 终止结果 | `unknown` 保持保守。 | 结果必填。 |

物理停止仍不确定时，Restart 不能替换该运行。

<a id="core_dependent_stop--中断的三核心归属方运行"></a>
### 中断的三核心归属方运行

三核心演示在私有的 `default.owner-run.json` 收据中保存 `run_id`、`revision`、`pending`、`attempted` 和 `stopped`。普通部署 Stop 只有在 EI 和 EOS 都确认该运行停止后才删除收据。若归属方已失去该运行，IsOS 会保留收据并报告归属方不可用；旧运行已无效，因此监控读取返回 409。

<a id="core_dependent_stop--本地紧急-stop"></a>
### 本地紧急 Stop

即使 IsOS 服务未运行或归属方拒绝保存的运行 ID，CLI 命令 `isos stop` 仍可紧急关闭。它向确认属于所选工作区或绑定到其准确归属方套接字的本地进程发送 SIGTERM；进程仍存在时升级为 SIGKILL。保存的运行 ID 不是准入条件。确认没有匹配进程后，IsOS 退役过期的归属方运行收据，并记录紧急审计条目。审计仅表示进程已退出，**不证明执行器处于物理安全状态**，也不取代终止停止收据。命令显著警告此操作可能中断运行中的硬件。再次 Run 前，操作员必须独立检查硬件。

默认由操作系统的同一用户进程权限控制此命令。可选文件 `config/isos/emergency-stop.toml` 可包含 `code_sha256 = "<64 位小写十六进制字符>"`；`ISOS_STOP_CODE_SHA256` 环境变量会覆盖该值。配置后，命令要求 `--code-stdin` 并从标准输入读取口令。不匹配时，在发送信号前拒绝执行。不得把口令作为命令行参数传入，否则可能通过 shell 历史和进程列表泄露。可选口令是本地操作系统权限的补充，不会赋予关闭其他用户进程的权限。

在源码工作区运行 `isos stop` 时，工作区中的 `kill.sh` 脚本负责向进程发送信号，也支持 dry-run 预览。它选择本工作区进程，以及绑定到本工作区准确的 `eos-owner.sock` 或 `ei-owner.sock` 的归属方，即使这些归属方从另一工作区启动；其子进程也包含在内。脚本报告被发送信号的 PID。脚本确认没有匹配进程后，IsOS 才退役收据。私有的紧急停止标记使离线状态显示为 `stopped`，终止结果为 `emergency_signal_unproven`；新的 Run 会清除该标记。源码启动器会等待 Stop 的标准错误输出转发完成后才返回 shell，因此最后的警告会先于下一条提示符显示。重新启动前核实物理安全状态。

独立确认物理安全的操作员可在停止 IsOS 管理器后运行 `isos physical-stop recover --run-id <准确的 ID> --attest-safe`。CLI 要求准确、常规且私有的归属方运行收据，并要求本地扫描没有可能的进程候选。它先写入包含运行 ID、修订、收据 SHA-256 和 `operator_attestation` 的私有审计记录，然后才退役收据。缺少声明、管理器仍运行、收据变化、运行 ID 错误或存在进程候选都会拒绝恢复。此命令不向归属方发送信号，也不证明物理安全；操作员声明承担该判断。后续 Run 仍须通过正常的图、归属方与生成配置准入。

<a id="core_events"></a>
<a id="core_events--本地核心日志契约"></a>
## 本地核心日志契约

[English](../../../docs/isos/interfaces/core.md#core_events) · [运行时日志](../core/logger_cn.md)

每个核心写入一个私有 UTF-8 JSONL 文件及同名的易读 `.log` 配套文件。文本行采用 `IsOS-v0.2.3@<来源>: [级别][UTC 时间戳]<消息>`，消息中的换行会转义。EOS 和 EI 通过私有 `GET /v1/logs` 所有者套接字路由提供选定的所有者与工作进程记录。IsOS 验证这些记录并保留有界、持久的分核心日志池，供日志 API 和工作台读取。IsOS 不把日志记录当作进程健康或执行器状态的证明；健康状态仍以部署状态的 owner 回复为准。

| 字段 | 类型 | 规则 |
| --- | --- | --- |
| `timestamp` | RFC3339 UTC 字符串 | 必填；生产者时间，以 `Z` 结尾。 |
| `level` | 字符串 | 必填：`trace`、`debug`、`info`、`warn` 或 `error`。 |
| `source` | 字符串 | 必填逻辑身份：图节点采用 `<core>.<layer>.<group>.<node>@aether-v0.2.3`；系统组件采用带核心前缀的名称，如 `isos.runtime.manager.diagnostic@aether-v0.2.3`。 |
| `message` | 字符串 | 必填的有界诊断文本；不得包含秘密或原始硬件载荷。 |
| `event_code` | 字符串 | 可选的稳定机器代码。 |
| `run_id`、`correlation_id` | 字符串 | 可选操作身份，可用于关联状态及 HTTP 错误。 |
| `node_path` | 字符串 | 事件属于节点时可选的规范图路径。 |
| `fields` | 对象 | 可选的有界结构化细节。 |

示例记录：

```json
{
  "timestamp": "2026-11-09T09:24:22.340Z",
  "level": "info",
  "source": "eos.hardware.mock.adapter@aether-v0.2.3",
  "message": "Mock hardware is ready",
  "event_code": "mock_hardware_ready",
  "run_id": "r-example"
}
```

生产者路径为 `data/isos/log/deployments/<deployment>/isos.jsonl`、`data/eos/log/<deployment>/eos.jsonl` 和 `data/ei/log/<deployment>/ei.jsonl`。IsOS 从其部署日志目录下的 `isos.jsonl` 与 `owner-pool/{eos,ei}.jsonl` 提供日志。它读取有界完整行，核对来源核心与日志池，并按时间合并被接受的记录。所有者页面包含重要概况信息以及全部警告与错误；入池前将 `warning` 和 `critical` 规范为共享的 `warn` 与 `error` 级别。畸形、过大或文件被替换的记录会形成缺口诊断，不会变成可信事件。本地文件仅归属方可读；回环日志 API 继续执行 Host、Origin 检查和过滤。

<a id="core_handoff"></a>
<a id="core_handoff--绑定的软件包进程交接"></a>
## 绑定的软件包进程交接

[English](../../../docs/isos/interfaces/core.md#core_handoff) · [进程声明](package/README_cn.md#process) · [核心目录](core_cn.md)

`isos.package-process@v1` 交接是一条有界、私有且绑定运行的 JSON 消息。只有在验证精确锁定发行包、载荷、注册类型、资源和所有者能力后才写入。选定所有者必须在就绪前重验其适配器及软件包特定语义。未知所有者或适配器在启动前拒绝。

| 字段 | 类型 / 单位 | 用途与验证 | 缺省值 |
| --- | --- | --- | --- |
| `contract` | 精确字符串 / 身份 | `isos.package-process@v1`。 | 必填。 |
| `schema_version` | 精确字符串 / 版本 | `0.1.0`。 | 必填。 |
| `owner_core` | 字符串 / 核心 ID | 选中节点类型的注册所有者。 | 必填。 |
| `run_id`, `graph_revision` | 有界字符串；小写 64 位十六进制 / 运行与 SHA-256 | 把交接限制到一个已准入运行和保存的图。 | 必填。 |
| `node_path`, `node_type` | 字符串 / 规范路径与注册类型 | 标识选中的进程。 | 必填。 |
| `package` | `{id,version}` / 精确锁定 | 选择一个已验证发行包，不选最新版。 | 必填。 |
| `entrypoint`, `resources` | 已验证路径及摘要 / 文件 | 受控的可执行文件与所选资源文件；不限定软件包文件名。 | 必填；资源可为空。 |
| `entrypoint_sha256` | 小写 64 位十六进制 / 文件字节 | 固定 IsOS 验证载荷树后选中的可执行文件字节；所有者在生成子进程前核对。 | 必填。 |
| `args` | 最多 16 个字符串的数组 / 参数 | 字面量或有界的 `${socket}` 与 `${resource.<setting>.path}` 模板；不执行 shell 展开。 | 空数组。 |
| `lifecycle_adapter` | 已注册字符串 / 所有者能力 | 所有者必须在启动前接受。 | 必填。 |
| `release_sha256`, `payload_sha256` | 小写 64 位十六进制 / 字节 | 绑定被接受的清单与载荷。 | 必填。 |

依赖顺序及就绪/停止条件保留在运行时计划和所有者策略中，不是该交接的字段。它不是公开端点、图字段或执行器授权。

<a id="core_setup"></a>
<a id="core_setup--选中核心设置适配器"></a>
## 选中核心设置适配器

[English](../../../docs/isos/interfaces/core.md#core_setup) · [核心 HTTP](api/http_cn.md#http_cores)

已实现的外部所有者设置交换使用**模式 3**。IsOS 选择配置的所有者可执行程序；客户端不能提交路径。输入上限 1 MiB，输出上限 4 MiB，截止时间 10 秒。`identity`、`catalog` 与 `options` 只读；`options` 根据归属方投影返回上下文选择项；`validate` 在 Apply 前重新检查草稿；`render` 预览有界生成文件。IsOS 在发布前拒绝错误核心、不兼容身份、格式错误或过大响应、越界路径及超时。此外部通信格式独立于 IsOS 自有 HTTP `"0.1.0"` 模式。

`isos serve` 运行时，会在 IsOS 运行目录中的仅所有者可访问 Unix 套接字上启动空闲的 EOS 和 EI 服务。IsOS 通过这些套接字上的 HTTP/1.1 调用 `POST /v1/identity`、`/v1/catalog`、`/v1/options`、`/v1/validate` 和 `/v1/render`。传输请求使用 `schema_version:1`；所有者将其转换为此处的模式 3 设置操作，并返回模式 3 响应。请求只携带该所有者的图投影、已验证的软件包存储凭据、端点和修订号。所有者套接字缺失时操作失败；IsOS 不会改选其他所有者。独立的离线设置检查仍直接调用选中的适配器。

服务健康路由为 `GET /v1/health`，返回 `schema_version:1`、固定核心身份和 `state:"idle"`。目前服务对 Apply、Run、Stop、状态和事件返回 HTTP 501；在完成所有者工作进程移交之前，这些操作继续使用现有的 IsOS 进程监管和运行栅栏。

所有者以类型化诊断拒绝设置请求时，IsOS 在本地管理器日志中以 `setup_owner_diagnostic` 记录最多 32 组诊断 `path` 和 `code`。图响应保留稳定的公开拒绝代码；本地日志给出所有者报告的准确节点失配位置，且不会把任意诊断消息复制到共享游标。
直接离线调用适配器时，只有包含有效模式 3 响应、`ok` 为 false 且诊断非空，退出状态 2 才会被接受。其他非零退出，以及状态 2 却报告成功的响应，都按失败关闭处理。

```json
{
  "schema_version": 3,
  "ok": true,
  "diagnostics": [],
  "files": [],
  "identity": {
    "core": "eos",
    "setup_schema_version": 3,
    "product_version": "0.2.3",
    "graph_version": "0.2.1",
    "node_type_version": "0.2"
  }
}
```

| 字段 | 类型 / 单位 | 用途与验证 | 缺省值 |
| --- | --- | --- | --- |
| `schema_version` | 整数 / 外部所有者模式 | 请求与响应均精确为 `3`。 | 必填。 |
| `op` | `identity`、`catalog`、`options`、`validate`、`render` / 操作 | 选择一个有界操作。 | 请求必填。 |
| `core` | 字符串 / 所有者角色 | 必须匹配选中配置的所有者。 | 请求必填。 |
| `graph`, `revision` | JSON 对象与可选 64 位十六进制 / 图及 SHA-256 | 验证/渲染草稿及可选保存字节栅栏。 | 依操作而定。 |
| `ok`, `diagnostics`, `files` | 布尔值与数组 / 结果 | 封闭响应；诊断条目有 `path`、`code`、`message`；文件条目有安全相对 `path`、`content`。 | 响应必填。 |
| `identity`, `catalog` | JSON 或 null / 所有者元数据 | 按操作提供；身份具有 `setup_schema_version:3`。 | 可选。 |

对于 `options`，`catalog.setting_options` 是有界的 `{node_path, setting, selected, options}` 分组数组。每个选项含 `value`、`label`、`description`、`available`、`reason_code` 和 `reason`。`node_path` 属于请求的归属方投影；`setting` 指向已声明的归属方选择器或归属方拥有的硬件字段。归属方报告不可用的已选值，不会静默替换。IsOS 在公开结果前检查分组身份、数量上限及草稿选中值。此只读预览不授权 Apply：`validate` 和 `render` 会按当前归属方输入重新检查选中的设置。

EOS 可以为未完成的硬件实例返回适配器选项。缺少选择时 `selected` 为 `null`；此操作不验证可运行的配置。IsOS 从准确版本且经过验证的软件包节点集配置向公开目录响应添加相机服务和设备选项。设备选项可以附带该已验证实例的 `profile` 供编辑器复制；编辑器保留自身节点身份和图连线。Apply 和 Run 会重新验证最终实例。
