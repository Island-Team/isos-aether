# 本地 HTTP 路由

本目录面向 IsOS 服务及本地客户端实现者。路由以 `/api/isos/v1/` 开始；请求经过本地 Host/Origin 准入后由 `app` 解码，图、软件包和运行时归属模块作出决策。[外部节点事件请求](../ui/node-events_cn.md)随节点契约记录。

- [封套](envelope_cn.md)定义回环请求准入及成功与错误响应字段，包括诊断代码、阶段和图路径。
- [版本](version_cn.md)返回 API 身份，使客户端选择受支持接口。
- [图](graph_cn.md)说明草稿验证、保存修订和 Apply；保存实例模式见[节点](../node/instance_cn.md)。
- [图库](graph-pool_cn.md)保存与活动部署独立的命名草稿。
- [目录](catalog_cn.md)公开注册类型与选择器选项，不更改保存的图。
- [软件包](packages_cn.md)说明围绕精确已验证锁定的安装、链接、检查和移除。
- [核心](cores_cn.md)向本地客户端公开归属方发现与就绪。
- [运行时](runtime_cn.md)说明带运行／修订围栏的 Run、Restart、Stop、状态及日志。
- [所有者控制投影](owners_cn.md)通过 IsOS 转发有界 EOS 公共操作和视觉流，EOS 仍负责准入。
- [呈现](presentations_cn.md)公开选中的输出元数据；[UI 选择](../ui/selection_cn.md)拥有通用显示规则。

[English](../../../../docs/isos/interfaces/http/README.md)
