# Saved node instance

[中文](../../../../docs_cn/isos/interfaces/node/instance_cn.md) · [Profile](profile.md) · [Type](type.md)

Graph format `0.2.0` has exactly three core keys: `isos`, `ei`, and `eos`. Instances live at `cores.<core>.layers.<layer>.groups.<group>.nodes.<id>`. The map key equals `id`; `node_type` selects a registered `<core>.<layer>.<group>.<type>@v0.2` at that exact placement. The canonical reference path is `<core>/<layer>/<group>/<id>`.

| Field | Meaning |
| --- | --- |
| `id`, `node_type` | Instance identity and registered wrapper type. |
| `label`, `description` | Display text. |
| `settings` | Only settings declared by the selected type. |
| `inputs` | References to declared source ports. |
| `package` | Exact verified release pin when a package type requires one. |

The starter graph contains one IsOS button, three EI nodes, and four EOS nodes, with seven declared edges. The saved graph does not contain live process state, owner socket paths, credentials, event values, or a hardware command. IsOS owns revisions and Apply; EI and EOS receive their respective core projections. The EOS mock arm instance declares six joints and its adapter and movement limits. See the [instance schema](../../../../share/isos/contracts/node/node-instance.schema.json).
