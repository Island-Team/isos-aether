# Client API and frontend interfaces

<!-- doc-navigation:start -->
<details>
<summary>Folder: isos/interfaces/api</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [Client interactions](client.md)
  - [Local HTTP routes](http.md)

</details>
<!-- doc-navigation:end -->

[中文](../../../../docs_cn/isos/interfaces/api/README_cn.md) · [WebUI](../../core/ui/webui.md) · [TUI](../../core/ui/tui.md)

These contracts define what the Web and terminal clients may send and display through IsOS. IsOS owns saved graph revisions, packages, run identity, and Stop. EOS owns actuator admission and hardware health. Clients keep local presentation state and recheck owner evidence before control.

| Contract | Scope |
| --- | --- |
| [Local HTTP API](http.md) | Loopback routes, request admission, envelopes, and diagnostics. |
| [Client interactions](client.md) | Node events, monitor, Vision, selection, control, graph loading, logs, and workspace panels. |

## TUI

The terminal client accepts a loopback `--origin` and `--locale en|cn`. Keys `1`–`4` select Home, EOS, EI, and Logs; `f` refreshes; `r` and `s` require a second press to Run or Stop; `x` cancels confirmation; `q` quits. It uses the [IsOS runtime](http.md#http_runtime) and [core](http.md#http_cores) routes through the shared service transport.
