# P07 IsosUI log cursor fixtures

[中文](README_cn.md) · [UI contract](../../../../docs/isos/interfaces/api/client.md#log_cursors) · [Design](../../../../docs/isos/core/ui/webui.md)

`cases.json` freezes 30 UI state and service-boundary cases. `check_log_cursors.py` validates stable IDs, bounds, exact P07 query/response names, required safety assertions and English/Chinese documentation links. These fixtures specify target behavior, not implemented UI. Implementation tests should turn every case into a reducer or stub HTTP replay, including an actual IsOS rotation and collector recovery replay. Save complete output under ignored `data/test/<run-id>/`.
