# Southbound EOS interface

[中文](../../../../docs_cn/ei/design/io/eos-exchange.md) · [Body](body.md) · [World](world.md) · [Primitives](primitives.md) · [Skills](skills.md) · [EOS 2 wire examples](../../interfaces/eos.md)


EOS owns the wire contract and EI consumes its generated bindings. See the [EI–EOS contract boundary](../../../island/architecture.md#cross-owner-contracts) and [operation examples](../../interfaces/eos.md) for the exact flow.


## Data flow

\`\`\`mermaid
flowchart LR
  discover["Gateway discovery"] --> generation["Atomic robot/device generation"]
  capabilities["CapabilityService catalog"] --> binding["Exact ready capability join"]
  generation --> binding
  resources["Read/watch resources"] --> typed["Canonical schema validation"]
  binding --> registry["Executable registry"]
  typed --> perception["World + detection + ASR admission"]
  safety --> operation["SubmitOperation with operation_id"]
  operation --> terminal["WatchOperation terminal acknowledgement"]
  interrupt["Timeout or cancellation"] --> cancel["CancelOperation by operation_id"]
  cancel --> terminal
  terminal --> mission["Mission outcome + structured log"]
\`\`\`

Every executable binding must be healthy, match the installed schema at its exact version, and pass local policy. Unhealthy bindings remain visible for diagnostics but cannot execute. Refresh replaces the full generation atomically.

## Interface groups

- [Body](body.md) covers robot/device identity, state, health, binding scope, rediscovery, and status.
- [World](world.md) covers typed world state, detection, speech, resource watches, and camera capture.
- [Primitives](primitives.md) covers action versions, durable operation identity, terminal outcomes, explicit cancellation, and uncertain results.
- [Skills](skills.md) covers descriptors and asynchronous execution through the dedicated skill service.

