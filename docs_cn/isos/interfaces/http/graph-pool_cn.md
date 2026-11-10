# 图库 API v1

[English](../../../../docs/isos/interfaces/http/graph-pool.md) · [活动图 API](graph_cn.md)

图库是本地 IsOS 配置目录中的图文件工作区。部署绑定只记录选定的图库名称；图字节仅存在于图库。所有路由使用与图 API 相同的回环 Host 和 Origin 检查。保存已绑定文件会改变其保存修订，因此 Run 前必须通过 `PUT /api/isos/v1/graph` 刷新生成配置。保存不会启动或停止进程。

## GET /api/isos/v1/graph-pool

返回按名称排序的 `data.graphs` 数组，条目包含 `name`、准确字节 SHA-256 `revision` 和 `version`。图库最多接受 128 份草稿。不安全或超大条目会被拒绝。

## GET /api/isos/v1/graph-pool/{name}

返回单份草稿的 `data:{name,revision,graph}`。名称包含 1–48 个 ASCII 字母、数字、连字符或下划线。名称不存在时返回 404。

## PUT /api/isos/v1/graph-pool/{name}

保存 `0.2.0` 图文件。先从 `GET /api/isos/v1/graph/new` 返回的包含独立按钮链与模拟机械臂监测链的十六节点演示图开始，再发送 `{"schema_version":"0.1.0","revision":"","graph":<草稿>}` 创建新名称。替换已有文件时提交 GET 返回的准确修订；编辑器会提示同名保存将替换该文件。修订变化返回 409。编辑期间文件可以不完整；Apply 要求三个核心，并验证完整图。图的大小上限为 1 MiB。

Web 编辑器从图库打开一个命名文件，并合并经修订检查的归属方选定目录与全局目录，使仅在归属方目录中的节点类型仍显示设置和端口。画布位置和上次打开的图库名称保存在浏览器本地。该文件缺失时，编辑器在保存时要求输入新名称。
