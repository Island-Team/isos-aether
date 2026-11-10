
[中文](../../../../docs_cn/ei/design/runtime/optional.md)

## Contents

- [Architecture](#architecture)
- [Data flow](#data-flow)
- [Conditions](#conditions)
- [Related documentation](#related-documentation)

## Architecture

```mermaid
flowchart LR
  Memory["Core memory"] -->|"RecallIndex"| Vector["opt.memory"]
  Media["Sandbox media client"] --> Sandbox["opt.sandbox child"]
  Core["Core runtime"] -. "continues when omitted" .-> Runtime
```

## Owned modules

`memory/vec.py` provides the sqlite-vec recall index behind the internal `RecallIndex` protocol. It embeds completed plans, replaces index rows by stable plan ID, and returns ranked IDs for core memory to resolve against authoritative records.

`sandbox/__main__.py` starts the isolated playground. `sandbox/server.py` owns its local EOS-compatible and media endpoints, while `sandbox/devices.py` produces configured microphone, camera, and speaker behavior. The sandbox is a development input surface, not a production device runtime or a safety bypass.

## Data flow

For optional memory, core supplies a completed plan or query text. The vector implementation creates a deterministic embedding, updates or queries its index, and returns identifiers and scores; core reloads the matching plan records, drops unknown IDs, and constructs the bounded recall digest.


## Conditions

Normal vector indexing replaces an existing plan entry with the same stable ID and ranks available matches. An empty index or empty authoritative corpus produces no recall. Malformed stored vectors, missing sqlite-vec support, database failures, or unknown returned plan IDs degrade optional recall without changing episodic-memory authority.

Normal sandbox startup exposes only configured loopback services and enabled devices. Disabled devices produce their documented disabled state; empty media produces no fabricated observation. Invalid configuration, non-loopback binding, malformed media, unavailable child IPC, or child startup timeout keeps the sandbox route unavailable without taking down the northbound API or core runtime.

Restart rebuilds process-local sandbox state and reopens optional vector storage. On shutdown, vector connections close; sandbox tasks, sockets, device resources, and the child process are stopped. With either feature disabled or its dependency absent, composition uses the documented core behavior rather than silently emulating it.

## Related documentation

See [memory.md](../optional/memory.md) for vector recall details, [sandbox.md](../optional/sandbox.md) for the playground protocol, [memory design](../cognition/memory.md) for authoritative memory, [EOS exchange design](../io/eos-exchange.md) for EOS validation, and [EI runtime interface](../../interfaces/runtime.md) for sandbox proxy behavior.
