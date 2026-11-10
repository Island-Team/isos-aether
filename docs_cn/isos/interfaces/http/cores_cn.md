# Core API v1

[English](../../../../docs/isos/interfaces/http/cores.md) · [owner 边界](../manager/README_cn.md)


## GET /api/isos/v1/cores/readiness

返回 `data.cores`、图与 IsOS/UI 配置就绪。它使用选定的软件包和 EI 所有者 stage 目录验证已保存图，包括每个已注册 `isos.<layer>.<name>@v0.2` 节点的位置。它描述选定安装路径与兼容性；实时就绪请查看部署状态和所有者事件。

已启用核心尚无选中生成配置时，返回 `graph_configured: false` 并提示 Apply 保存图。先前已 Apply 的图库文件以新字节保存后，`configuration_diagnostic.code` 为 `pending_apply`，Run 在再次 Apply 前继续受限。若选中的程序在 Apply 后发生变化，`configuration_diagnostic.code` 为 `generated_launch_input_mismatch`，`ready` 为 false，操作提示再次 Apply。无效的选中快照会报告其诊断，不会被误报为未保存的图。已停止但就绪的核心提示运行保存图。存在归属方层却没有对应进程节点时，会明确报告缺少进程。真正空白的图使 EOS 和 EI 均处于未启用状态，不产生错误。

对于仅报告健康的三核心图，EOS owner 的 `ready` 状态与 EI owner 的 `running` 状态都会使 `running: true`。此字段反映部署状态报告的 owner 工作进程；`ready` 字段另行描述已保存配置能否运行。若 owner 失败，部署状态变为 degraded，工作台会在浏览器日志记录受影响 owner 的状态。

## GET /api/isos/v1/cores/setup-status

返回 `data.ready`、`issues`、`graph_revision`、backend 选项及进程摘要。响应示例：`{"schema_version":"0.1.0","ok":true,"data":{"ready":false,"issues":["selected owner setup is unavailable"],"graph_revision":null,"backends":[],"process":null},"error":null}`。

## GET /api/isos/v1/cores/setup/{role}/catalog

返回已保存图对应的选定 EOS 或 EI 归属方 setup 目录，不发布配置。路由只接受 `eos` 或 `ei`；归属方不可用时返回 503。

## POST /api/isos/v1/cores/setup/{role}/validate

请求 `{"schema_version":"0.1.0","graph":{"version":"0.2.0","robot":{"id":"robot","name":"Robot"},"cores":{"isos":{"layers":{}}}},"revision":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}`。IsOS 将草稿及修订传给所选 owner，不发布配置。owner 响应含数字 `schema_version:2`、`ok`、`diagnostics`、`files` 和可选 `identity` 或 `catalog`。

## POST /api/isos/v1/cores/setup/{role}/render

使用与 validate 相同的请求形状，只预览有界相对生成文件，不发布。缺图返回 400，未知 role 或操作返回 404，超时、畸形响应或 setup 身份未声明 `setup_schema_version:2` 返回 503。

`schema_version` 为字符串/—且必须为 `0.1.0`；`role` 是封闭字符串/—，防止选择任意可执行文件；`graph` 为已注册 profile 对象/—；`revision` 是可选 64 位小写 SHA-256 字符串，供已有保存字节围栏的调用者使用。外部 setup wire 使用模式 2 和有界 stdin/stdout；产品版本不代表 setup 兼容。

## 失败示例与字段契约

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


## 验证

图中 `isos.ei_cognition.builder@v0.2` 位于 `ei_cognition/context` 时，就绪检查必须使用与准入相同的选定 EI stage 目录。含节点路径的 EOS 或 EI 事件必须指向已保存 `isos` 图中所属所有者层内的节点，并通过监督器的已准入节点集合。重放时应将此端点与部署状态、选定所有者事件和 Stop 结果对照。

聚焦检查使用 `ISOS_AETHER_BIN_DIR=<workspace>/target/debug RUST_TEST_THREADS=1 cargo test -p isos`、`RUST_TEST_THREADS=1 cargo test -p aether-runtime --test grpc_skills`、`cargo fmt --all -- --check` 和 `cargo clippy -p isos -p aether-runtime --all-targets -- -D warnings`。把完整输出保存在 `data/test/<run-id>/`。

针对缺少生成配置的情况，手工以同一修订 Apply 后重新生成了 EOS 启动输入；就绪状态从 `graph_configured: false` 变为 `true`，操作提示为 `Run the saved graph`。UI 类型检查、208 个 Vitest 测试、本地与托管 Web 构建、TUI 构建、Rust 格式与 Clippy，以及中英文链接审计均通过。
