# IsOS TUI

<!-- doc-navigation:start -->
<details>
<summary>目录：isos/core/ui</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [IsOS WebUI](webui_cn.md)

</details>
<!-- doc-navigation:end -->

[English](../../../../docs/isos/core/ui/tui.md)

## TUI 首次使用

可阅读 [WebUI 快速入门](webui_cn.md)，或按照下面的 TUI 步骤操作。

先启动 IsOS 服务器并等待配置就绪，再选择客户端。在 WebUI 中检查选定节点图与软件包目录，依次验证、保存、Apply、Run、查看所有者状态和日志，最后 Stop。两个客户端都显示管理器结果。

## TUI 快速入门

以 `--origin http://127.0.0.1:16000 --locale cn` 启动已构建的终端客户端。按 `1` 查看部署状态，按 `2` 和 `3` 查看 EOS、EI 就绪状态，按 `4` 查看当前日志。按两次 `r` 运行已保存修订，或按两次 `s` 停止。随后检查可见结果和当前日志。

## 客户端行为

`ui/src/tui/index.tsx` 中的 Ink 客户端通过共享的 IsOS 服务传输读取进程状态、核心就绪状态、配置状态和当前日志。实时状态和日志页通过 WebSocket 接收；核心就绪和配置状态在启动及手动刷新时读取。界面显示首页、EOS、EI 和日志视图；运行与停止需要再次按键确认。停止结果不确定时，界面会提示操作员检查状态和日志。参见 [TUI 接口](../../interfaces/api/README_cn.md#tui)。
