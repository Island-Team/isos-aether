# Node data types

This contract is for producers, consumers, and IsosUI renderers. A release contribution or core catalog declares a data type once; registered [ports](ports.md) reference its exact `id`. IsOS rejects duplicate IDs, missing payload metadata, and mismatched port identities before Apply. An incompatible structure or meaning needs a distinct versioned ID. A producer owns payload bytes and authorization.

| Field | Type / unit | Meaning and validation | Absence/default |
| --- | --- | --- | --- |
| `id` | nonblank string / type ID | Stable identity used by ports and edges. | Required. |
| `version` | nonblank string / schema version | Version of this payload meaning, independent of package and HTTP versions. | Required. |
| `purpose` | nonblank string / display text | Explains what consumers may use the value for. | Required. |
| `media_type` | nonblank string / MIME type | Describes representation; consumers must also check codec and schema. | Required. |
| `codec` | nonblank string / encoding ID | Identifies exact encoding such as JSON, UTF-8, image, pixel, or audio encoding. | Required. |
| `schema` | JSON object / payload structure | Owner-defined shape and units for structured values or planes. | Required. |
| `max_payload_bytes` | integer 1–100,000,000 / bytes | Caps one encoded payload before presentation or delivery. | Required. |

The same fields can describe records, text, time series, images, pixel planes, depth, detections, and audio. A renderer chooses a method only after checking the registered port, codec, media type, schema, bound, and its own capabilities; [UI selection](../ui/selection.md) describes that choice. Type names alone do not authorize decoding.

The starter registers `button_activation@v0.2` and `arm_joint_delta@v0.2` as JSON v1 payloads with a 1024-byte bound. Activation has `event_id` and `kind: "activate"`; it contains no movement target. The EOS delta has exactly six finite `joint_deltas_rad` values in radians. IsOS passes the first type through EI; EOS alone creates the second. See [EOS arm control](../../../eos/interfaces/arm-control.md) for bounds and admission.

[中文](../../../../docs_cn/isos/interfaces/node/data-types_cn.md)
