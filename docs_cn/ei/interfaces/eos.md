# EOS 2 能力操作

[English](../../../docs/ei/interfaces/eos.md) · [Link transport](../design/io/link.md) · [EOS 共享契约](../../island/architecture_cn.md#跨所有者契约)


## 能力身份与准入


```text
hardware/{robot_id}/{device_class}/{part_id}/{primitive}@{version}
runtime/{robot_id}/{primitive}@1.0.0
```

例如：

```text
hardware/robot-123/base/base-0/base.velocity@1.0.0
runtime/robot-123/robot.stop@1.0.0
```


## Operation 线协议交换


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

Pending operation 不含 accepted 字段，也不含终态 acknowledgement：

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

只有 acknowledgement 中的终态 SUCCEEDED 且 result 为 succeeded，并且 operation ID、primitive、version 和 accepted 值均匹配时才算成功。终态与 acknowledgement 不一致属于协议错误。Rejected、failed、cancelled、timed out 和 outcome unknown 都不代表成功。

## 取消与不确定结果

取消是显式的 CancelOperation 请求，并使用相同的持久 operation ID：

```json
{
  "operation_id": "75a852c0-894a-4d93-b1b9-31fd642621cd"
}
```


传输层不会将成功的 gRPC submission 当作物理执行成功。响应必须已经处于终态，或由 watch stream 返回身份匹配的终态 operation。每个响应和更新都会校验 operation identity。

## 验证

Stub contract 测试覆盖 robot identity、精确准入绑定、command identity、pending 到 terminal 流程、取消、错误 acknowledgement，以及不重试不确定结果。公开发布还要求针对某个精确公开 EOS release 完成真实 EOS process 集成测试。
