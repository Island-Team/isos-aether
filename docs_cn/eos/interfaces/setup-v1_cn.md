# EOS 设置交换

[English](../../../docs/eos/interfaces/setup-v1.md) · [所有者生命周期](owner-health_cn.md) · [EOS 接口](README_cn.md)

IsOS 通过仅供所有者使用的 Unix 套接字调用持久的 EOS 所有者。传输信封使用 `schema_version: 1`、`core: "eos"` 和与 `POST /v1/<op>` 匹配的 `op`。设置响应使用模式版本 `2`；传输、设置、图和节点类型版本相互独立。一次性的 `aether-profile setup --json` 以模式版本 `2` 接受相同的设置内容。

## 操作

`identity` 返回产品及支持的图和节点类型版本。`catalog` 返回四个实时演示节点及两个类型化载荷描述。`options` 为内置模拟机械臂返回图范围内的 `adapter.type` 选项。`validate` 检查仅含 EOS 的图投影并返回诊断，不生成文件。`render` 还返回已验证的 `eos/graph.json` 文件，供 IsOS 再次检查保存的修订后原子发布。设置不会启动工作进程。

```json
{
  "schema_version": 1,
  "core": "eos",
  "op": "validate",
  "revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "graph": {
    "version": "0.2.0",
    "robot": {"id": "robot", "name": "Robot"},
    "cores": {"eos": {"layers": {"interface": {"groups": {"ingress": {"nodes": {}}}}}}}
  },
  "endpoints": {},
  "packages": []
}
```

示例说明信封结构；有效图要求每个出现的分组至少有一个注册的 EOS 节点。投影只包含 `cores.eos`；其声明的跨 core 输入可引用经 IsOS 准入的 EI 来源。无效图版本、缺少 EOS 层、未知类型、错误设置和不兼容的本地连接返回 `ok:false`、有界诊断且不生成文件。请求和响应大小分别限制为 1 MiB 和 4 MiB。[所有者生命周期契约](owner-health_cn.md)定义 Run、Status、Stop 和健康状态。
