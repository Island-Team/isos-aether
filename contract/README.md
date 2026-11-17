# Contract files: where to start

Use the [contract catalog interface](../docs/isos/interfaces/contract-catalog.md) for ownership and classification rules. [`catalog.json`](catalog.json) gives every contract group a plain-language purpose, origin, consumers, and domain labels. Run `python3 scripts/check_contract_catalog.py` from the repository root for a short summary, or `python3 scripts/check_contract_catalog.py --explain <path>` to understand one file.

| Need | Open | Meaning |
| --- | --- | --- |
| Graph node declarations | [`node/`](node/README.md) | The two JSON Schemas validate registered types and instances; the headers define native worker ABIs. |
| Cross-owner calls and payloads | [`proto/`](proto/README.md) | Protobuf is the authored source for services and typed messages. Folder names here are wire import paths, not graph node placement. |
| Google API HTTP annotations | [`proto/google/api/`](proto/google/api/annotations.proto) | Minimal compatible declarations for the standard `google/api/...` imports; these are not a full Google API distribution. |
| EOS runtime validation | [`schemas/actions/`](schemas/actions/ack.v1.json), [`body/`](schemas/body/base.v1.json), [`world/`](schemas/world/world.v1.json) | Forty JSON Schemas are generated from matching `<domain>/<name>.proto` files. The manifest lists IDs and checksums. Edit the Protobuf source. |
| EI northbound validation | [`schemas/northbound/`](schemas/northbound/primitive_invoke_request.v1.json) | Thirteen schemas are checked against EI wire models. |
| EOS model providers | [`schemas/models/`](schemas/models/asr_request.v1.json) | Private provider-neutral request and response validation. |
| North HTTP API description | [`schemas/north-http.openapi.json`](schemas/north-http.openapi.json) | One generated OpenAPI file from HTTP annotations. Edit the Protobuf annotations. |

The generated EOS JSON files are runtime inputs, not independent contract source. Catalog IDs are for discovery; graph node IDs keep their separate `<core>/<layer>/<group>/<node>` structure. Project Protobuf imports now use paths relative to `proto/`; the `google/api/...` imports keep their standard names.
