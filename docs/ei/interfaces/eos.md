# EOS 2 capability operations

[中文](../../../docs_cn/ei/interfaces/eos.md) · [Link transport](../design/io/link.md) · [Shared EOS contract](../../island/architecture.md#cross-owner-contracts)


## Capability identity and admission


```text
hardware/{robot_id}/{device_class}/{part_id}/{primitive}@{version}
runtime/{robot_id}/{primitive}@1.0.0
```

For example:

```text
hardware/robot-123/base/base-0/base.velocity@1.0.0
runtime/robot-123/robot.stop@1.0.0
```


## Operation wire exchange


Request:

```json
{
  "capability_name": "hardware/robot-123/base/base-0/base.velocity@1.0.0",
  "arguments": {
    "linear_x_m_s": 0.1,
    "linear_y_m_s": 0.0,
    "angular_z_rad_s": 0.0,
    "valid_for_ms": 100
  },
  "operation_id": "75a852c0-894a-4d93-b1b9-31fd642621cd"
}
```

A pending operation has no accepted field and no terminal acknowledgement:

```json
{
  "operation_id": "75a852c0-894a-4d93-b1b9-31fd642621cd",
  "status": "OPERATION_STATUS_PENDING"
}
```


```json
{
  "operation_id": "75a852c0-894a-4d93-b1b9-31fd642621cd",
  "status": "OPERATION_STATUS_SUCCEEDED",
  "accepted": true,
  "acknowledgement": {
    "type": "primitive.ack",
    "primitive": "base.velocity",
    "primitive_version": "1.0.0",
    "accepted": true,
    "reason": null,
    "reason_code": null,
    "command_id": "75a852c0-894a-4d93-b1b9-31fd642621cd",
    "sequence": 42,
    "result": {
      "status": "succeeded"
    }
  }
}
```

Only terminal SUCCEEDED with an acknowledgement result of succeeded, matching operation ID, primitive, version, and accepted value is success. A terminal status and acknowledgement that disagree are a protocol error. Rejected, failed, cancelled, timed out, and outcome-unknown results are not success.

## Cancellation and uncertain outcomes

Cancellation is an explicit CancelOperation request using the same durable operation ID:

```json
{
  "operation_id": "75a852c0-894a-4d93-b1b9-31fd642621cd"
}
```


The transport deliberately does not interpret a successful gRPC submission as successful physical completion. A response must be terminal, or the watch stream must yield a matching terminal operation. Operation identity is checked on every response and update.

## Verification

The stubbed contract tests cover robot identity, exact admitted bindings, command identity, pending-to-terminal flow, cancellation, malformed acknowledgements, and non-retry of uncertain outcomes. Public release additionally requires a real EOS process integration test against an exact public EOS release.
