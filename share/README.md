# Shared source resources

Resources in this tree are grouped by the component that owns their source. The repository has one root `share/` tree; installed resource paths remain defined by the installer and product configuration.

| Directory | Owner and contents | Source status |
| --- | --- | --- |
| [`isos/contracts/node/`](isos/contracts/node/README.md) | IsOS authored node type and instance JSON Schemas | Authored contracts |
| [`isos/defaults/`](isos/defaults/island.toml) | IsOS product default `island.toml` | Authored product configuration |
| [`ei/defaults/ei/`](ei/defaults/ei/) | EI factory settings; links point to canonical files in `config/ei/` where those files own the content | Authored settings or links to authored settings |
| [`eos/contracts/`](eos/contracts/eos/README.md) | EOS Protobuf contracts and typed payload source | Authored contracts |
| [`eos/workflows/`](eos/workflows/) | EOS workflow resources | Authored resources |
| [`eos/schemas/eos/`](eos/schemas/eos/) | EOS public runtime JSON Schemas and manifest | Generated; regenerate from EOS contracts |
| [`ext/north-http/openapi/`](ext/north-http/openapi/north-http/openapi.json) | North HTTP adapter description, owned by the extension | Generated; not an EOS contract |

Build and install rules preserve these ownership boundaries and the existing installed paths. Teaching examples live under [`examples/`](../examples/README.md).
