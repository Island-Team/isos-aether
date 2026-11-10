# EOS 契约源码

> 历史契约：当前按钮到模拟机械臂图未实现此接口。当前数据及命令路径参见[机械臂控制](arm-control_cn.md)；设置、Run、Status 和 Stop 参见[所有者生命周期](owner-health_cn.md)。


[English](../../../docs/eos/interfaces/contracts.md) · [接口目录](README_cn.md) · [gRPC 与 IPC](grpc_cn.md)

EOS protobuf 与 JSON Schema 的权威源码位于 `share/eos/contracts/eos/`。`gateway.proto`、`skills.proto` 与 `capabilities.proto` 定义当前 Rust 运行时使用的面向进程的服务消息。`robot/`、`perception/`、`skills/`、`actions/`、`body/` 和 `world/` 等领域目录定义载荷与校验注解。[Rust 技能可执行文件契约](skills/rust_cn.md)定义独立的工作进程协议。

生成器在 `share/eos/schemas/eos/` 下生成 JSON Schema 与清单。protobuf envelope 提供传输身份；选定的生成 schema 校验有界参数与结果。图节点类型 ID、进程线协议模式、源码包版本和外部软件包版本是独立身份。

## 更新契约

修改所属 `.proto` 或手写 schema，重新生成派生产物，并同步更新接口页及英文镜像。运行：

```sh
python3 scripts/generate_eos_contracts.py --output share/eos/schemas/eos
python3 scripts/generate_eos_contracts.py --output share/eos/schemas/eos --check
cargo test --workspace
python3 scripts/check_doc_links.py --all
```

检查生成差异；公开契约变更应加入请求、成功、拒绝和故障夹具。传输边界以下的硬件样本与命令保持类型化 Rust 值。
