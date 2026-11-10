# 目录 HTTP API

[English](../../../../docs/isos/interfaces/http/catalog.md) · [封套](envelope_cn.md) · [图](../node/README_cn.md)

目录路由公开注册节点类型、选中类型、节点选项及节点集预览。结果来自经过验证的通用 `node_contributions` 和选中所有者目录；软件包 ID 不改变解析或优先级。所有路由都需要回环 Host/Origin 准入。

| 路由 | 结果 | 状态影响 |
| --- | --- | --- |
| `GET /api/isos/v1/catalog/node-types` | 注册类型及可用性。 | 无。 |
| `GET /api/isos/v1/catalog/node-types/selected` | 已应用图引用的类型。 | 无。 |
| `POST /api/isos/v1/catalog/node-options` | 所提供草稿的声明设置、稳定资源 ID 及 EOS/EI 归属方上下文选择项。 | 无。 |
| `GET /api/isos/v1/catalog/node-sets` | 经过验证的声明节点集。 | 无。 |
| `POST /api/isos/v1/catalog/node-sets/preview` | 候选图组合；不发布。 | 无。 |

需要正文的请求使用 `schema_version:"0.1.0"`。格式错误的模式返回 400；未知精确锁定、类型、资源或所有者返回有界准入诊断。过期草稿修订返回 409。字段身份与验证细节见[图](../node/instance_cn.md)、[选项](../node/selectors_cn.md)和[资源](../packages/resources_cn.md)。

归属方选择项分组保留 `node_path`、`setting`、草稿的 `selected` 值，以及带可用性和原因字段的有界 `options`。IsOS 在返回前检查归属范围和分组身份。工作台把响应绑定到当前草稿；归属方请求失败时，选择器保持不可用。草稿类型目录与选项使用同一批本地选定的 EOS/EI setup 适配器，即使持久化归属方仍运行较旧的已准入世代。预览可能过期，因此 Apply 会向实时归属方重新验证；被拒绝的 Apply 无法启动该草稿。
