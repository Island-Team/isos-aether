# Catalog HTTP API

[中文](../../../../docs_cn/isos/interfaces/http/catalog_cn.md) · [Envelope](envelope.md) · [Graph](../node/README.md)

The catalog routes expose registered node types, selected types, node options, and node-set previews. They derive results from verified generic `node_contributions` and the selected owner catalog; no package ID changes parsing or priority. All routes require loopback Host/Origin admission.

| Route | Result | State effect |
| --- | --- | --- |
| `GET /api/isos/v1/catalog/node-types` | Registered types and availability. | None. |
| `GET /api/isos/v1/catalog/node-types/selected` | Types referenced by applied graph. | None. |
| `POST /api/isos/v1/catalog/node-options` | Choices for declared settings, stable resource IDs, and contextual EOS/EI owner selectors for the supplied draft. | None. |
| `GET /api/isos/v1/catalog/node-sets` | Verified declared node sets. | None. |
| `POST /api/isos/v1/catalog/node-sets/preview` | Candidate graph composition; never publication. | None. |

Requests use `schema_version:"0.1.0"` where a body is required. Malformed schema is 400; unknown exact pin, type, resource, or owner returns a bounded admission diagnostic. A stale draft revision is 409. Details of field identity and validation are in [graph](../node/instance.md), [options](../node/selectors.md), and [resources](../packages/resources.md).

Owner choice groups retain `node_path`, `setting`, the draft's `selected` value, and bounded `options` with availability and reason fields. IsOS checks owner scope and group identity before returning them. The workbench ties the response to the current draft; a failed owner request leaves the selector unavailable. Draft type catalogs and options use the same selected local EOS/EI setup adapters, even when a persistent owner still runs an older admitted generation. Apply repeats validation with the live owner because a preview can become stale; a rejected Apply cannot start that draft.
