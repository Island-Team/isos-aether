# 节点、软件包、核心与 IsosUI 接口

<!-- doc-navigation:start -->
<details>
<summary>目录：isos/interfaces</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [共享契约目录](contract-catalog_cn.md)
  - [核心归属方契约](core_cn.md)
  - [归属方实现说明](manager_cn.md)
- 下级目录：
  - [api/](api/README_cn.md)
  - [node/](node/README_cn.md)
  - [package/](package/README_cn.md)
  - [server/](server/README_cn.md)

</details>
<!-- doc-navigation:end -->

本目录为连接节点与 IsOS 的作者提供外部契约。**注册类型**声明位置、设置、端口和生命周期；**实例**选择该类型并提供值。**核心**拥有执行权。**精确锁定**标识一个已验证的软件包发行。**运行围栏**组合运行身份与保存图修订，使延迟事件不能影响替代运行。IsOS 当前按这些契约准入图声明和软件包；运行时传输仍由选中归属方负责。

- [IsOS 节点](node/README_cn.md)定义 IsOS UI 节点分组。[共享节点接口](../../node/interfaces/README_cn.md)定义所有所有者共用的类型、实例、端口、数据类型、选择器和连线规则。
- [软件包](package/README_cn.md)定义精确发行、声明资源、进程和验证。发布者可据此贡献节点类型，而不需要 IsOS 中的软件包专属逻辑。
- [核心](core_cn.md)定义 IsOS 的设置、进程交接、事件、授权和终止停止交换。
- [共享契约目录](contract-catalog_cn.md)按用途、维护方、使用方和领域分类源码契约，独立于图节点路径。
- [EI 服务端接口](server/ei_cn.md)定义 IsOS 与 EI 的设置、图路由、事件、读取器身份及安全重置交换。EI 自身的客户端和节点契约见 [EI 接口](../../ei/interfaces_cn.md)。
- [EOS 服务端接口](server/eos_cn.md)定义 IsOS 与 EOS 的设置、生命周期、控制、软件包及事件交换。EOS 自身的硬件、调度器和模型契约见 [EOS 接口](../../eos/interfaces_cn.md)。
- [API](api/README_cn.md)定义本地 HTTP 路由以及 Web/TUI 客户端契约。

私有管理器 IPC、本地 HTTP 路由细节和源码工作流程见[开发](../core/server_cn.md)。

[English](../../../docs/isos/interfaces/README.md)
