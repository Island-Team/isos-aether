# Current-run log navigation — P07 UI contract

[中文](../../../docs_cn/isosui/interfaces/log-cursors_cn.md) · [Workbench design](../design.md) · [First use](../usage.md)


## Selection and ordering




| UI input or owner field | JSON type / unit | UI validation and use |
| --- | --- | --- |
| `view` | literal `current` / — | Required for P07; any other value is an IsOS error. |
| `source`, `level` | source ID or `all`; severity enum / — | Default `all` and `info`; filter detail only. Do not use source/level to select an anchor. |
| `node_path` | canonical saved-graph node JSON Pointer / — | Optional exact filter, at most 512 UTF-8 bytes; reject malformed or member pointers before request. A rejected filter returns HTTP 400 `log_node_path_invalid` at `/query/node_path`. |
| `correlation_id` | ASCII ID / — | Optional exact filter, 1–64 letters, digits, `_` or `-`; never free-text search. A rejected filter returns HTTP 400 `log_correlation_id_invalid` at `/query/correlation_id`. |
| `limit` | integer / events | Default 100, range 1–200; clamp UI input before request. |
| `cursor`, `next_cursor` | opaque ASCII string / position | Incoming cursor is at most 512 bytes; pass the returned value unchanged with the same query. Never parse or store across runs; changed filters reset it. Reusing a filter-mismatched cursor returns HTTP 400 `log_cursor_filter_mismatch` at `/query/cursor`; a malformed or tampered cursor returns `log_cursor_invalid` at the same path. The fixed diagnostic omits the token and filter values. |
| `scope` | object `{kind,run_id,active_revision}` / — | Required; `kind` is manager or deployment, ID is 1–64 ASCII bytes, active revision is a digest or null. Identity selects log history, not live health. |
| `process-status.run_id`, `run_id_state` | string or null; enum / run identity | `run_id_state` is `none`, `known` or `unavailable`; only `known` has the last admitted run ID, 1–64 ASCII bytes. `none` permits manager scope; `unavailable` blocks a current-run claim. |
| `anchor` | object `{kind,state,event}` / — | Required; pending/unavailable has null event, found has a complete sanitized P06 event. Pinned above all filters. |
| `lines`, `rotated` | event array; Boolean / — | At most `limit` same-scope events strictly after anchor; `rotated` alone does not mean loss. |
| `diagnostics` | array of `{code,count}` / lost events | Stable code from the four listed above; nonnegative count or null if unknown. Show a persistent accessible notice for loss or run change. |

## Rows, errors and bounds




The operator can pause, resume, jump to first event and copy safe IDs. `aria-live=polite` announces a new-run boundary or gap once; streaming rows do not flood announcements. Empty, no matching event, collector unavailable and retention gap have distinct English and Chinese text. The [P07 fixtures](../../../ui/tests/fixtures/log_cursors/README.md) freeze these UI transitions.
