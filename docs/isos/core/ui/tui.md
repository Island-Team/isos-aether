# IsOS TUI

<!-- doc-navigation:start -->
<details>
<summary>Folder: isos/core/ui</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [IsOS WebUI](webui.md)

</details>
<!-- doc-navigation:end -->

[中文](../../../../docs_cn/isos/core/ui/tui_cn.md)

## TUI first use

Open the [WebUI quick start](webui.md) or follow the TUI steps below.

Start the IsOS server and wait for setup readiness before choosing a client. In WebUI, inspect the selected graph and package catalog, validate, save, Apply, Run, watch owner state and logs, then Stop. Both clients report the manager outcome.

## TUI quick start

Run the built terminal client with `--origin http://127.0.0.1:16000 --locale en`. Use `1` for deployment status, `2` and `3` for EOS and EI readiness, and `4` for current logs. Press `r` twice to Run the saved revision or `s` twice to Stop. Check the visible outcome and current logs.

## Client behavior

The Ink client in `ui/src/tui/index.tsx` reads process status, core readiness, setup status, and current logs through the shared IsOS service transport. It receives live status and log pages over WebSocket; readiness and setup are read on startup and explicit refresh. It displays Home, EOS, EI, and Logs views, and keeps Run and Stop behind a second keypress. An uncertain Stop outcome stays visible until the operator inspects status and logs. See the [TUI interface](../../interfaces/api/README.md#tui).
