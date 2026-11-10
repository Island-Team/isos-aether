# Reader preflight design

[中文](../../../../docs_cn/eos/design/runtime/reader-features_cn.md) · [Owner contract](../../interfaces/reader-features.md)

EOS advertises the graph version, registered node types, and features accepted by each selected binary. IsOS uses those exact observations to decide whether a saved deployment can run. The profile tool, runtime, and config server derive built-in type IDs from the same registry. Package-contributed types enter only through verified sidecars passed to setup; a binary probe does not claim that a package is installed.

For Validate and Apply, IsOS fixes the saved graph revision, verifies package pins and selected binaries, derives required features from graph nodes and edges, and invokes each binary directly. It checks exact identity, type IDs, and required features before owner rendering or process changes. Run and Restart check the immutable generation and selected binary hashes again. Apply selects a new generation.

Build the EOS binaries with `cargo build --workspace`, then run `python3 tests/reader_features_cli.py`. The replay checks current graph `0.2.0` probe identity, setup identity agreement, and validation of the checked-in mock profile. Save complete output in ignored `data/test/<run-id>/`.
