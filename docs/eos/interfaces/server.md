<a id="external_http"></a>
<a id="external_http--eos-loopback-http-bridge"></a>
# EOS loopback HTTP bridge

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos/interfaces</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Subfolders:
  - [node/](node/README.md)

</details>
<!-- doc-navigation:end -->

[中文](../../../docs_cn/eos/interfaces/server_cn.md#external_http_cn) · [Control admission](../../isos/interfaces/server/eos.md#control) · [Vision](node/interface/egress.md#interface_egress--vision)

`aether-server` is the Rust client-facing process for control and setup catalog. It owns two loopback listeners: `--listen` for runtime control and `--catalog-listen` for read-only setup routes, with `--profile` selecting the graph snapshot. The catalog listener is validated and bound before runtime readiness is reported. Setup-only invocation supplies `--profile` and `--catalog-listen` without a runtime socket. The saved graph still declares `bridge` and `config` process nodes; IsOS combines their settings into one `bridge` child at launch. It binds loopback, validates Host and Origin, bounds request bodies and private IPC, and forwards only admitted public operations to the selected `aether-runtime` Unix socket. It does not release an actuator directly. IsOS selects and supervises the bridge as part of the applied deployment.

<a id="external_http--control-route"></a>
## Control route

`POST /api/eos/v2/control` accepts bounded JSON for admitted `primitive_typed`, authorized result lookup, and read-only status, observation, and health. The [control contract](../../isos/interfaces/server/eos.md#control) defines request fields, grant handling, deadlines, errors, and Dispatcher reservation, with readable JSON request, success, and error examples. Browser callers cannot supply a private owner grant or invoke raw dispatch and shutdown operations.

<a id="external_http--failure-and-stop"></a>
## Failure and Stop

Invalid Host or Origin, malformed JSON, excessive payload, missing grant, stale revision, unavailable runtime, and failed owner attestation produce bounded errors without exposing private paths or tokens. IsOS owns deployment Stop. EOS closes command admission and stops admitted actuators before terminating its direct selected service child.
