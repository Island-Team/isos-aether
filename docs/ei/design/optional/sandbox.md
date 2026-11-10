
[中文](../../../../docs_cn/ei/design/optional/sandbox.md)


## Activation and failover


```toml
[launch]
backend = true
memory = true
sandbox = true
```





## Device configuration


Linux automatic selection prefers PipeWire, then PulseAudio/ALSA for audio; the camera uses GStreamer `autovideosrc`. Per-device `backend` and `device` overrides support hosts where automatic selection is unsuitable. Configuration also bounds recording duration and PCM snapshot size, camera dimensions/JPEG quality/frame count/rate, and speaker WAV bytes/duration. No Python media library is required.

## EOS surface

The child implements the existing `GatewayService` without modifying `gateway.proto`. Discovery advertises every enabled `microphone`, `camera`, and `speaker` device, including unavailable hardware with `alive=false` and `admitted=false`; each descriptor carries the class-specific configuration and operational state. These body values are discovery metadata, not readable resources. Usable cameras provide bounded JPEG capture through `CaptureFrames`; the playground publishes no semantic world resources and provides no speech recognition, synthesis, or `speaker.play` primitive. Its media WebSocket still supports explicit raw WAV capture and playback for local device diagnostics.

The playground is an EOS-compatible backend only. It does not add an Agent-local execution command. Backend selection and failover are launch concerns rather than model actions.

## WebSocket media protocol

Every JSON command contains correlated `id` and `op` fields and receives `accepted`, `complete`, or `error` events:

- `status` returns camera, microphone, and speaker identity, configured backend/device selection, availability, and current capture/playback state.
- `mic.start` begins recording; `mic.stop` returns a media metadata event, followed by one binary WAV frame, then `complete`.
- `mic.cancel` stops and discards the connection's recording, then returns `complete` without a binary frame.
- `camera.capture` accepts bounded `count` and `interval_ms`; each ordered result is a metadata event followed by one binary JPEG frame.
- `speaker.start` claims the uploader; binary messages append WAV chunks and matching `speaker.finish` validates and plays the completed file.

Only one microphone recording and one speaker upload may be active. Conflicts return `busy`, malformed/bounded media returns structured errors, and a client disconnect releases work it owns.

## Boundary


Each monitor heartbeat also reads runtime status. Device availability updates the backend ledger, the agent's always-present backend context section, and the northbound `/backend` frame consumed by the TUI.
