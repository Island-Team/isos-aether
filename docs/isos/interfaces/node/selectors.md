# Settings and selectors

This page is for type publishers and UI authors. IsOS currently validates instance settings against the registered type and projects selector choices from declared sources. A selector is a setting declaration, not a package-specific form or a request-time actuator parameter. [Resources](../packages/resources.md) explain verified package choices.

| Field | Type / unit | Meaning and validation | Absence/default |
| --- | --- | --- | --- |
| `name` | nonblank string / setting ID | Unique within one registered type. | Required. |
| `data_type` | enum / JSON value kind | `string`, `number`, `integer`, `boolean`, `array`, `object`; current aliases also include `string_array`, `string_table`, `positive_integer`, and `nonnegative_integer`. | Required. |
| `required` | boolean / presence rule | A missing required instance value needs a type-declared default. | Required. |
| `description` | nonblank string / display text | Tells the author what to choose and its unit. | Required. |
| `unit` | nonblank string / physical or display unit | States how a numeric or textual value is measured. | Optional. |
| `min`, `max` | finite numbers / setting unit | Bound numeric instance values; `min` cannot exceed `max`. | Optional; no corresponding bound. |
| `default` | value matching `data_type` | Supplies an omitted instance value. | Optional; no default. |
| `choices` | nonempty array of matching values | Bounds valid selections. | Optional; no finite list. |
| `selector.provider` | nonblank string / choice provider | Names `package`, `graph`, `owner`, or local `ui` choice discovery. `owner` asks the selected EOS or EI setup service for contextual choices; `ui` supplies browser-local choices. | Required when `selector` exists. |
| `selector.resource` | nonblank string / resource family | Names the kind of choices requested. | Required when `selector` exists. |
| `selector.package_from` | nonblank string / setting reference | Selects the exact package pin supplying choices. | Optional. |
| `selector.filter` | object / provider filter | Restricts options without interpreting owner payload. | Optional; empty filter. |

IsosUI renders provider-backed choices from the declaration. Graph, package, and owner choices are previews; Apply repeats validation so stale selections cannot be admitted. A missing owner result blocks the selector instead of permitting unverified text. A key or key chord illustrates a fixed node setting: it belongs to the instance profile, while any emitted signal belongs to the output port's separate payload contract. This slice does not enable a browser key selector or a keyboard control route.

[中文](../../../../docs_cn/isos/interfaces/node/selectors_cn.md)
