# 所有者控制投影

[English](../../../../docs/isos/interfaces/http/owners.md) · [HTTP 路由](README_cn.md) · [EOS 控制契约](../../../eos/interfaces/control_cn.md)

## 概述

IsOS 为 Web UI 提供选中 EOS 公共控制桥的有界投影。IsOS 检查活动运行及保存图修订，从已验证图解析桥的环回地址，并且仅转发公共操作。EOS 仍负责命令准入和执行器安全。该路由不能选择任意所有者端点或提供私有授权。

## 身份验证

调用方必须通过 IsOS 环回 Host 和获准 Origin 检查。请求不携带令牌或所有者秘密。IsOS 只接受运行中、运行 ID 已知、保存图字节匹配且桥进程运行中的部署。所有者桥另行执行自身准入。

## 接口

请求和响应使用 JSON。IsOS [封套](envelope_cn.md)的 schema 为 `0.1.0`；其中 `data` 是有界 EOS 公共响应，保留 EOS 的 `ok` 和操作字段。请求体最多 20,480 字节，所有者响应最多 65,536 字节。操作只能是 `discover`、`status`、`health`、`observe`、`time`、`result`、`primitive`、`primitive_typed` 或 `manual`。顶层 `grant`、`token` 和 `endpoint` 字段会被拒绝。操作专属字段与验证由 [EOS 控制契约](../../../eos/interfaces/control_cn.md)定义。

## 路由

### POST /api/isos/v1/owners/eos/control

IsOS Web UI 发送包含 EOS 公共操作的 JSON 对象。例如 `{"op":"time"}` 在 EOS 接受时返回 `{"schema_version":"0.1.0","ok":true,"data":{"ok":true,"monotonic_ns":123},"error":null}`。状态码 `400` 表示 JSON 格式错误或操作未列出；`403` 表示 Host 或 Origin 被拒绝；`409` 表示图修订过期；`413` 表示请求过大；`422` 表示 EOS 拒绝；`502` 表示所有者响应无效或过大；`503` 表示运行、管理器、端点或桥不可用。失败使用相同封套，例如 `{"schema_version":"0.1.0","ok":false,"data":null,"error":{"code":"owner_bridge_unavailable","message":"EOS bridge is unavailable","details":null,"correlation_id":"req-1"}}`。

### GET /api/isos/v1/owners/eos/vision/gates

Web UI 通过 IsOS 读取当前 EOS 门目录。成功的 IsOS 封套包含 EOS 的 `revision` 和 `gates` 字段。IsOS 要求桥处于活动状态，且 JSON 响应不超过 65,536 字节。失败时，本地准入返回 `403`，图字节过期返回 `409`，目录无效或过大返回 `502`，运行或 EOS 不可用返回 `503`。

### GET /api/isos/v1/owners/eos/vision/ws

Web UI 通过本地 Host/Origin 准入后与 IsOS 建立 WebSocket。IsOS 检查同一活动桥和图围栏，然后转发一条最多 4,096 字节的文本订阅、EOS 文本证明及最多 8 MiB 加帧头与元数据的二进制帧，不改写来源证明。EOS 验证订阅并发送证明；浏览器仍检查运行、图、来源、投影和摘要。任一侧关闭都会关闭转发。初始 HTTP 拒绝返回 `403`、`409` 或 `503`；上游断开时关闭套接字。
