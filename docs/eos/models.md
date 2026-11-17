# EOS interfaces

<!-- doc-navigation:start -->
<details>
<summary>Folder: eos</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Documents in this folder:
  - [EOS documentation](README.md)
  - [EOS interfaces](interfaces.md)
- Subfolders:
  - [core/](core/README.md)
  - [interfaces/](interfaces/)

</details>
<!-- doc-navigation:end -->

## Table of contents

- [Reading the examples](#external_models--reading-the-examples)
- [Health](#external_models--health)
- [Calls](#external_models--calls)
- [Automatic speech recognition (ASR)](#external_models--automatic-speech-recognition-asr)
- [Object detection](#external_models--object-detection)
- [Human detection](#external_models--human-detection)
- [Human pose](#external_models--human-pose)
- [Voice activity detection (VAD)](#external_models--voice-activity-detection-vad)
- [Text-to-speech synthesis (TTS)](#external_models--text-to-speech-synthesis-tts)
- [Face identification](#external_models--face-identification)
- [General VLM request](#external_models--general-vlm-request)


<a id="external_models"></a>
<a id="external_models--internal-model-interfaces"></a>
## Internal model interfaces

[中文](../../docs_cn/eos/models_cn.md#external_models_cn)

[Interface overview](interfaces.md#readme) · Model runtime

EOS talks to models through private, capability-based interfaces. These interfaces describe what EOS sends and receives without depending on a model vendor, framework, deployment, or application domain. Provider URLs, headers, credentials, and native payload formats remain behind the model adapter and never cross the EOS client-facing gRPC interface.

EOS owns two internal operations:

- `Health(model_id)` returns the latest normalized health record for a configured model.
- `Call(model_id, request)` validates a capability request, calls the configured service, normalizes its response, and returns a structured call status.

Model IDs also form the private dependency seam for advanced hardware primitives. A hardware profile's advanced `[[device.primitive]]` declaration names one exact `model_id`; the model must be configured with the capability required by that primitive and `enabled = true`. EOS refuses startup for a missing, disabled, or wrong-capability dependency and never substitutes another model with the same capability. This binding stays internal and does not change the public primitive interface.

The authoritative request and response schemas live in `contract/schemas/models/`. They are private runtime contracts and are not included in the public EOS contract manifest under `contract/schemas/`.

The configured provider transport may be HTTP, WebSocket, or gRPC. A same-host gRPC provider should use a Unix domain socket; a TCP target is also supported. The private `island.eos.external.models.v1.ModelProviderService` in `contract/proto/external/models/model_provider.proto` exposes unary `Health` and `Call`. Configure it with `transport = "grpc"` and `endpoint = "unix:/path.sock"` or `endpoint = "127.0.0.1:port"`; omit the HTTP `health_endpoint`. `Call` carries the configured provider model ID, capability, and canonical UTF-8 JSON bytes, validated against the same capability request and response schemas used by the other transports. Calls are bounded by deadlines, cancellation, and message-size limits. This buffered interface does not add incremental streaming.

<a id="external_models--reading-the-examples"></a>
### Reading the examples

The examples use descriptive placeholders such as `<base64-encoded JPEG image>`. Replace a placeholder, including its angle brackets, with the corresponding base64 data. Optional fields are shown when they clarify the interface, but a request does not need to include every field.

Shared conventions:

- Times ending in `_s` are seconds.
- Confidence and probability values range from `0.0` to `1.0`.
- Image coordinates are pixels in the source image, with the origin at its top-left corner.
- A bounding box is `[x1, y1, x2, y2]`: top-left followed by bottom-right.
- Count fields must equal the length of the corresponding array.
- Unknown fields are rejected rather than forwarded to a provider.

<a id="external_models--health"></a>
### Health

An enabled HTTP model may provide a health endpoint. EOS probes configured health endpoints on a dedicated worker and normalizes the result. A model starts in the local `checking` state and can recover after any later valid probe. Health is observational: an unhealthy probe does not suppress an inference attempt. Without a health endpoint, health remains unavailable and EOS emits one configuration warning rather than warning on every call.

A health response identifies the service, the configured model, and the capabilities it currently provides:

```json
{
  "interface_version": "1.0",
  "status": "ready",
  "service": {
    "name": "inference-service",
    "version": "1.4.0"
  },
  "model": {
    "id": "primary-model",
    "version": "2.1.0"
  },
  "capabilities": [
    "vision.human_pose"
  ]
}
```

Provider status is normalized to `ready`, `degraded`, or `unavailable`. Identity or capability mismatches, malformed responses, and timeouts produce `unavailable` health without exposing provider details.

<a id="external_models--calls"></a>
### Calls

Each configured model declares one capability and therefore one request and response schema. EOS validates both sides of every call and converts any provider-specific representation in its adapter. This keeps callers independent of the selected model implementation.

ASR, TTS, and VAD requests use the optional `stream` boolean to make delivery mode explicit. Omitting it is equivalent to `false`. The current `Call` API is buffered and accepts only `false`; `true` fails explicitly instead of being silently changed to a non-streaming request. Incremental delivery requires a separate streaming call API in addition to a streaming transport adapter.

For example, automatic speech recognition receives encoded audio and returns a normalized transcript. EOS may publish the derived `asr.snapshot` northbound, but the source audio remains private.

<a id="external_models--automatic-speech-recognition-asr"></a>
### Automatic speech recognition (ASR)

ASR converts audio into text. Only `input.data_b64` is required.


```json
{
  "input": {
    "data_b64": "<base64-encoded audio>"
  }
}
```

Optional request fields are `input.filename`, `input.media_type`, `language`, `prompt`, `temperature`, and `stream`.


```json
{
  "text": "Please open the door.",
  "language": "en",
  "duration_s": 1.84,
  "segments": [
    {
      "start_s": 0.0,
      "end_s": 1.84,
      "text": "Please open the door.",
      "confidence": 0.97
    }
  ]
}
```

Only `text` is required in the response. `language`, `duration_s`, `segments`, and each segment's `confidence` are optional.

<a id="external_models--object-detection"></a>
### Object detection

Object detection locates and classifies objects in an image. `conf` sets the minimum confidence and `iou` sets the intersection-over-union threshold; both are optional. Only `input.data_b64` is required; `input.filename` and `input.media_type` are optional transport hints.


```json
{
  "input": {
    "data_b64": "<base64-encoded JPEG image>"
  },
  "conf": 0.4,
  "iou": 0.5
}
```


```json
{
  "objects": [
    {
      "label": "person",
      "class_id": 0,
      "confidence": 0.96,
      "bbox": [
        120,
        80,
        420,
        700
      ]
    }
  ],
  "object_count": 1
}
```

Every detection has a `label`, `confidence`, and `bbox`. `class_id` is optional because a provider may not use a numeric class catalogue.

<a id="external_models--human-detection"></a>
### Human detection

`vision.human` returns tracked person boxes. Every row requires `track_id`, `bbox`, and `confidence`; `distance_m` is retained when aligned depth provides it. This remains distinct from label-based `vision.object_detection`.

A valid response with one tracked person is:

```json
{
  "persons": [
    {
      "track_id": 7,
      "bbox": [210, 80, 510, 700],
      "confidence": 0.94,
      "distance_m": 2.1
    }
  ],
  "person_count": 1
}
```

<a id="external_models--human-pose"></a>
### Human pose

`vision.human_pose` accepts an image and optional tracked `persons` rows, then returns complete COCO-17 joints plus current posture and behavior fields.


```json
{
  "input": {
    "data_b64": "<base64-encoded JPEG image>"
  }
}
```

`persons`, `conf`, and `kpt_conf` are optional. `depth_input`, `depth_scale`, and `depth_intrinsics` are also optional. When provided, `depth_input` is a lossless single-channel depth image aligned pixel-for-pixel with `input`; multiplying a raw depth value by `depth_scale` converts it to metres. `depth_intrinsics` contains `width`, `height`, `fx`, `fy`, `ppx`, `ppy`, `distortion_model`, and five `coeffs` for that final aligned pixel coordinate system. Its dimensions must match both decoded images. A multipart adapter sends the aligned depth image as `depth_file` alongside the RGB `file`.

The RTMPose WebSocket adapter maps these canonical fields to `depth_b64`, `depth_scale`, and `depth_intrinsics`. If depth is unavailable or the request would exceed its configured byte limit, EOS omits all three and retains the RGB-only pose path.

Camera transport details are resolved before this private model boundary. EOS converts the camera's aligned color and depth feeds into backend-neutral camera telemetry before creating this request, so middleware-specific message types do not enter the models runtime.


```json
{
  "persons": [
    {
      "track_id": 7,
      "bbox": [
        210,
        80,
        510,
        700
      ],
      "confidence": 0.94,
      "distance_m": 2.1,
      "keypoints": [
        [
          310,
          120
        ],
        [
          300,
          110
        ],
        [
          320,
          110
        ],
        [
          290,
          115
        ],
        [
          330,
          115
        ],
        [
          275,
          210
        ],
        [
          345,
          210
        ],
        [
          250,
          320
        ],
        [
          370,
          320
        ],
        [
          235,
          420
        ],
        [
          385,
          420
        ],
        [
          285,
          410
        ],
        [
          335,
          410
        ],
        [
          280,
          540
        ],
        [
          340,
          540
        ],
        [
          275,
          675
        ],
        [
          345,
          675
        ]
      ],
      "keypoint_scores": [
        0.95,
        0.94,
        0.94,
        0.91,
        0.92,
        0.96,
        0.96,
        0.93,
        0.93,
        0.9,
        0.91,
        0.97,
        0.97,
        0.95,
        0.95,
        0.92,
        0.92
      ],
      "pose": "stand",
      "pose_confidence": 0.93,
      "action": "none",
      "action_confidence": 0.98
    }
  ],
  "person_count": 1
}
```


<a id="external_models--voice-activity-detection-vad"></a>
### Voice activity detection (VAD)

VAD identifies speech regions in audio. Its input format is deliberately fixed: mono, 16 kHz, 16-bit linear PCM in a RIFF/WAVE container. Only `input.data_b64` is required.


```json
{
  "input": {
    "data_b64": "<base64-encoded PCM WAV audio>"
  }
}
```

Optional request fields are `input.filename`, `input.media_type`, `input.sample_rate_hz`, `input.channels`, `threshold`, and `stream`. When present, the format fields must describe the fixed format above. Adapters default them to `audio.wav`, `audio/wav`, 16000 Hz, and one channel.


```json
{
  "speech": true,
  "probability": 0.98,
  "segments": [
    {
      "start_s": 0.12,
      "end_s": 1.72,
      "confidence": 0.98
    }
  ]
}
```

Segments are ordered, do not overlap, and use an inclusive start and exclusive end. `speech` is `true` exactly when `segments` is non-empty. `probability` is the highest segment confidence, or `0.0` when no speech is detected.

<a id="external_models--text-to-speech-synthesis-tts"></a>
### Text-to-speech synthesis (TTS)

TTS converts text into WAV audio. Only `text` is required.


```json
{
  "text": "The update is complete."
}
```

Optional request fields are `language`, `voice`, `instruction`, `temperature`, `top_p`, `reference_audio_b64`, `reference_audio_media_type`, `prompt_text`, and `stream`. Provider-specific names belong in the external adapter and are not part of this interface.


```json
{
  "data_b64": "<base64-encoded WAV audio>",
  "media_type": "audio/wav",
  "format": "wav"
}
```

<a id="external_models--face-identification"></a>
### Face identification


<a id="external_models--recognize-faces"></a>
#### Recognize faces

`recognize` returns every detected face. `return_image` optionally requests an annotated copy of the input image.

Request:

```json
{
  "operation": "recognize",
  "input": {
    "data_b64": "<base64-encoded JPEG image>"
  },
  "return_image": true
}
```

Response:

```json
{
  "status": "success",
  "operation": "recognize",
  "data": {
    "faces": [
      {
        "identity": "Person A",
        "known": true,
        "similarity": 0.97,
        "bbox": [
          10,
          20,
          110,
          160
        ],
        "direction_px": -4.0
      }
    ],
    "face_count": 1,
    "annotated_image": "<base64-encoded annotated image>"
  },
  "message": "Recognition completed."
}
```

`direction_px` is the horizontal displacement of the face center from the image center; a negative value means left. `direction_px`, `annotated_image`, and `message` are optional.

The `message` field is optional. Of the non-recognition operations, only the internal `Runtime::RegisterFace` path is used; it validates and drives ArcFace's registration protocol exclusively for the `face.register` primitive; typed inference callers cannot select that operation.

<a id="external_models--arcface-request-timing"></a>
#### ArcFace request timing

The camera runtime waits for human detection and sends recognition frames only while a tracked person is present. It schedules requests at 250 ms intervals while identity is unresolved, then at least one second apart after matching a registered identity. A longer configured minimum interval still applies. The worker retains only the latest waiting frame, logs queue, inference, and frame age, and discards results whose source frame is more than three seconds old.


<a id="external_models--general-vlm-request"></a>
### General VLM request

The `vla-gemma4` model serves the private `vision.vlm` capability and targeted posture classification through one configured server and health probe. General requests accept a text conversation in `messages` and one optional JPEG image in `input`. A text-only call works without an image. When an image is present, the `gemma4-unified-v1` adapter attaches it to the latest user message; a user message is therefore required for image calls. The caller supplies the image explicitly. The model runtime does not capture a frame or read a file on its own.

```json
{
  "messages": [
    {
      "role": "user",
      "content": "What is in front of me?"
    }
  ],
  "input": {
    "data_b64": "<base64-encoded JPEG image>",
    "media_type": "image/jpeg"
  },
  "max_tokens": 128,
  "stream": false
}
```

`input`, `temperature`, `max_tokens`, `reasoning`, and `stream` are optional. `reasoning` is a boolean that enables Gemma thinking when true; it defaults to false. Targeted posture classification always disables thinking. The bounded response contains the generated text. EOS validates both sides before returning it.

Built-in skills can call `ReadVlmImage` to obtain one fresh frame from an admitted camera device or one bounded JPEG from an `asset://` URI under the EOS state `assets` directory. The returned canonical `input` goes into an ordinary VLM inference request; it does not change the model contract.

A successful VLM response is:

```json
{
  "text": "A chair is in front of the camera."
}
```
