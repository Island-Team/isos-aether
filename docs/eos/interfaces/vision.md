# EOS v2 Vision interface

> Historical contract. This interface is not implemented by the current button-to-mock-arm graph. See [arm control](arm-control.md) for the active data and command path and [owner lifecycle](owner-health.md) for setup, Run, Status, and Stop.


[中文](../../../docs_cn/eos/interfaces/vision_cn.md) · [Package node IDL](node-contributions.md) · [Owner first-use guide](../design/runtime/vision-provenance.md)


## Data and rate contract

`vision_frame` uses codec `vision_frame_jpeg_v1`, media type `image/jpeg`, and an 8 MiB JPEG payload limit. The logical `VisionFrame` fields are `gate_path` (string, canonical gate path), `sequence` (optional unsigned integer, source frame count), `timestamp_unix_ms` (optional signed integer, Unix milliseconds), `width_px` and `height_px` (positive integer, pixels), `source_rate_hz` (finite positive number, hertz), and `jpeg` (JPEG bytes). The binary WebSocket payload carries metadata separately from JPEG bytes.


## Read-only browser API

`GET /api/eos/v2/vision/gates` returns `{"revision":"<graph-hash>","gates":[{"path":"<gate-path>","label":"Front camera","output_port":"preview","source_path":"<camera-path>","source_rate_hz":30.0}]}`. `revision` is the lowercase SHA256 hex digest of the loaded graph bytes. The list includes active gates only. `path` and `source_path` are canonical graph paths, `label` is a display string, `output_port` is `preview`, and `source_rate_hz` is hertz.

Connect to `WS /api/eos/v2/vision/ws` and send one masked text frame first:

```json
{
  "op": "subscribe",
  "gate_path": "isos/eos_interface/gate/front",
  "revision": "<graph-hash>",
  "target_fps": 30,
  "max_width_px": 1280,
  "jpeg_quality": 80
}
```

`op`, `gate_path`, and `revision` are required strings. The three numeric options are optional and use the bounds above. The server sends `{"type":"subscribed"}` or `{"type":"error","message":"..."}` as text. Each binary frame is `u32` little-endian metadata JSON byte length, then that many UTF-8 metadata JSON bytes, then JPEG bytes. Metadata includes `gate_path`, `width_px`, `height_px`, and `source_rate_hz`; `sequence` and `timestamp_unix_ms` appear when present. The subscription is rejected if the graph revision changed or the gate is inactive. One WebSocket connection carries one subscription. Frames are bounded to 8 MiB JPEG plus 4 KiB metadata. Browser origin restrictions from the HTTP bridge still apply.

EOS captures and encodes in its runtime; the HTTP bridge only relays validated read-only frames. No browser endpoint accepts raw camera control or actuator commands.

## Q01 gate provenance extension — implementation candidate



The captured source receives `source_frame_id`, a nonblank EOS-issued opaque ID unique within `(runtime_stream_id, source_path)`, even when the camera provides no sequence or capture timestamp. EOS freezes the canonical `source_path`, optional source `sequence` and `timestamp_unix_ms`, dimensions, and source bytes with that ID before gate delivery. Camera-reported sequence and time remain diagnostic and never substitute for the EOS ID or the delivery identity. The gate snapshot holds the source and delivery identity together; the HTTP bridge must not assemble metadata from a later status read.

Each binary WebSocket frame retains the existing `u32` little-endian metadata-length prefix and 8 MiB JPEG bound. The metadata JSON must be at most **4096 UTF-8 bytes**, measured before the JPEG; length zero, length above 4096, truncated JSON, non-object JSON, or an absent JPEG is invalid. The extension adds the following required fields to the same metadata object. Existing fields keep their meanings.

| Field | Type / unit | Purpose, absence and validation |
|---|---|---|
| `projection_revision` | string / — | Required 64 lowercase hex digits of the loaded EOS projection; equals the existing Vision catalog/subscription `revision`. |
| `runtime_stream_id` | UUID string / — | Required EOS-start ID; never reused after restart. |
| `source_path` | canonical graph path / — | Required camera source of the delivered gate, matched to the selected graph edge. |
| `source_frame_id` | nonblank opaque string / — | Required EOS capture ID; max 128 UTF-8 bytes; unique for this source and stream. |
| `gate_generation` | unsigned 64-bit integer / deliveries | Required, positive, strictly increasing per gate and stream; no wrap or reuse. |
| `render` | object / pixels and quality 1–100 | Required `{ "max_width_px": 1280, "jpeg_quality": 80 }`, the validated subscription parameters used for this rendition. |
| `jpeg_sha256` | string / — | Required lowercase SHA-256 hex of the **exact JPEG bytes after metadata**; computed by EOS after encoding, before the immutable frame is queued. |

For example, the metadata portion of one frame is:

```json
{
  "gate_path": "isos/eos_interface/gate/front",
  "graph_revision": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
  "projection_revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "runtime_stream_id": "8b4e9b70-5481-4d88-9bd6-4f07cb466ae4",
  "source_path": "isos/eos_hardware/camera/front",
  "source_frame_id": "cam-41",
  "gate_generation": 7,
  "width_px": 1280,
  "height_px": 720,
  "source_rate_hz": 30.0,
  "render": {
    "max_width_px": 1280,
    "jpeg_quality": 80
  },
  "jpeg_sha256": "c8f1b57ba3b319e7c5ddce3dac7f09c729d8d4bce2259c4f03d68cdbb7f69f0d"
}
```

The illustrative digest above is a shape example, not a digest of an omitted JPEG. A receiver verifies `jpeg_sha256` over the received bytes. Different subscriptions may render the same delivery at different widths or qualities, so delivery identity can match while JPEG digests differ. A Q01 replay that claims byte equality must select identical render parameters and compare each verified digest and binary payload length; otherwise it compares the delivery identity and reports both rendition digests separately. For Q01, EOS encodes the fixed 1280-pixel/quality-80 rendition once from the committed gate snapshot, retains those immutable bytes with the latest slot, and serves those same bytes to the browser and owner listeners when both request that render. It computes the digest once over that JPEG; replacing or clearing the slot drops the bytes. Other render choices have separate bytes and digests. A rendition never changes the delivery identity or gate cap counters.




**Design-round verification, 2026-11-02:** `cargo fmt --all` and `cargo fmt --all -- --check` passed. Changed-page local links and fenced JSON examples passed a read-only parser check. No Vision runtime behavior or selected Q01 replay was implemented or claimed in this documentation branch. Formatter output is in local `data/test/20261102-q01-camera-design/fmt.log`.

### Frozen 0.x owner transport and handshake

The existing `aether-http-bridge` keeps `--listen <loopback-host:port>` and the public browser `GET /api/eos/v2/vision/gates` and `WS /api/eos/v2/vision/ws` on its TCP listener. A selected Q01 run additionally starts that same bridge process with `--owner-vision-socket <absolute-path>`, its existing `--socket <EOS-private-runtime-socket>`, and `--run-id <IsOS-run-id>`. The new owner listener is a Unix socket named `eos-vision-owner.sock` in the IsOS-selected private Vision endpoint directory, which can be shorter than the control-grant run directory. The endpoint directory has mode 0700 and matching UID; the socket has mode 0600. Its mode-0600 verifier is in that same directory. IsOS checks both Vision socket paths against the Unix pathname limit before launch. The listener accepts only `GET /api/eos/v2/vision/owner-ws` with `Host: localhost`, no `Origin` or `Sec-Fetch-*`, no query credentials, and `Authorization: Bearer <64-lowercase-hex>`. It never routes to public gates, control, result or browser Vision paths. The bridge obtains the verifier path only from `AETHER_BRIDGE_VISION_VERIFIER_FILE`, not a command argument; it fails startup if the verifier is absent or unsafe. The selected EOS runtime must advertise `vision-provenance-v1` and the selected bridge `vision-owner-uds-v1` in `--capabilities`; the focused EOS implementation advertises these tokens; selected IsOS must still pin and verify the exact candidate bytes.




**Contract repair verification, 2026-11-02:** `python3 tests/fixtures/q01_vision_provenance/check_cases.py` validated 12 frozen cases; `cargo fmt --all -- --check` and `git diff --check` passed. Changed-page links and JSON examples passed the read-only check. Output is in ignored `data/test/20261102-q01-contract-repair/`. This remains design evidence only.
