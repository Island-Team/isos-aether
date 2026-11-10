# Control-grant boundary

[中文](../../../../docs_cn/isos/interfaces/core/control-grants_cn.md) · [Design](../../design/README.md)

The implemented IsOS grant bundle consists of private per-run files. The two control grant files are random, distinct secrets delivered only through reserved owner environment paths. The private `verifier.json` has the exact fields below. IsOS checks owner-only directory/file permissions, no symlink ancestors, the selected run and revision, and grant hashes. Stop and replacement revoke the files. EOS owns actuator command admission; a package contribution cannot create another route.

| `verifier.json` field | Type / unit | Purpose and validation | Absence/default |
| --- | --- | --- | --- |
| `schema_version` | integer / version | Current private verifier value `1`. | Required. |
| `run_id` | string / run identity | Exact bundle generation. | Required. |
| `active_revision` | lowercase 64-hex / SHA-256 | Binds saved graph bytes. | Required. |
| `operator_control_sha256` | lowercase 64-hex / grant bytes | Digest of second private grant. | Required. |
