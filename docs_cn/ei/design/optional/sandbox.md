# 可选本地媒体沙盒

[English](../../../../docs/ei/design/optional/sandbox.md) · [架构](../architecture.md)

`[launch] sandbox = true` 启用本地媒体 playground；后端选择和故障切换属于启动配置，不是模型动作。它作为 EOS 兼容后端运行，不新增 Agent 本地执行命令，也不改变 `gateway.proto`。设备诊断不赋予执行器权限。

## 设备与发现

Linux 自动选择优先使用 PipeWire 音频，然后退到 PulseAudio/ALSA；相机采用 GStreamer `autovideosrc`。每设备 `backend` 和 `device` 可覆盖自动选择。配置限制录音时长与 PCM 大小、相机尺寸/JPEG 质量/帧数/速率、扬声器 WAV 字节和时长；无需 Python 媒体库。

子进程实现现有 `GatewayService`。发现会列出启用的麦克风、相机与扬声器，包括以 `alive=false`、`admitted=false` 标记的不可用硬件；描述包含设备类别配置与运行状态。这些是机身发现元数据，不是可读资源。可用相机通过 `CaptureFrames` 提供有界 JPEG。Playground 不发布语义世界资源，也不提供语音识别、合成或 `speaker.play` 原语。

## WebSocket 媒体协议

每个 JSON 命令含关联的 `id` 和 `op`，接收 `accepted`、`complete` 或 `error` 事件。`status` 返回设备身份、后端选择、可用性与录制/播放状态；`mic.start` 开始录音，`mic.stop` 返回媒体元数据、一个二进制 WAV 帧及完成事件，`mic.cancel` 丢弃本连接录音且不返回二进制帧。`camera.capture` 按受限 `count` 和 `interval_ms` 返回有序元数据与 JPEG 帧。`speaker.start` 占用上传者，后续二进制消息附加 WAV 分块，匹配的 `speaker.finish` 验证并播放。

同一时间仅允许一个麦克风录音及一个扬声器上传。冲突返回 `busy`，畸形或超限媒体返回结构化错误；客户端断连释放其工作。监控心跳还会读取运行状态，设备可用性进入后端账本、Agent 上下文中的后端区段及 TUI 使用的 `/backend` 帧。
