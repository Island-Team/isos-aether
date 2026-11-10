# 选中核心设置适配器

[English](../../../../docs/isos/interfaces/core/setup.md) · [核心 HTTP](../http/cores_cn.md)

已实现的外部所有者设置交换使用**模式 2**。IsOS 选择配置的所有者可执行程序；客户端不能提交路径。输入上限 1 MiB，输出上限 4 MiB，截止时间 10 秒。`identity`、`catalog` 与 `options` 只读；`options` 根据归属方投影返回上下文选择项；`validate` 在 Apply 前重新检查草稿；`render` 预览有界生成文件。IsOS 在发布前拒绝错误核心、不兼容身份、格式错误或过大响应、越界路径及超时。此外部通信格式独立于 IsOS 自有 HTTP `"0.1.0"` 模式。

`isos serve` 运行时，会在 IsOS 运行目录中的仅所有者可访问 Unix 套接字上启动空闲的 EOS 和 EI 服务。IsOS 通过这些套接字上的 HTTP/1.1 调用 `POST /v1/identity`、`/v1/catalog`、`/v1/options`、`/v1/validate` 和 `/v1/render`。传输请求使用 `schema_version:1`；所有者将其转换为此处的模式 2 设置操作，并返回模式 2 响应。请求只携带该所有者的图投影、已验证的软件包存储凭据、端点和修订号。所有者套接字缺失时操作失败；IsOS 不会改选其他所有者。独立的离线设置检查仍直接调用选中的适配器。

服务健康路由为 `GET /v1/health`，返回 `schema_version:1`、固定核心身份和 `state:"idle"`。目前服务对 Apply、Run、Stop、状态和事件返回 HTTP 501；在完成所有者工作进程移交之前，这些操作继续使用现有的 IsOS 进程监管和运行栅栏。

所有者以类型化诊断拒绝设置请求时，IsOS 在本地管理器日志中以 `setup_owner_diagnostic` 记录最多 32 组诊断 `path` 和 `code`。图响应保留稳定的公开拒绝代码；本地日志给出所有者报告的准确节点失配位置，且不会把任意诊断消息复制到共享游标。
直接离线调用适配器时，只有包含有效模式 2 响应、`ok` 为 false 且诊断非空，退出状态 2 才会被接受。其他非零退出，以及状态 2 却报告成功的响应，都按失败关闭处理。

```json
{
  "schema_version": 2,
  "ok": true,
  "diagnostics": [],
  "files": [],
  "identity": {
    "core": "eos",
    "setup_schema_version": 2,
    "product_version": "0.2.1",
    "graph_version": "0.2.0",
    "node_type_version": "0.2"
  }
}
```

| 字段 | 类型 / 单位 | 用途与验证 | 缺省值 |
| --- | --- | --- | --- |
| `schema_version` | 整数 / 外部所有者模式 | 请求与响应均精确为 `2`。 | 必填。 |
| `op` | `identity`、`catalog`、`options`、`validate`、`render` / 操作 | 选择一个有界操作。 | 请求必填。 |
| `core` | 字符串 / 所有者角色 | 必须匹配选中配置的所有者。 | 请求必填。 |
| `graph`, `revision` | JSON 对象与可选 64 位十六进制 / 图及 SHA-256 | 验证/渲染草稿及可选保存字节栅栏。 | 依操作而定。 |
| `ok`, `diagnostics`, `files` | 布尔值与数组 / 结果 | 封闭响应；诊断条目有 `path`、`code`、`message`；文件条目有安全相对 `path`、`content`。 | 响应必填。 |
| `identity`, `catalog` | JSON 或 null / 所有者元数据 | 按操作提供；身份具有 `setup_schema_version:2`。 | 可选。 |

对于 `options`，`catalog.setting_options` 是有界的 `{node_path, setting, selected, options}` 分组数组。每个选项含 `value`、`label`、`description`、`available`、`reason_code` 和 `reason`。`node_path` 属于请求的归属方投影；`setting` 指向已声明的归属方选择器或归属方拥有的硬件字段。归属方报告不可用的已选值，不会静默替换。IsOS 在公开结果前检查分组身份、数量上限及草稿选中值。此只读预览不授权 Apply：`validate` 和 `render` 会按当前归属方输入重新检查选中的设置。
