# EI setup 交换

[English](../../../docs/ei/interfaces/setup.md) · [读取器身份](reader-features.md) · [阶段管线](../design/pipeline.md)

IsOS 使用一份有界 JSON 请求调用选定的 `ei-config` 进程，操作为 `identity`、`catalog`、`options`、`validate` 或 `render`。托管 setup envelope 使用数字 `schema_version: 2` 和 `core: "ei"`。EI 接受图格式 `0.2.0` 的独立 `cores.ei` 投影；EI 核心验证三个激活节点和三个机械臂监测节点的位置、空设置及其声明的激活输入。旧健康状态节点和仅声明型节点将被拒绝。IsOS 管理保存的修订、软件包验证及返回文件的发布。

`values` 包含 IsOS 可编辑的 EI listener 与 gateway 连接。`mode` 必须为 `isos`；`server` 和 `gateway` 使用各自文档定义的严格字段。setup 错误包含稳定 JSON Pointer `path`、`code` 和有界 `message`，且不生成文件。setup 操作不启动 EI 运行时，也不连接 EOS。

```json
{
  "schema_version": 2,
  "op": "identity",
  "core": "ei"
}
```

身份报告源码产品版本 `0.2.1`、图格式 `0.2.0`、节点类型版本 `0.2`、精确支持集合、setup 模式版本 `2` 及选定特性。EI 所需的 EOS Gateway、Capability 与 Skill 服务版本是独立的线协议身份。

对于被拒绝的 EI 节点，单次 setup 命令和所有者套接字返回相同的类型化诊断。`path` 指向被拒绝的图字段，例如 `/graph/cores/ei/layers/cognition/groups/intent/nodes/bad/node_type`；`code` 区分 `unsupported_node_type_version` 与 `invalid_node_placement`。IsOS 可以在 setup 诊断中记录此有界响应，而无需依赖未准入的所有者 stderr。

## 持久化所有者控制套接字

`options` 返回空的 `setting_options` 列表，因为目前节点设置是静态的。`validate` 和 `render` 验证 EI 激活节点声明与解析后的端点输入。

`ei-config owner --socket <绝对路径>` 在 Unix 套接字上运行持久的 EI 所有者服务。套接字文件权限为 `0600`，父目录必须预先存在。`GET /v1/health` 返回 `schema_version:1`、`core:"ei"`、配置版本 `2`，以及根据工作进程状态得出的 `idle`、`running` 或 `exited`。`POST /v1/identity`、`/v1/catalog`、`/v1/options`、`/v1/validate` 和 `/v1/render` 使用 JSON 信封：`schema_version:1`、`core:"ei"`，且 `op` 与路径一致。其他字段沿用上述设置交换，响应仍使用设置 schema 版本 2。请求、头部和响应的上限分别为 1 MiB、8 KiB 和 4 MiB；每个连接的超时为十秒。

生产环境的 `POST /v1/run` 接受传输版本 1 的信封，包含 `core:"ei"`、`op:"run"`、有界 `run_id`、小写 64 位十六进制 `revision`、所有者投影后的 `graph`、`endpoints`、已验证的 `packages`，以及 `launch:{sha256,args,env}`。EI 重新验证图并核对 `ei` 工作程序的 `sha256`，随后每个所有者在独立进程组中最多启动一个工作进程。仅报告健康状态的工作进程不占用完整运行时的安全状态锁；所有者仍约束选定运行和 Stop。`POST /v1/status` 和 `/v1/stop` 要求精确匹配运行身份。Stop 向受管工作进程发信号并回收，然后返回 `outcome:"stopped"` 与 `worker_exited:true`。IsOS 客户端退出不会终止所有者服务；Stop 后可接受新运行，同时拒绝旧运行身份。`/v1/apply` 返回 501。EI 将诊断写入本地 JSONL 文件。

EI 所有者从信封注入 `ISOS_RUN_ID`，并拒绝 `launch.env` 中冲突的值。已完成的运行身份在服务整个生命周期内不可重复使用。工作进程意外退出时，Stop 回收它后才可启动另一运行。

`POST /v1/activate` 接受版本 1 的激活封套，包括运行 ID、修订、来源节点、事件 ID 和 `kind:"activate"`。所有者检查运行、工作进程和 EI 的有序图路径，再转发至 Run 提供的 `endpoints.eos_owner.path`。成功响应包含 EOS 命令 ID、`status:"accepted"`，以及 EI 运行、修订和事件 ID；物理执行异步完成。`POST /v1/result` 使用相同的版本和运行约束，携带 `op:"result"` 及事件 ID，转发 EOS 按该事件关联的 `accepted`、`running`、`succeeded`、`failed`、`cancelled` 或 `outcome_unknown` 结果；仅成功结果携带六个模拟关节位置。旧运行返回 409；无效路径返回 422；工作进程不可用或 EOS 结果不确定返回 503。Stop 在结束工作进程前关闭接纳。

`POST /v1/monitor` 接受数字版本 1、`core:"ei"`、`op:"monitor"`、运行 ID 和修订。成功响应在 EI 运行与修订封套下携带未经修改的 EOS `observation`。包含监测节点的图在 Run 时必须提供 `endpoints.arm_monitor.joint_names`，即已应用 EOS 机械臂节点的六个有序名称。监测节点的 `forward_rate_hz` 默认为 20，允许 1–20 Hz；运行所属的后台任务按不高于该速率轮询 EOS，与 IsOS 读取者无关。读取返回最新的已验证快照，刷新后三个间隔与一秒两者取较长值，在该时限内有效。旧运行返回 409；无效图路径返回 422；工作进程或 EOS 所有者丢失、关节名称不符、采样无效、序号倒退或缓存过期返回 503。Stop 清除任务和缓存，然后才允许新运行。
