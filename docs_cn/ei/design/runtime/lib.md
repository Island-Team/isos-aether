# EI 配置与日志

[English](../../../../docs/ei/design/runtime/lib.md) · [架构](../architecture.md) · [启动](../../start/quick-start_cn.md)

EI 在构建运行模块前从 `--etc` 或 `EI_ETC` 解析配置根目录；缺失时启动失败。`models.toml` 定义模型绑定和生成限制；`modes.toml` 选择 `profiles/` 下的配置；profile 定义 EOS 后端与操作员原语暴露。`prompts.toml` 是模型提示来源。`missions.toml` 可以定义规范命名任务及受保护计划的严格白名单；`[decision] load_mission_templates` 默认 `false`。`safety/gate.toml` 定义确定性出口界限。产品级 `island.toml` 选择共享资源根目录和组件启动路径。

dotenv 读取器把缺失文件当成空覆盖层，忽略注释、空行和格式错误的赋值，并保留已有进程变量。它只报告载入的变量名，不求值 shell 语法。秘密保留在环境查询中，不写入结构化配置输出。

## 日志与状态

一个经过清理的事件写入本地核心 JSONL 文件；独立运行时也可写入配置的控制台。未知日志键会被拒绝。在任何 sink 接收前，递归清理会遮蔽含秘密的键及 URL，排除原语参数、任意结果、提示、转录、提供商负载、图像和音频，并限制嵌套数据。关联和生命周期上下文仅在一次请求或执行期间有效。

源码检出中的单一核心文件为 `data/ei/log/<deployment>/ei.jsonl`（独立运行使用 `local`）。每行 JSON 包含时间、级别、来源、消息，以及可选事件码、run ID、关联 ID 和清理后的字段。来源使用 EI 节点 ID；系统代码使用 `ei.owner_service.py@aether-v0.2.1`。IsOS 直接读取该文件。本地文件默认记录 `DEBUG` 及以上；控制台通常从 `WARNING` 开始，在启动和关闭时显示 `INFO`。`--log-level` 与 `[logging]` 可调整详细程度。

模型客户端接收文本和支持的图像，将图像转为带 MIME 的数据 URL，并发送 chat-completions 请求。非流式回复必须有可用 choice；流式回复消费有效 SSE 增量并关闭响应。HTTP、URL、超时、解码、畸形 JSON、空 choice 与流中断失败都成为带有界详情的 `ModelClientError`。缺少用量字段不会改变返回文本。
