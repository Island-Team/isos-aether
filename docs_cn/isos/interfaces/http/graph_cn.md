# Graph API v1

[English](../../../../docs/isos/interfaces/http/graph.md) · [节点接口](../node/README_cn.md)

IsOS 管理已保存图及其精确字节 SHA-256 修订。所有 `/api/isos/v1/*` 路由都执行回环 Host 与 Origin 准入。JSON 修改使用 `schema_version: "0.1.0"`。成功响应为 `{ "schema_version": "0.1.0", "ok": true, "data": ..., "error": null }`；失败响应设置 `ok: false`，返回稳定错误代码、安全消息及关联 ID。过期修订返回 409，图准入失败返回 422。

`GET /api/isos/v1/graph` 从绑定的图库文件读取 `{pool_name, revision, graph, read_only, diagnostics, missing_file}`，不修改其字节。文件缺失时返回可编辑的三核心起始草稿、空修订及 `missing_file:true`；编辑器必须先以图库名称保存，才能 Apply。`GET /api/isos/v1/graph/new` 返回包含独立按钮链与监控链的 16 节点演示图，但不保存。

`POST /api/isos/v1/graph/validate` 接受 `{schema_version, graph}`，返回 `{valid, diagnostics}`。`PUT /api/isos/v1/graph` 接受 `{schema_version, pool_name, revision}`。修订必须等于命名图库文件的精确字节摘要。IsOS 验证三个核心、注册的所有者类型、静态连接、软件包锁定及所有者投影，发布生成配置，并把部署绑定到该图库文件。文件缺失返回 404；修订过期返回 409。旧调用方仍可提交 `{schema_version, revision, graph}`；此方式会更新当前绑定的图库文件，编辑器不应使用。

`POST /api/isos/v1/graph/connection` 预览一条草稿连接。草稿包含该输入引用后，请求携带 `{schema_version, graph, source, output, target, input}`。`source` 与 `target` 是规范的四段节点路径；`output` 与 `input` 是端口名。响应为包含准确 `source`、`output`、`target`、`input`、路由类别、通道及 QoS 字段的 `{allowed:true, edge}`，或 `{allowed:false, diagnostic}`。预览期间其他必需输入和固定绑定目标可以尚未连接。选定目录不可用时返回 503，API 版本错误返回 400。该读取不保存草稿，Apply 会重新执行完整准入。

当前图格式为 `0.2.0`，包含 `isos`、`ei`、`eos` 核心，类型 ID 为 `<core>.<layer>.<group>.<node>@v0.2`。[演示图](../../../../config/graph-pool/demo.nodes.json) 声明分属控制与观测路径的 16 个节点，不生成 IsOS 子进程规格。工作进程实时健康通过部署状态读取；图读取仅代表配置。
