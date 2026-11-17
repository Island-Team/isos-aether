"""Deterministic Q03 owner benchmarks; measurements are not performance gates."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import platform
import time
from datetime import UTC, datetime
from pathlib import Path

from ei.core.system.stages import (
  STAGE_TYPES,
  StagePayload,
  StageRuntime,
  StageValue,
  compose_stages,
)

_JPEG = (
  b"\xff\xd8\xff\xc0\x00\x0b\x08\x00\x01\x00\x01\x01\x01\x11\x00"
  b"\xff\xda\x00\x08\x01\x01\x00\x00\x3f\x00\x00\xff\xd9"
)
_GRAPH_REVISION = "0" * 64
_PROJECTION_REVISION = "a" * 64


def percentile(samples: list[int], rank: float) -> float:
  """Nearest-rank percentile in milliseconds, preserving raw nanoseconds."""
  if not samples:
    raise ValueError("latency samples are empty")
  ordered = sorted(samples)
  return ordered[max(0, math.ceil(len(ordered) * rank) - 1)] / 1_000_000


def accounting(offered: int, delivered: int, overflow: int, pending: int) -> None:
  if delivered + overflow != offered or pending:
    raise RuntimeError(
      f"barrier accounting failed: {offered=} {delivered=} {overflow=} {pending=}"
    )


def drain_barrier(
  runtime: StageRuntime,
  edges: dict,
  offered: dict[str, int],
  *,
  interval_ns: int,
  timeout_ns: int = 2_000_000_000,
) -> None:
  deadline = time.monotonic_ns() + timeout_ns
  while any(
    runtime.edge_counters[edge].delivered + runtime.edge_counters[edge].overflow
    != offered[name]
    or edge in runtime._pending
    for name, edge in edges.items()
    if offered[name]
  ):
    if time.monotonic_ns() >= deadline:
      raise TimeoutError("stage drain barrier exceeded 2 seconds")
    time.sleep(interval_ns / 1_000_000_000)
    runtime.drain_ready(limit=1)


def _graph(*, contended: bool) -> dict:
  nodes = {}
  paths = {}
  for spec in STAGE_TYPES.values():
    node_id = spec.id.split(".")[2].split("@")[0]
    path = f"isos/ei_{spec.layer}/{spec.group}/{node_id}"
    paths[spec.group] = path
    nodes[path] = {
      "id": node_id,
      "description": spec.description,
      "node_type": spec.id,
      "settings": (
        {
          "backend_alias": "robot-a",
          "gateway": {"transport": "tcp", "host": "127.0.0.1", "port": 16022},
        }
        if spec.group == "link"
        else {}
      ),
      "inputs": {},
    }
  wires = (
    ("perception", "resource", "link", "resource"),
    ("perception", "result", "link", "result"),
    ("context", "observations", "perception", "observations"),
    ("decision", "selected", "missions", "selected"),
    ("decision", "snapshot", "context", "snapshot"),
    ("safety", "proposal", "decision", "proposal"),
    ("monitor", "health", "link", "health"),
  )
  for target, input_name, source, output in wires:
    nodes[paths[target]]["inputs"][input_name] = {
      "node": paths[source],
      "output": output,
    }
  nodes[paths["perception"]]["inputs"]["resource"]["priority"] = 0
  if contended:
    nodes[paths["perception"]]["inputs"]["resource"]["frequency_hz"] = 30
    nodes[paths["context"]]["inputs"]["observations"]["priority"] = -10
  layers: dict = {}
  for path, node in nodes.items():
    _, layer, group, node_id = path.split("/")
    layers.setdefault(layer, {"groups": {}})["groups"].setdefault(group, {"nodes": {}})[
      "nodes"
    ][node_id] = node
  return {"version": "0.2.1", "robot": {}, "cores": {"isos": {"layers": layers}}}


def stage_run(
  items: int, *, contended: bool, interval_ns: int = 1_000_000, fail_owner: bool = False
) -> dict:
  graph = _graph(contended=contended)
  resource_payload = {
    "backend": "robot-a",
    "reference": "robot",
    "sample": {"text": "ready", "eos_robot_id": "robot-a"},
  }
  observations_payload = {"items": []}
  payloads = {}
  for name, fields in (
    ("resource", resource_payload),
    ("observations", observations_payload),
  ):
    encoded = json.dumps(fields, sort_keys=True, separators=(",", ":")).encode()
    payloads[name] = {
      "sha256": hashlib.sha256(encoded).hexdigest(),
      "bytes": len(encoded),
    }
  runtime = StageRuntime(compose_stages(graph), "q03-bench", auto_drain=False)
  for path in runtime.composition.instances:
    runtime.bind_owner(path, object())
  edges = {
    edge.output: edge
    for edge in runtime.composition.edges
    if edge.output in {"resource", "observations"}
  }
  ingress: dict[str, int] = {}
  delivered: dict[str, list[dict]] = {name: [] for name in ("resource", "observations")}

  def handler(value: StageValue) -> None:
    if fail_owner:
      raise RuntimeError("injected owner failure")
    at = time.monotonic_ns()
    delivered[value.kind].append(
      {
        "id": value.event_id,
        "ingress_ns": ingress[value.event_id],
        "delivered_ns": at,
        "latency_ns": at - ingress[value.event_id],
      }
    )

  for edge in edges.values():
    runtime.bind(edge.target, edge.input, handler)
  runtime.start()
  start = time.monotonic_ns()
  offered = {"resource": items, "observations": items if contended else 0}
  for index in range(items):
    deadline = start + index * interval_ns
    delay = deadline - time.monotonic_ns()
    if delay > 0:
      time.sleep(delay / 1_000_000_000)
    now = datetime.now(UTC)
    resource_id = f"r-{index:05d}"
    ingress[resource_id] = time.monotonic_ns()
    runtime.publish(
      "resource",
      StageValue(
        "resource",
        resource_id,
        "q03-bench",
        edges["resource"].source,
        now,
        StagePayload(
          "resource",
          resource_payload,
        ),
        received_at=now,
      ),
    )
    if contended:
      observation_id = f"o-{index:05d}"
      now = datetime.now(UTC)
      ingress[observation_id] = time.monotonic_ns()
      runtime.publish(
        "observations",
        StageValue(
          "observations",
          observation_id,
          "q03-bench",
          edges["observations"].source,
          now,
          StagePayload("observations", observations_payload),
          received_at=now,
        ),
      )
    runtime.drain_ready(limit=1)
  final_offer = time.monotonic_ns()
  drain_barrier(runtime, edges, offered, interval_ns=interval_ns)
  end = time.monotonic_ns()
  routes = {}
  for name, edge in edges.items():
    counters = runtime.edge_counters[edge]
    if offered[name]:
      accounting(
        offered[name],
        counters.delivered,
        counters.overflow,
        int(edge in runtime._pending),
      )
    rows = delivered[name]
    if len(rows) != counters.delivered or len({row["id"] for row in rows}) != len(rows):
      raise RuntimeError("delivered identities disagree with counters")
    if any(
      not row["id"].startswith("r-" if name == "resource" else "o-") for row in rows
    ):
      raise RuntimeError("wrong route identity")
    samples = [row["latency_ns"] for row in rows]
    routes[name] = {
      "offered": offered[name],
      "accepted": offered[name],
      "delivered": counters.delivered,
      "throughput_delivered_per_second": counters.delivered / ((end - start) / 1e9),
      "overflow": counters.overflow,
      "gaps": counters.gaps,
      "median_ms": percentile(samples, 0.5) if samples else None,
      "p99_ms": percentile(samples, 0.99) if samples else None,
      "samples": rows,
      "id_digest": hashlib.sha256(
        "\n".join(row["id"] for row in rows).encode()
      ).hexdigest(),
    }
  if any(item.code not in {"evidence_overflow"} for item in runtime.diagnostics):
    raise RuntimeError("unexpected stage diagnostic")
  runtime.stop()
  if runtime.drain_ready(limit=1) or runtime._pending:
    raise RuntimeError("post-stop delivery or pending evidence")
  return {
    "case": "stage-contended" if contended else "stage-baseline",
    "payloads": payloads,
    "elapsed_seconds": (end - start) / 1e9,
    "post_offer_drain_seconds": (end - final_offer) / 1e9,
    "throughput_delivered_per_second": sum(
      route["delivered"] for route in routes.values()
    )
    / ((end - start) / 1e9),
    "routes": routes,
    "errors": [],
  }


def main() -> None:
  parser = argparse.ArgumentParser()
  parser.add_argument(
    "--case", choices=("stage-baseline", "stage-contended"), required=True
  )
  parser.add_argument("--output", type=Path, required=True)
  parser.add_argument(
    "--items", type=int, help="Diagnostic sample count; release defaults are 10000/1000"
  )
  parser.add_argument("--runs", type=int, default=5)
  args = parser.parse_args()
  if args.runs < 1 or (args.items is not None and args.items < 1):
    parser.error("runs and items must be positive")
  items = args.items or 10000
  run = lambda: stage_run(items, contended=args.case == "stage-contended")
  warmup = run()
  measured = [run() for _ in range(args.runs)]
  rates = [
    item.get(
      "throughput_accepted_per_second", item.get("throughput_delivered_per_second")
    )
    for item in measured
  ]
  summary = {
    "case": args.case,
    "items": items,
    "warmup_elapsed_seconds": warmup["elapsed_seconds"],
    "runs": args.runs,
    "throughput_median": sorted(rates)[len(rates) // 2],
    "throughput_range": [min(rates), max(rates)],
    "clock": "time.monotonic_ns",
    "host": {
      "platform": platform.platform(),
      "cpu_count": os.cpu_count(),
      "python": platform.python_version(),
    },
    "measurements": measured,
  }
  args.output.parent.mkdir(parents=True, exist_ok=True)
  args.output.write_text(json.dumps(summary, indent=2) + "\n")
  print(
    json.dumps(
      {key: value for key, value in summary.items() if key != "measurements"},
      sort_keys=True,
    )
  )


if __name__ == "__main__":
  main()
