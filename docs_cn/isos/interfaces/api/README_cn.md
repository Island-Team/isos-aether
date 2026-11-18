# 客户端 API 与前端接口

<!-- doc-navigation:start -->
<details>
<summary>目录：isos/interfaces/api</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [Client interactions](client_cn.md)
  - [本地 HTTP 路由](http_cn.md)

</details>
<!-- doc-navigation:end -->

[English](../../../../docs/isos/interfaces/api/README.md) · [WebUI](../../core/ui/webui_cn.md) · [TUI](../../core/ui/tui_cn.md)

这些契约定义 Web 和终端客户端可经 IsOS 发送及显示的内容。IsOS 拥有已保存图修订、软件包、运行身份和 Stop；EOS 拥有执行器准入和硬件健康。客户端保留本地呈现状态，并在控制前重新检查所有者证据。

| 契约 | 范围 |
| --- | --- |
| [本地 HTTP API](http_cn.md) | 回环路由、请求准入、响应信封和诊断。 |
| [客户端交互](client_cn.md) | 节点事件、监控、视觉、选择、控制、图加载、日志和工作区面板。 |

## TUI

终端客户端接受回环地址的 `--origin` 和 `--locale en|cn`。按键 `1`–`4` 选择首页、EOS、EI 和日志；`f` 刷新；`r` 和 `s` 需要再次按下才能运行或停止；`x` 取消确认；`q` 退出。它通过共享服务传输使用 [IsOS 运行时](http_cn.md#http_runtime)和[核心](http_cn.md#http_cores)路由。
