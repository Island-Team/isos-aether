# EI stage graph fixture

[中文](README_cn.md)

`valid_ei.json` contains the seven registered EI stage nodes and their local evidence edges in graph format `0.2.0`. The graph has one `isos` core with `ei_*` layers and current `isos.ei_*.*@v0.2` node type IDs. The gateway port is `16200`, matching the selected EOS gRPC endpoint.

`unified_owner_replay_cli.py` adds these layers to a fresh IsOS starter graph, then exercises real EI and EOS owner validation, revision-bound rendering, and rejection without changing the saved graph. Run `PYTHONPATH=crates/isos/tests python3 -m unittest -q unified_owner_replay_cli` after building the selected binaries and installing EI's Python dependencies.
