
[中文](../../../../docs_cn/ei/design/io/link.md)

中文 · Core overview · EOS 2 operation examples

Link owns one EOS gRPC client per configured stable robot alias. It publishes complete body/resource generations, reads and watches typed world resources, captures camera frames on demand, executes versioned primitives, and exposes the monitor's direct stop path. EOS drivers and providers remain private implementation details.

## Resources and camera capture

ListResources exposes the supported typed world, detection, and speech resources. WatchResource announces changed URIs; Link starts the watch before its initial read, rereads on notifications, and performs bounded periodic rereads to cover missed notifications. Only schema-valid snapshots are published. Camera frames are retrieved on demand through CaptureFrames.

## Primitive operations



Link returns rejected, failed, cancelled, timed-out, and outcome-unknown acknowledgements as non-success. A successful submission RPC alone does not mean the physical action succeeded. If the watch ends without a terminal result, or response identity is inconsistent, Link raises a transport/contract error.

Cancellation uses explicit CancelOperation with the original operation ID only for cancellable action classes. Link waits up to five seconds for a terminal cancellation boundary. It never automatically resubmits after timeout, disconnection, or OUTCOME_UNKNOWN. Fresh state reconciliation or operator intervention is required.

When a Submit reply or Watch stream is lost, Link queries GetOperation under the same grant and ID. A canonical matching terminal result settles that call; failed lookup leaves the result uncertain and fences later writes pending rediscovery. Stop and emergency use the fixed safety/monitor route; authored stage edges carry evidence and cannot command the gateway.

## Transport conditions

Before initialization, discovery-dependent properties raise rather than returning authoritative empty data. A forced initialize replaces the complete generation atomically. Unknown resources, versions, targets, cameras, and skills fail closed. gRPC status and details are preserved in link errors, source owners control stream reconnection, and closing releases the channel.

**Related:** southbound, registry, safety, and the EOS compatibility record.

## Registered node behavior

**Inputs:** gateway; admitted and emergency_stop are governed internal routes. **Outputs:** discovery, resource, result, health. The [stage dataflow design](../pipeline.md) describes EI runtime values. IsOS owns the graph port and setting contract, including wire priority and frequency.

**Algorithm and failure:** Resolve the IsOS-provided endpoint, discover resources and capabilities, verify exact identities, then atomically publish a generation. On disconnect mark the generation stale and block new actions; close channels after dependents stop.
