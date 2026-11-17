# IsOS 指南

<!-- doc-navigation:start -->
<details>
<summary>目录：isos</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 下级目录：
  - [core/](core/README_cn.md)
  - [interfaces/](interfaces/README_cn.md)
  - [start/](start/)

</details>
<!-- doc-navigation:end -->

IsOS 负责图 `0.2.1`、精确软件包验证、Apply 和运行身份；源码版本为 `0.2.3`。它从已安装或链接的软件包发行清单取得贡献节点声明与资源，再请求所选 EI 和 EOS 核心验证各自的图投影。软件包名称和硬件实现不选择特殊的 IsOS 路径。参见[节点注册表](../node/interfaces/registry_cn.md#registry)和[软件包准入](interfaces/package/README_cn.md#verification)契约。


操作人员按以下路径阅读：

- [首次使用](start/quick-start_cn.md)建立本地主目录，说明检查状态、选择已验证节点、验证、应用和运行的顺序。
- [配置](core/config_cn.md)说明哪个文件提供默认值、如何选择生产覆盖值，以及如何修复启动错误。
- [软件包选择](start/quick-start_cn.md)解释如何安装或链接精确发行、检查贡献、选择稳定资源 ID，而不依赖软件包特有路径。
- [发布者登录](interfaces/package/README_cn.md#authentication)解释用于发布软件包的独立 Island-port CLI 登录；本地图控制不使用此凭据。
- [运行部署](start/quick-start_cn.md)说明依据保存修订和归属方结果进行 Validate、Apply、Run、Restart、Stop 及状态检查。
- [故障排查](start/quick-start_cn.md)使用结构化诊断定位软件包拒绝、启动、就绪、日志和停止不确定等失败。

作者与贡献者按以下路径阅读：

- [外部接口](interfaces/README_cn.md)包含发布者和图作者需要的通用节点、软件包、核心及 IsosUI 契约。
- [已注册 UI 节点族](core/server_cn.md)说明视觉监控节点和 UI 所属消费者的边界。
- [设计](core/README_cn.md)解释模块归属、软件包准入、数据选择和当前按钮到模拟机械臂的控制与监视路径，不重复定义通信字段。
- [开发](core/server_cn.md)为实现者定位本地 HTTP 路由、管理器 IPC、源码边界和验证命令。
- [维护](core/README_cn.md)索引当前证据和剩余集成门槛。

当前 IsOS API 使用 `/api/isos/v1/*`；需要模式的 JSON 负载使用 `"schema_version":"0.1.0"`。软件包及核心作者从[外部接口](interfaces/README_cn.md)开始。

[English](../../docs/isos/README.md)
