#!/usr/bin/env python3
"""Check P05 bilingual links and shared contract identifiers."""

import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[5]
PAIRS = [
    ("docs/isos/interfaces/package/README.md", "docs_cn/isos/interfaces/package/README_cn.md"),
    ("docs/node/interfaces/graph.md", "docs_cn/node/interfaces/graph_cn.md"),
    ("docs/isos/interfaces/api/http.md", "docs_cn/isos/interfaces/api/http_cn.md"),
    ("docs/isos/core/server.md", "docs_cn/isos/core/server_cn.md"),
    ("docs/isos/start/quick-start.md", "docs_cn/isos/start/quick-start_cn.md"),
    ("crates/isos/tests/fixtures/reader_features/README.md", "crates/isos/tests/fixtures/reader_features/README_cn.md"),
]
SHARED = [
    "supported_graph_versions", "supported_node_type_versions", "supported_node_types",
    "package-process-v1", "reader_identity_invalid", "reader_graph_unsupported",
    "reader_type_unsupported", "reader_feature_unsupported", "reader_binary_mismatch",
    "reader_package_pin_mismatch",
    "managed-config-v1", "eos-gateway-client-v2", "uv sync", "/settings/role",
]


def main() -> None:
    for en, cn in PAIRS:
        for name in (en, cn):
            path = ROOT / name
            text = path.read_text()
            for target in re.findall(r"\[[^]]+\]\(([^)]+)\)", text):
                if target.startswith(("http://", "https://", "/")):
                    continue
                destination = path.parent / target.split("#", 1)[0]
                assert destination.exists(), (name, target)
    en = (ROOT / PAIRS[0][0]).read_text()
    cn = (ROOT / PAIRS[0][1]).read_text()
    for token in SHARED:
        assert token in en and token in cn, token
    print("P05 bilingual links and shared identifiers valid")


if __name__ == "__main__":
    main()
