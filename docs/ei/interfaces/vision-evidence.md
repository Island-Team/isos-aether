# EI vision evidence inspection

[中文](../../../docs_cn/ei/interfaces/vision-evidence.md) · [Stage contract](../design/pipeline.md) · [Managed configuration](setup.md)







## Read-only inspection


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



## Private Q01 duplicate-generation test seam


The socket accepts one newline-terminated UTF-8 JSON object of at most 1024 bytes, with a two-second read timeout and exactly these keys: `schema_version` (integer 1), `op` (`replay_latest`), `deployment_run_id` (selected run ID), `graph_revision` (64 lowercase SHA-256 hex characters), `runtime_stream_id` (current attested stream ID), `gate_generation` (positive integer), `source_frame_id` (current EOS frame ID), and `jpeg_sha256` (64 lowercase SHA-256 hex characters). These values only compare against current accepted state. The caller supplies no JPEG, metadata replacement, endpoint, grant, graph edge, or command. Extra fields, malformed JSON and oversized requests return `invalid_request`; wrong run or graph revision returns `binding_stale`; a changed current stream, generation, frame ID or digest returns `target_changed`. A disconnected or revoked source returns `unavailable`; a stopped owner returns `stopped`. Only the matching current request is consumed.

