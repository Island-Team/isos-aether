# First use

[中文](../../../docs_cn/isos/start/quick-start_cn.md) · [Node contract](../interfaces/node/README.md)

The starter graph contains three cores and 16 connected nodes. Its IsOS button passes an activation through EI input, cognition, and safety, then EOS ingress, demo skill, dispatcher, and mock six-joint arm. A separate read-only path returns the same arm's state through EOS and EI to the IsOS monitor node.

1. Build IsOS, `aether-profile`, and `aether-runtime`, and install the EI programs from `ei/pyproject.toml`. Set `ISOS_AETHER_BIN_DIR` and `ISOS_EI_BIN_DIR` to the selected executable directories.
2. Run `isos config init`. The saved demo graph is at the selected IsOS home under `config/graph-pool/demo.nodes.json`; the deployment stores only its pool-name binding.
3. Validate and Apply that pool file. Apply requires its exact-byte SHA-256 through `--revision`; `config apply --file` accepts only a file in the graph pool.
4. Open the workbench with `isos web` (or `isos serve` for a foreground server). This starts IsOS and idle owner catalog services; it does not start EI or EOS deployment workers. Moving nodes changes only the browser's saved canvas layout. When ready to execute, use **Save and Run** in the workbench or explicitly run `isos start` / `isos run`. Then read `isos status --json`: EI should report `running`, EOS `ready`, and the EOS mock adapter `ready`.
5. In the workbench, click **Activate** on the button node. The button first shows an accepted event ID, then reads the correlated result separately; each distinct accepted activation moves the mock arm by a small fixed joint delta when it succeeds. Inspect the final positions, the independent arm monitor snapshot, and owner logs. Then run `isos deployment stop` to stop both owners. Press Ctrl+C to close a foreground `isos run` session. `isos stop` invokes the workspace emergency stop using the configured code.

After changing a selected core binary, Apply the graph again before Run to refresh the verified launch-input digest. IsOS can exit while EI and EOS keep running; restart IsOS and read status to reconnect. A failed owner is reported separately while the healthy owner remains live. See the [runtime design and replay](../design/runtime/owner-health.md) and [graph API](../interfaces/http/graph.md).

Use `isos web` when editing the graph without running the deployment. `isos run` is a foreground command that explicitly starts both deployment workers; ending its terminal session also ends that workbench server. A detached `isos web` server remains available while the graph is edited.
