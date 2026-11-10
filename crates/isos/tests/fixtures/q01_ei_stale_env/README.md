# Q01 EI test environment guard fixtures

The two cases are graph-authored settings for the IsOS-owned `ei` process node. Validate and Apply must reject each at the exact JSON Pointer with `ei_test_reserved_env`, before writing graph bytes or starting a child. `env_from` sources are not reserved by this guard; only the target key is interpreted by EI. A manager-inherited `EI_Q01_STALE_TEST=1` remains available for a disposable selected run.
