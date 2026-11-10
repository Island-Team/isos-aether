# EOS 回环 HTTP bridge

[English](../../../../docs/eos/interfaces/external/http.md) · [控制准入](../control_cn.md) · [视觉](../vision_cn.md)

`aether-http-bridge` 是面向客户端的 Rust 进程。它绑定回环地址，校验 Host 与 Origin，限制请求体和私有 IPC，并只将已准入的公开操作转发到选定的 `aether-runtime` Unix 套接字。它不直接释放执行器。IsOS 在已应用部署中选择并监督 bridge。

## 控制路由

`POST /api/eos/v2/control` 接收有界 JSON，用于已准入的 `primitive_typed`、授权结果查询及只读状态、观察与健康。[控制契约](../control_cn.md)定义请求字段、授权、期限、错误与 Dispatcher 预留。浏览器调用方不能提供私有所有者授权，也不能调用原始调度或关闭操作。

## 视觉路由

只读 Vision WebSocket 从选定的活动相机 gate 传送 JPEG 帧。bridge 在每次有界轮询中验证所有者证明、运行与图修订。它不创建相机路线，也不改变已应用图。参见[视觉契约](../vision_cn.md)。

## 故障与 Stop

无效 Host 或 Origin、畸形 JSON、过大载荷、缺少授权、过期修订、运行时不可用及所有者证明失败，都产生有界错误且不泄露私有路径或令牌。IsOS 负责部署 Stop。EOS 在终止其直接仿真器子进程前关闭命令准入并停止已准入执行器。
