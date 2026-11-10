# IsOS 设计

[架构](architecture_cn.md)是当前总览。专题页面与 Rust 源码边界对应：

- [服务端与启动器](app/server_cn.md)说明 CLI 准入、回环服务就绪和 manager 交接。
- [软件包准入](domain/admission_cn.md)说明精确版本验证与归属方启动。
- [软件包管理器](domain/packages_cn.md)记录已实现流程与明确标注的目标工作。
- [Manager 服务](runtime/services_cn.md)说明本地归属方配置。
- [展示选择](ui/presentation_cn.md)说明输出归属和当前不可用状态。
- [注册节点族](node/README_cn.md)对应进程和 UI 消费方。
- [控制路径](control-path_cn.md)跟踪 UI 按键到 EOS 准入。
- [验证](verification_cn.md)记录命令与证据；[剩余工作](remaining-work_cn.md)列出待完成关口。

公开字段由[接口](../interfaces/README_cn.md)定义。

[English](../../../docs/isos/design/README.md)
