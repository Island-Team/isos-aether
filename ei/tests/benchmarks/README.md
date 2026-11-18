# EI Q03 fixed owner benchmarks

These are local evidence measurements, not capacity or release thresholds. Run
from a pinned checkout with `uv sync --extra dev --extra vec`. Save JSON under an
ignored `data/test/<run-id>/ei-bench/` directory and retain command output.
The default invocation runs one warm-up and five measured repetitions:

```sh
uv run python tests/benchmarks/ei_q03.py --case stage-baseline --output data/test/<run-id>/ei-bench/stage-baseline.json
uv run python tests/benchmarks/ei_q03.py --case stage-contended --output data/test/<run-id>/ei-bench/stage-contended.json
uv run pytest -q tests/benchmarks/test_ei_q03.py
```

`--items` and `--runs` are diagnostic overrides; release evidence uses 10,000
stage offers and five measured runs. Stage resource
samples use `{"backend":"robot-a","reference":"robot","sample":{"eos_robot_id":"robot-a","text":"ready"}}` (92 canonical bytes, SHA-256 `e389fc4fa4bda35e7565b6d2d42b6c8b1625c1ba9935b4bd4f8f374f654d1955`) with `r-00000` through
`r-09999`; contended observations independently use `{"items":[]}` (12 bytes,
SHA-256 `eef46741adfc3a9f76294d3b78f37a45f113092ac9d44ee77c7a038a88ff09a1`) with
`o-00000` through `o-09999`. The JSON output records both exact canonical
published payload hashes and lengths. Each 1 ms offer tick publishes resource first and,
in contention, observations second, then drains exactly one ready item. The
post-offer barrier drains once per 1 ms until `delivered + overflow = offered`
on each active route with no pending value, or fails after two seconds. Stage
latency is monotonic ingress-to-handler. The single latest-value slot may
coalesce offers; the benchmark verifies IDs, counters and post-Stop silence.

Neither stage case sends actuator or safety traffic. Results are offered-rate-limited and do not represent maximum capacity.
