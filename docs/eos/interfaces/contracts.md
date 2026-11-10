# EOS contract sources

> Historical contract. This interface is not implemented by the current button-to-mock-arm graph. See [arm control](arm-control.md) for the active data and command path and [owner lifecycle](owner-health.md) for setup, Run, Status, and Stop.


[中文](../../../docs_cn/eos/interfaces/contracts_cn.md) · [Interface index](README.md) · [gRPC and IPC](grpc.md)

The authoritative source for EOS protobuf and JSON Schemas is `share/eos/contracts/eos/`. `gateway.proto`, `skills.proto`, and `capabilities.proto` define the process-facing service messages used by the current Rust runtime. Domain folders such as `robot/`, `perception/`, `skills/`, `actions/`, `body/`, and `world/` define payloads and validation annotations. The [Rust skill executable contract](skills/rust.md) defines the separate worker protocol.

The generator derives JSON Schemas and a manifest under `share/eos/schemas/eos/`. A protobuf envelope supplies transport identity; the selected generated schema validates bounded arguments and results. Graph node type IDs, process wire schemas, source package versions, and external package versions are separate identities.

## Updating a contract

Edit the owning `.proto` or authored schema, regenerate derived artifacts, and update its interface page and Chinese mirror. Run:

```sh
python3 scripts/generate_eos_contracts.py --output share/eos/schemas/eos
python3 scripts/generate_eos_contracts.py --output share/eos/schemas/eos --check
cargo test --workspace
python3 scripts/check_doc_links.py --all
```

Review generated diffs and add a request, success, rejection, and failure fixture for a public contract change. Keep hardware samples and commands as typed Rust values below the transport boundary.
