# 通用软件包贡献

<!-- doc-navigation:start -->
<details>
<summary>目录：isos/interfaces/package</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)

</details>
<!-- doc-navigation:end -->

## 目录

- [软件包准入与发布规则](#软件包准入与发布规则)
- [软件包发行契约](#release--软件包发行契约)
- [通用软件包资源](#resources--通用软件包资源)
- [通用进程贡献](#process--通用进程贡献)
- [已验证软件包准入](#verification--已验证软件包准入)
- [为软件包发布登录](#authentication--为软件包发布登录)


本页供软件包发布者及核心作者使用。IsOS 当前对每个软件包采用相同的精确锁定和已验证字节流程；`kind` 只是描述性元数据。软件包通过 `node_contributions` 贡献注册节点类型、资源、进程、数据类型及可选节点集。归属核心在就绪前验证负载特有语义。

- [发行](README_cn.md#release)定义软件包身份、版本、来源、工件和贡献封套；发布者从此开始。
- [软件包自带的配置表单](README_cn.md#release--软件包自带的配置表单)定义通用 Web 编辑器显示的中英文字段元数据、校验范围和模板值。
- [资源](README_cn.md#resources)定义实例无需猜测路径即可选择的稳定 ID、类别、展示元数据、包含文件和摘要。
- [进程](README_cn.md#process)定义归属方运行进程的入口、生命周期适配器、主机输入和资源引用。
- [验证](README_cn.md#verification)解释图发布或启动前进行的精确锁定、文件、目录和归属方能力检查。

[注册节点契约](../node/README_cn.md)对软件包及内置节点使用相同的类型和端口规则。[核心交接](../core_cn.md#core_handoff)把已验证进程绑定到一个运行及修订。

[English](../../../../docs/isos/interfaces/package/README.md)

## 软件包准入与发布规则

IsOS 只在所选工件及所有声明文件的字节摘要匹配后安装精确的 `{id,version}` 版本。安装使节点类型和资源可供选择，但不修改已保存的图、Apply 状态或当前运行。Apply 检查所选发行版、所有者声明及图修订；Run 重新检查已保存字节与启动输入。Stop 在注销运行前要求所有者静止证明及直属子进程退出。

`pkg update` 为本地已验证的软件包 ID 报告已批准的补丁发行版。`pkg upgrade` 重新检查所选发行版，并要求绑定工件的批准，之后将新版本与旧版本并存安装；两者都不修改图的精确锁定。正式发布要求精确版本的资格检查通过、匹配的不可变源码标签、已验证的归档字节及成功的受管重放；草稿或部分上传不算正式发布。[软件包操作指南](../../core/pkg_cn.md)说明命令和恢复步骤。

`pkg refresh-sources` 仅检查当前绑定图中锁定的 `local_source` 软件包。若记录的检出干净且 HEAD 仍与收据一致，就复用已验证的安装字节；否则从干净检出重建。明确执行 `pkg install --source` 会强制构建。从目录安装的发行包字节保持不变。`install_dev.sh` 默认调用该检查；`--no-source-refresh` 跳过该步骤。

<a id="release"></a>
<a id="release--软件包发行契约"></a>
## 软件包发行契约

[English](../../../../docs/isos/interfaces/package/README.md#release) · [验证](README_cn.md#verification) · [资源](README_cn.md#resources)

`isos.package@v1` 是活动的 IsOS 贡献身份。新发行清单使用 `"schema_version":"0.2.1"`；现有精确锁定的 `0.2.0` 和 `0.1.0` 仍可读取。`node_contributions` 声明注册节点类型、Linux `.so` 库、外部接口、路由、进程、资源、数据类型和节点集。IsOS 仅通过精确 `{id,version}` 锁定读取发行包。选中的软件包类型在 Apply、Run 或 Restart 前必须由该已验证发行包声明；运行时服务的类型列表本身不会注册类型。含有通用 EOS 硬件名称的历史发行包仍可读取，但 Apply 需要实际的已验证类型声明。

| 字段 | 类型 / 单位 | 用途与验证 | 缺省值 |
| --- | --- | --- | --- |
| `contract` | 字符串 / 身份 | 必须为 `isos.package@v1`。 | 必填。 |
| `schema_version` | 字符串 / 版本 | 新发行使用 `0.2.1`；现有锁定可读取 `0.2.0` 和 `0.1.0`。 | 必填。 |
| `id`, `version` | 字符串 / 精确软件包锁定 | 规范包 ID 和精确发行版本；不推断最新版。 | 必填。 |
| `kind` | 字符串 / 目录类别 | 用于发现的有界稳定描述符；仅为元数据，不选择行为或解析器。 | 必填。 |
| `source_commit` | 40 位十六进制字符串 / Git 提交 | 发布来源；独立于载荷摘要。 | 必填。 |
| `description`, `license`, `source_url`, `release_url` | 字符串 / 显示文本或 URL | 仅用于目录元数据；不能授予运行时能力。 | 可选，缺省为空。 |
| `contracts` | 字符串数组 / 身份 | 声明外部能力；所有者准入仍需验证。 | 可选，缺省空数组。 |
| `compatibility` | 对象 / 适配边界 | `isos_min_inclusive` 与 `isos_max_exclusive` 是 SemVer 边界；`required_owner_capabilities[]` 包含唯一的 `{owner,capability}` 对，所有者为 `isos`、`ei` 或 `eos`。IsOS 拒绝不兼容的已安装构建；选中的所有者须在启动前声明所需能力。 | 既有发行包可省略。 |
| `node_contributions` | 对象 / 声明 | 类型、库、进程、数据类型、路由、桥接和节点集；IsOS 验证结构与所有权。 | Apply/Run 必需。 |
| `assets` | 数组 / 候选工件 | 按主机匹配的有界下载候选，包含 URL、字节长度和 SHA-256。 | 归档安装必需。 |

模式 `0.2.1` 新增 `node_contributions.runtime_services[]` 和 `package_config`。每项服务声明 ID、包内相对入口、有界参数（仅展开 `${socket}`）、配置服务类型和贡献节点类型 ID。`package_config` 使用 SHA-256 摘要绑定包内相对 JSON 描述符与 TOML 模板。描述符契约为 `isos.package-config@v1`，模式为 `0.1.0`；其 `package`、`host` 和 `services.<instance>` 节包含类型化字段与中英文标签和说明。本地 TOML 按部署和精确包版本存储在已验证归档之外。配置未变化时不会生成新修订；编辑被拒绝时保留已保存的修订。HTTP `packages/config` 端点返回描述符、TOML、解析值和修订号；带修订号的 `PUT` 保存，`packages/config/validate` 只校验而不保存。


发行包可在有界的所有者扩展中包含不透明元数据。IsOS 不把这些元数据解释为场景、设备、运行库或适配器权限。所属核心在宣布就绪前验证其语义。[进程契约](README_cn.md#process)定义通用贡献字段。

软件包 ID 是稳定的 `namespace/name`，版本是精确锁定。新版本拥有独立的收据和载荷，因此安装新版本不会替换旧锁定或改写已保存图。软件包类别与发布者名称不选择运行时解析器。贡献节点类型的核心前缀须与声明的所有者一致；进程只能位于自身核心的 `process` 层。历史收据仍可读取，但跨所有者的进程位置会在准入时被拒绝。

```json
{
  "contract": "isos.package@v1",
  "schema_version": "0.2.1",
  "id": "vendor/sample-a",
  "kind": "component",
  "version": "1.2.3",
  "source_commit": "0000000000000000000000000000000000000000",
  "node_contributions": {
    "node_types": [],
    "libraries": [],
    "port_routes": [],
    "bridge_adapters": [],
    "processes": [],
    "node_sets": []
  },
  "assets": []
}
```

空列表示例只说明结构；准入还要求选中的注册类型和声明的所有者能力。[CLI 发布流程](../../core/pkg_cn.md)与软件包准入分开。

<a id="release--软件包自带的配置表单"></a>
### 软件包自带的配置表单

发布者可以在已验证发行清单中加入 `package_config`，让 Web 软件包详情显示配置字段。`descriptor` 和 `template` 都是包内相对文件，发行清单分别以 SHA-256 绑定其内容。IsOS 通过 `GET /api/isos/v1/packages/config?id=<id>&version=<version>` 返回已验证的描述符；Web 客户端无需软件包专用代码即可呈现字段。描述符提供中英文节标题、字段标签、说明及可选提示。软件包专用文案和范围应放在这里；变更后须发布新的精确版本。已安装版本继续使用自身原有的已验证描述符。

JSON 描述符包含 `contract:"isos.package-config@v1"`、`schema_version:"0.1.0"` 和 `sections[]`。每节有唯一的 `id`（`package`、`host` 或 `services.<instance>`）、中英文 `label` 与 `description`，以及 `fields[]`。每个字段有唯一的 slug `name`、`type`（`string`、`integer`、`boolean`、`directory`、`file` 或 `enum`）、`required`、中英文 `label` 和 `description`，以及可选的中英文 `note`。整数字段可声明 `min` 和 `max`；表单把它们用作输入范围，IsOS 在保存时校验。`default` 可填充初始空白输入，但软件包的 TOML 模板必须包含实际配置值：仅声明默认值不会将其写入 TOML 或传给所有者。文件和目录值须为绝对路径；选中服务所需路径必须存在且类型匹配。运行服务及其环境模板只能引用已声明的配置字段。

例如，软件包可在 `services.camera` 节声明 `"name":"width"`、`"min":64`、`"max":2048` 的整数字段及中英文标签和说明，并在模板的 `[services.camera]` 中设置 `width = 1280`。软件包所有者仍须按自己的相机实现验证该值。UI 和 IsOS 不赋予该字段相机语义。[软件包配置 API](../api/http_cn.md#http_packages)支持读取、校验及带修订检查的保存；配置按部署和精确包版本存储，与节点图草稿分离。配置变更保存后，需要在下次 Run 前重新 Apply。

以下精简描述符和模板展示这两个文件；发行清单须绑定其准确路径及摘要：

```json
{
  "contract": "isos.package-config@v1",
  "schema_version": "0.1.0",
  "sections": [
    {
      "id": "package",
      "label": {
        "en": "Package",
        "cn": "软件包"
      },
      "description": {
        "en": "Exact pin identity.",
        "cn": "精确版本标识。"
      },
      "fields": [
        {
          "name": "id",
          "type": "string",
          "required": true,
          "label": {
            "en": "Package ID",
            "cn": "软件包标识"
          },
          "description": {
            "en": "Must match the release ID.",
            "cn": "须与发行包标识一致。"
          }
        }
      ]
    },
    {
      "id": "services.camera",
      "label": {
        "en": "Camera service",
        "cn": "相机服务"
      },
      "description": {
        "en": "Camera settings.",
        "cn": "相机设置。"
      },
      "fields": [
        {
          "name": "width",
          "type": "integer",
          "required": true,
          "min": 64,
          "max": 2048,
          "label": {
            "en": "Width (px)",
            "cn": "宽度（像素）"
          },
          "description": {
            "en": "Frame width.",
            "cn": "画面宽度。"
          }
        }
      ]
    }
  ]
}
```

```toml
[package]
id = "vendor/camera"

[services.camera]
width = 1280
```

<a id="release--运行服务字段021"></a>
### 运行服务字段（`0.2.1`）

服务可以声明 `resources[]`，沿用进程的 `{setting,kind,descriptor}` 已验证资源目录引用。参数可以包含 `${socket}` 或匹配的 `${resource.<setting>.path}`。`environment[]` 包含有界的 `{name,value}` 项；值可以展开 `${config.host.<field>}`、`${config.service.<field>}`、`${generation_id}` 和 `${selected_nodes.settings_json}`。最后一个模板展开为以所选设备 ID 为键的 JSON 对象，只包含 `required_node_settings` 列出的设置。未知模板和保留环境变量名会被拒绝。只有 EOS 硬件节点以精确包版本和 `service_instance` 选中服务时，服务才会启动。

`runtime_services[].required_node_settings` 是可选映射：键为服务节点类型 ID，值为该类型已声明的非空设置名称列表。Apply 和 Run 时，该类型的每个已选节点都必须提供这些设置。Apply 时节点类型设置模式还会校验值类型及范围。设置保存在图节点上；修改后重新 Apply，并在下次 Run 生效。

Apply 记录所选服务实例及部署本地 TOML 的 SHA-256。Run 和 Restart 将其与选中代次比较；配置变化后必须重新 Apply。IsOS 在 EOS 就绪前按精确包版本和服务实例各启动一次服务，并在 Run 请求中向 EOS 传递私有套接字和服务代次。图作者不能填写套接字和可执行文件路径。EOS 的 Stop 证明先于 IsOS 终止和回收服务子进程；未确认的子进程退出报告为未知。

`runtime_services[].requirements[]` 项包含 `{path,kind}`。`kind` 为 `file` 或 `directory`；`path` 以 `${config.host.<field>}` 或 `${config.service.<field>}` 开头，可以带相对后缀。IsOS 对每条已选路径做规范化。对于文件，Apply 代次记录内容 SHA-256，并在 Run 和 Restart 比较，因此替换本地库后必须重新 Apply。这是通用包元数据；具体包选择需要固定的本地文件。



<a id="resources"></a>
<a id="resources--通用软件包资源"></a>
## 通用软件包资源

[English](../../../../docs/isos/interfaces/package/README.md#resources) · [发行包](README_cn.md#release) · [进程](README_cn.md#process)

经过验证的发行包可以为选择器与进程设置声明资源。描述符契约为 `isos.package-resources@v1`，具有 `schema_version:"0.1.0"`；最多包含 256 条资源记录，整体限制为 256 KiB。IsOS 限制描述符大小，验证所声明文件的包含关系与摘要，并公开稳定 ID。它不解析资源字节，也不解释所有者元数据中的关系。选定核心必须在就绪前验证语义。资源文件路径不是图设置或可执行权限。

| 字段 | 类型 / 单位 | 用途与验证 | 缺省值 |
| --- | --- | --- | --- |
| `kind` | 有界字符串 / 类别 | 指明注册的资源类别；不是硬编码软件包分类。 | 必填。 |
| `id` | 有界字符串 / 稳定选择 ID | 在同一发行包内与 `kind` 组合唯一；不含路径，可保存于图。 | 必填。 |
| `label` | 有界字符串 / 显示文本 | 面向用户的选项名；不影响准入。 | 必填。 |
| `description` | 有界字符串 / 显示文本 | 选择说明；不影响准入。 | 必填。 |
| `path` | 安全相对字符串或 null / 载荷路径 | 可选的已验证载荷内普通文件。 | 逻辑资源为 null。 |
| `sha256` | 小写 64 位十六进制字符串或 null / 文件字节 | 精确摘要，在准入时重验；与 `path` 同时出现。 | 逻辑资源为 null。 |
| `metadata` | 有界 JSON 值 / 所有者自定义 | 序列化后最多 16 KiB；IsOS 不解释，关系由所有者验证。 | 可选，缺省为 null。 |

未知 ID、重复 `(kind,id)`、字节变化、越界路径及符号链接以有界软件包诊断失败。错误保留规范图指针。过期修订在更改前失败。Apply、Run 和 Restart 前执行同样检查。

```json
{
  "kind": "configuration",
  "id": "balanced",
  "label": "Balanced",
  "description": "General profile",
  "path": "profiles/balanced.json",
  "sha256": "0000000000000000000000000000000000000000000000000000000000000000"
}
```

摘要仅展示格式，发布者必须使用真实文件摘要。

<a id="process"></a>
<a id="process--通用进程贡献"></a>
## 通用进程贡献

[English](../../../../docs/isos/interfaces/package/README.md#process) · [资源](README_cn.md#resources) · [所有者交接](../core_cn.md#core_handoff)

`node_contributions.processes[]` 条目描述软件包进程，但不授予启动权限。IsOS 解析精确且已验证的锁定，匹配注册节点类型与位置，检查所声明的资源 ID 与摘要，并选择所属核心。所有者验证其生命周期适配器。IsOS 仅监护 IsOS 自有直接子进程；其他核心监护自己的子进程。

所有者软件包进程使用注册的 `<core>.process.<group>.<node>@v0.2` 类型，实例位于 `<core>/process/<group>/<instance>`，其中 `<core>` 为 `isos`、`ei` 或 `eos`。进程实例的 `node.package` 固定精确版本，设置保存稳定资源 ID。IsOS 在预览与 Apply 时验证选中软件包；选中所有者必须接受其生命周期适配器，才准许绑定运行的交接。所有者启动并停止自己的直接子进程。

当前进程贡献使用 `implementation.kind: "managed_process"`、`lifecycle: "process"`、声明的所有者核心及 `process` 层。历史 `0.1.x` 外部进程记录保留原有名称且仍可读取，但不能通过此契约执行。

当前 EOS 所有者每张图只接受一个软件包进程。第二个固定版本的 EOS 进程会在选择启动与 Stop 后端之前被拒绝。历史跨所有者图位置仍可作为记录读取，但在执行准入时被拒绝。

图中还需选择内置 IsOS 所有者节点 `isos.process.component@v0.2`，位置为 `isos/process/component/eos`。当 EOS 类型目录报告软件包生命周期家族由内置实现提供，或可从已验证的软件包适配器库加载时，才准许使用；两者均不满足则拒绝。

可选的 `node_contributions.processes[].args` 数组最多包含 16 个命令参数，每个参数最多 256 字节。参数可以是字面量、`${socket}` 或 `${resource.<setting>.path}`。所有者将 `${socket}` 绑定到私有运行 socket，将资源占位符绑定到已验证的选定文件；未知或缺失的占位符会拒绝启动。所有者在生成子进程前再次核对入口和所选资源摘要。声明不会启用 shell 解释。

| 字段 | 类型 / 单位 | 用途与验证 | 缺省值 |
| --- | --- | --- | --- |
| `node_type` | 字符串 / 已注册的 `@v0.2` 身份 | 把进程关联到一个注册类型。 | 必填。 |
| `entrypoint` | 安全相对字符串 / 载荷路径 | 已验证的可执行普通文件；不限定文件名或软件包 ID。 | 直接子进程必填。 |
| `lifecycle_adapter` | 字符串 / 所有者能力 | 必须由选中所有者声明并接受；未知适配器失败。 | 必填。 |
| `resources` | 数组 / 设置引用 | 每个设置指向已注册资源类别和描述符；选择的 ID 在验证发行包中解析。 | 无资源时为空。 |
| `host_inputs` | 数组 / 主机需求声明 | 所有者验证命名输入；其值不进入发行包或保存的图。 | 无需求时为空。 |

绑定的 `isos.package-process@v1` 交接携带已验证可执行文件和资源、所属核心、生命周期适配器、运行 ID 及保存的修订。依赖顺序及就绪/停止条件保留在 IsOS 运行时计划和所有者策略中，不是该交接的字段。所有者拒绝、就绪失败或停止不确定均不能变成成功替换。[交接契约](../core_cn.md#core_handoff)定义通信字段。

例如，位于 `eos/process/vendor/main` 的 `eos.process.vendor.worker@v0.2` 选择已声明的生命周期适配器与已验证资源。缺失声明、软件包字节变化、适配器不可用、未知资源 ID 或进程位置错误，都会在所有者接收启动请求前拒绝准入。

<a id="process--通用技能选择"></a>
### 通用技能选择

技能库只能通过精确软件包锁定及带 SHA-256 的声明普通文件选择。IsOS 验证发行包和选中字节；所属运行时验证 ABI 和可调用语义。递归文件发现绝不授予执行权限。选择变更需要带修订检查的 Apply，并在 Run 和 Restart 前重验。缺失或变化的库在活动替换前失败。

<a id="verification"></a>
<a id="verification--已验证软件包准入"></a>
## 已验证软件包准入

[English](../../../../docs/isos/interfaces/package/README.md#verification) · [发行包](README_cn.md#release) · [资源](README_cn.md#resources)

安装验证一个精确 `{id,version}` 发行包与工件，然后原子发布收据和载荷。本地链接冻结被接受元数据与载荷指纹；链接字节变化后必须明确重新链接。使用 `pkg install --source <绝对源码路径> --allow-source-build` 安装的软件包将源码检出记录为 `local_source`。Apply 和 Run 要求检出干净，并验证已安装软件包。若 HEAD 与收据中的源码提交一致，就复用该已验证候选包；若 HEAD 变化，则重新构建，并仅在验证成功后替换旧候选包。构建失败将拒绝本次操作，保留旧软件包。已发布软件包仍按精确版本和字节摘要锁定。图未锁定的无关已安装软件包不能阻止该图。

准入采用单一顺序：精确锁定 → 发行契约及载荷收据 → 注册节点类型与位置 → 所声明资源 ID 和受控文件摘要 → 所有者与生命周期能力 → 绑定运行的交接。锁定收集器覆盖所有受支持的图位置，重复诊断按规范图指针排序。不存在软件包特定优先级或后备解析器。

| 失败 | 结果 | 状态影响 |
| --- | --- | --- |
| 缺少必需的通用贡献 | `unsupported_package_contract` | 不更改保存的图或活动运行。 |
| 发行包无效、不安全路径、符号链接、摘要不符或元数据过大 | 带图指针和阶段的有界软件包诊断 | 不发生部分安装或替换。 |
| 未知注册类型、位置、所有者或适配器 | 类型化准入诊断 | 不启动子进程，不发布图。 |
| 未知资源 ID 或资源字节变化 | 类型化资源诊断 | 不启动子进程，不发布图。 |
| 保存修订或运行栅栏过期 | 冲突诊断 | 不替换活动运行。 |

直接安装/链接错误使用空图指针；图上错误使用实际选中节点的指针。诊断包含稳定代码、阶段、精确锁定、净化消息及规范指针，不泄露本地路径、原始元数据或秘密。响应封套见[HTTP 软件包](../api/http_cn.md#http_packages)。

<a id="verification--读取器与功能预检"></a>
### 读取器与功能预检

IsOS 在 Apply、Run 与 Restart 前检查选中的可执行文件及其精确已验证软件包贡献。产品版本、其他检出、可执行路径本身或软件包类型后缀都不能证明支持。能力探测有界、不启动运行时，并必须描述实际启动字节。可执行文件、解释器、模块树、软件包收据或生成修订变化时阻止替换。解释器缺失不会触发 `uv sync` 或其他引导。外部设置响应使用模式 3；IsOS 自有 HTTP 契约独立使用 `"0.1.0"`。

<a id="verification--读取器身份字段"></a>
### 读取器身份字段

| 字段 | JSON 类型 / 单位 | 用途、缺省及验证 |
| --- | --- | --- |
| `supported_graph_versions` | 非空唯一字符串数组 / 图版本 | 选中二进制实际读取的精确版本。选中读取器声明支持图版本时必需。 |
| `supported_node_type_versions` | 非空唯一字符串数组 / 类型后缀 | 二进制读取的精确后缀；不足以准入完整类型 ID。 |
| `supported_node_types` | 唯一完整 ID 数组 / 类型 | 读取器实现的内建 ID。贡献类型还需要已验证精确软件包锁定和所有者目录。 |
| `features` | 唯一 ASCII 字符串数组 / 能力 | 该二进制实际能力。缺失表示空集；所需能力随后失败。 |
| `binary` | 字符串 / 选中可执行身份 | 运行时 `--capabilities` 角色，例如 `aether-runtime`；设置使用 `core`。 |
| `product_version` | 非空字符串 / 发行版本 | 仅用于展示与来源，不是兼容性判据。 |
| `setup_schema_version` | 整数 / 外部设置模式 | 当前选中设置身份必须为 `3`。 |

IsOS 在消费合并所有者目录前检查图结构、精确锁定和经过验证的贡献。随后依据选中的设置读取器、图读取运行时及传输子进程检查图/类型版本与所需功能。`package-process-v1` 贡献需要所有者能力；选中运行时还可能根据已准入图要求 `managed-control-grants`、`managed-config-v1` 或 `eos-gateway-client-v2`。缺少功能时在启动前阻止。所有者声明功能不证明就绪；健康状态另行检查。诊断按规范图路径、二进制角色及功能 ID 排序。

当选中软件包声明 `compatibility.required_owner_capabilities` 时，IsOS 将每项 EI 或 EOS 要求与精确选中的所有者能力回复比较。IsOS 所属的外部节点能力要求目前保持拒绝，直到 IsOS 所有者提供等价的能力接口。缺少能力会在启动前拒绝所选锁定。验证发行包时检查所声明的 IsOS 版本范围；该范围不能代替选中所有者的能力证明。

<a id="verification--诊断字段"></a>
### 诊断字段

| 字段 | JSON 类型 / 单位 | 用途与验证 |
| --- | --- | --- |
| `code` | 稳定字符串 / 失败 | 为 `reader_identity_invalid`、`reader_graph_unsupported`、`reader_type_unsupported`、`reader_feature_unsupported`、`reader_binary_mismatch` 或 `reader_package_pin_mismatch`。 |
| `path` | JSON 指针 / 图字段 | 精确选中类型/软件包或 `/version`；选中子进程错误可用 `/settings/role`。 |
| `message` | 有界字符串 / 修复文本 | 净化后的操作，不含原始 stderr、本地路径或秘密。 |
| `stage` | `graph`、`package`、`reader`、`feature` / 阶段 | 标识失败的准入步骤。 |
| `reader.core`、`.binary`、`.binary_sha256` | 字符串及可选 64 位十六进制 / 选中二进制 | 精确选中所有者、可执行身份及字节来源。 |
| `reader.requested` | 对象 / 请求声明 | 适用的图版本、完整类型、功能及精确软件包锁定；不适用值为 null。 |
| `reader.supported` | 对象 / 已声明能力 | 排序后的图版本、类型版本、完整 ID 及功能；不可信声明为空数组。 |

<a id="authentication"></a>
<a id="authentication--为软件包发布登录"></a>
## 为软件包发布登录

本页面向通过 IsOS CLI 使用 Island-port 的发布者。`isos auth login` 使用 GitHub OAuth Device Flow，并将临时 GitHub 用户令牌换成 Island-port 访问及刷新令牌。它只请求 `user:email`；GitHub 令牌不写入磁盘。IsOS 将 Island-port 凭据保存到配置目录中的 `auth.json`，权限仅限文件所有者。此登录与 IsOS 的[本地 HTTP 请求准入](../api/http_cn.md#http_envelope)相互独立。

运行 `isos auth login` 启动设备流程，`isos auth whoami` 查看登录账号，`isos auth logout` 撤销刷新会话并移除本地凭据。Island-port 服务必须启用 Device Flow。API 返回 401 后访问凭据会刷新，替换的刷新令牌会保存。发布资格由 Island-port 决定；IsOS 不获取 GitHub 仓库写入凭据。

[English](../../../../docs/isos/interfaces/package/README.md#authentication)
