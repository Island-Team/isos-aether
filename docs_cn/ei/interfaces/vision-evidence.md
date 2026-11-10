# EI 视觉证据检查

[English](../../../docs/ei/interfaces/vision-evidence.md) · [阶段契约](../design/pipeline.md) · [托管配置](setup.md)







## 只读检查


```json
{
  "type": "vision_evidence",
  "state": "available",
  "deployment_run_id": "r7-cde",
  "graph_revision": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
  "projection_revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "runtime_stream_id": "8b4e9b70-5481-4d88-9bd6-4f07cb466ae4",
  "source_path": "isos/eos_hardware/camera/front",
  "source_frame_id": "cam-41",
  "gate_path": "isos/eos_interface/gate/front",
  "gate_generation": 7,
  "sequence": null,
  "timestamp_unix_ms": null,
  "width_px": 1280,
  "height_px": 720,
  "render": {
    "max_width_px": 1280,
    "jpeg_quality": 80
  },
  "jpeg_sha256": "c8f1b57ba3b319e7c5ddce3dac7f09c729d8d4bce2259c4f03d68cdbb7f69f0d",
  "jpeg_length_bytes": 12345,
  "accepted_count": 1,
  "rejected_count": 0,
  "rejection_counts": {}
}
```

`jpeg_length_bytes` 是经摘要验证的 JPEG 收到字节数，为 1 至 8 MiB 的正整数；没有当前帧时为 null。摘要只展示格式，并非省略 JPEG 字节的实际摘要。`type` 和 `state` 是必填枚举；已有选定绑定时，`deployment_run_id`、`graph_revision`、`projection_revision`、`source_path`、`gate_path`、`render` 与计数始终存在。`missing` 或 `disconnected` 时，`runtime_stream_id`、`source_frame_id`、`gate_generation`、可选源 `sequence`/`timestamp_unix_ms`、尺寸、摘要及 `jpeg_length_bytes` 为 null；旧身份只能出现于单独标为过时的 `last_accepted` 对象。计数是当前运行的饱和无符号 64 位整数；`rejection_counts` 只包含上述闭集原因。整条 JSON 不超过 4 KiB。检查输出不含 JPEG、授权、端点路径或模型内容，也不代表执行器动作或阶段证据。


## Q01 私有重复代际测试入口


套接字接收一条以换行结束、最长 1024 字节的 UTF-8 JSON 对象，读取超时两秒，键必须恰为：`schema_version`（整数 1）、`op`（`replay_latest`）、`deployment_run_id`（所选 run ID）、`graph_revision`（64 位小写 SHA-256 十六进制）、`runtime_stream_id`（当前已证明 stream ID）、`gate_generation`（正整数）、`source_frame_id`（当前 EOS 帧 ID）和 `jpeg_sha256`（64 位小写 SHA-256 十六进制）。这些值只用于比较当前已接受状态。调用者不能提供 JPEG、替换元数据、端点、授权、图边或命令。额外字段、错误 JSON 和超限请求返回 `invalid_request`；错误的运行或图修订返回 `binding_stale`；当前 stream、代际、帧 ID 或摘要变化返回 `target_changed`。断连或授权撤销返回 `unavailable`；所有者停止返回 `stopped`。只有匹配当前状态的请求消耗一次机会。

