# Cores API v1

[中文](../../../../docs_cn/isos/interfaces/http/cores_cn.md) · [Owner boundaries](../manager/README.md)


## GET /api/isos/v1/cores/readiness

Returns `data.cores`, graph and IsOS/UI configuration readiness. It validates the saved graph against the selected package and EI owner stage catalog, including each registered `isos.<layer>.<name>@v0.2` placement. It describes selected installed paths and compatibility; use deployment status and owner events for live readiness.

An enabled core with no selected generation reports `graph_configured: false` and asks the operator to Apply the saved graph. If a previously applied pool file is saved with new bytes, `configuration_diagnostic.code` is `pending_apply` and Run remains fenced until Apply. If a selected binary changed after Apply, `configuration_diagnostic.code` is `generated_launch_input_mismatch`, `ready` is false, and the action asks for another Apply. An invalid selected snapshot reports its diagnostic instead of being presented as an unsaved graph. A stopped but ready core asks the operator to run the saved graph. A core with owner layers but no process node reports that missing process explicitly. A truly empty graph leaves EOS and EI disabled without an error.

For the health-only three-core graph, EOS owner state `ready` and EI owner state `running` both set `running: true`. This field reflects the owner worker reported by deployment status; `ready` separately describes whether the saved configuration can run. If an owner fails, deployment status becomes degraded and the workbench records the affected owner state in its browser log.

## GET /api/isos/v1/cores/setup-status

Returns `data.ready`, `issues`, `graph_revision`, backend choices, and process summary. Example response: `{"schema_version":"0.1.0","ok":true,"data":{"ready":false,"issues":["selected owner setup is unavailable"],"graph_revision":null,"backends":[],"process":null},"error":null}`.

## GET /api/isos/v1/cores/setup/{role}/catalog

Returns the selected EOS or EI owner setup catalog for the saved graph without publishing configuration. The route accepts only `eos` or `ei`; an unavailable owner returns 503.

## POST /api/isos/v1/cores/setup/{role}/validate

Request `{"schema_version":"0.1.0","graph":{"version":"0.2.0","robot":{"id":"robot","name":"Robot"},"cores":{"isos":{"layers":{}}}},"revision":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}`. IsOS passes the draft and revision to the selected owner; no configuration is published. The owner reply contains numeric `schema_version:2`, `ok`, `diagnostics`, `files`, and optional `identity` or `catalog`.

## POST /api/isos/v1/cores/setup/{role}/render

Uses the same request shape as validate and previews bounded relative generated files without publishing them. A missing graph is 400; unknown role or operation is 404; timeout, malformed reply, or setup identity without `setup_schema_version:2` is 503.

`schema_version` is string/— and must equal `0.1.0`; `role` is a closed string/— to prevent selecting arbitrary executables; `graph` is the registered profile object/—; `revision` is an optional 64-character lowercase SHA-256 string when the caller already has a saved-byte fence. The external setup wire uses schema 2 and bounded stdin/stdout; a product version does not imply setup compatibility.

## Failure example and field contract

For `POST /api/isos/v1/cores/setup/eos/validate`, an illustrative structured failure is:

```json
{
  "schema_version": "0.1.0",
  "ok": false,
  "data": null,
  "error": {
    "code": "owner_unavailable",
    "message": "required owner is unavailable",
    "details": null,
    "correlation_id": "req-42"
  }
}
```


## Verification

For a graph with `isos.ei_cognition.builder@v0.2` under `ei_cognition/context`, readiness must use the same selected EI stage catalog as admission. An EOS or EI event with a node path must name a node in the saved `isos` graph under its owner's layer and pass the supervisor's admitted-node set. Replays should compare this endpoint with deployment status, selected owner events, and Stop results.

The focused check for this behavior uses `ISOS_AETHER_BIN_DIR=<workspace>/target/debug RUST_TEST_THREADS=1 cargo test -p isos`, `RUST_TEST_THREADS=1 cargo test -p aether-runtime --test grpc_skills`, `cargo fmt --all -- --check`, and `cargo clippy -p isos -p aether-runtime --all-targets -- -D warnings`. Save complete output under `data/test/<run-id>/`.

For the missing-generation case, a manual same-revision Apply regenerated EOS launch inputs; readiness changed from `graph_configured: false` to `true` with action `Run the saved graph`. The UI typecheck, 208 Vitest tests, local and hosted Web builds, TUI build, Rust formatting and Clippy, and bilingual link audit passed.
