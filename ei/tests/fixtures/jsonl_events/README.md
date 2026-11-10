# EI P06 event fixtures

[中文](README_cn.md) · [Owner interface](../../../../docs/ei/interfaces/events.md) · [Design](../../../../docs/ei/design/events.md)

`ei-role-policy.json` is the normative machine-readable list of the 16 EI codes, fixed messages, severities (including outcome-dependent terminal and stop-state severity), node rules and typed field/enum domains. EI uses this reviewed catalog for local lifecycle JSONL records; IsOS reads the file without a stderr event transport. `cases.json` names 45 positive, rejection, suppression and outage cases for implementation tests, including all four `ei_stopped` state/level pairings. `python3 tests/fixtures/jsonl_events/check_cases.py` validates the catalog and representative ID, role, field and byte boundaries. The checker is design evidence, not a runtime emitter test.

Implementation should replay every named case with fake clocks, stub EOS receipts, isolated local core JSONL. Then select an actual EI child under IsOS and inspect the accepted event in `data/ei/log/<deployment>/ei.jsonl`, verify the first failure and terminal event, and prove owner control survives a local logging failure. A startup failure before EI can safely emit belongs to IsOS supervision. Preserve full output under ignored `data/test/<run-id>/` and summarize commands/results in the design page.
