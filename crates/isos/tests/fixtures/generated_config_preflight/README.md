# P04 generated configuration fixtures

The [IsOS instance contract](../../../../../docs/isos/interfaces/node/instance.md) owns this schema. `cases.json` defines the diagnostic codes, stages, paths, and recovery states checked by `generated_config_preflight_cli.py` against a fresh `ISOS_HOME` and a format `0.2.0` graph with one `isos` core.

The replay selects installed EI component and factory default trees, renders the managed overlays, and checks the selected generation and manifest. Repeated Apply must preserve byte identity; changing a selected input must create a distinct immutable generation. Invalid defaults, escaping paths, rejected owner output, changed package receipts, tampered generated files, stale selection, and missing child secrets fail before process replacement. Run and Restart rejection preserve the active deployment.

The replay also checks transaction recovery around selection-map, required-marker, and graph publication, using the observed saved-byte revision to choose a complete generation. It checks private directory and file permissions, sorted manifest entries, stable generation hashes, and exclusion of secrets from graphs, manifests, logs, and API errors. Complete test output belongs under ignored `data/test/<run-id>/`.
