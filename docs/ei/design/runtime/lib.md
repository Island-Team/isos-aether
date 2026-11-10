# EI configuration and logging

[中文](../../../../docs_cn/ei/design/runtime/lib.md) · [Architecture](../architecture.md) · [Start](../../start/quick-start.md)

EI resolves its configuration root from `--etc` or `EI_ETC` before constructing runtime modules. A missing root aborts startup. `models.toml` defines model bindings and generation limits; `modes.toml` selects a profile under `profiles/`; profiles define EOS backends and operator primitive exposure. `prompts.toml` is the model-prompt source. `missions.toml` can define a strict whitelist of canonical named missions and protected plans; `[decision] load_mission_templates` defaults to `false`. `safety/gate.toml` defines deterministic egress bounds. Product `island.toml` selects shared resource roots and component launch paths.

The dotenv reader treats a missing file as an empty overlay, ignores comments, blank lines, and malformed assignments, and preserves existing process variables. It reports only variable names loaded and never evaluates shell syntax. Secrets stay in environment lookup and are absent from structured config output.

## Logging and state

One sanitized event flows to the local core JSONL file and, for standalone runs, the configured console. Unknown logging keys fail validation. Before any sink sees an event, recursive sanitization redacts secret-bearing keys and URLs, excludes primitive arguments, arbitrary results, prompts, transcripts, provider payloads, images, and audio, and bounds nested data. Correlation and lifecycle context is scoped to a request or execution.

In a source checkout the single core file is `data/ei/log/<deployment>/ei.jsonl` (`local` for standalone runs). Each JSON line contains timestamp, level, source, message, and optional event code, run ID, correlation ID, and sanitized fields. The source is an EI node ID or `ei.owner_service.py@aether-v0.2.1` for system code. IsOS reads this file directly. The local file captures `DEBUG` and higher by default; the console normally starts at `WARNING` and shows startup and shutdown at `INFO`. `--log-level` and `[logging]` change verbosity.

The model client accepts text and supported images, converts images to MIME data URLs, and emits chat-completions requests. Non-stream replies require a usable choice; streaming consumes valid SSE deltas and closes the response. HTTP, URL, timeout, decode, malformed JSON, empty-choice, and interrupted-stream failures become `ModelClientError` with bounded details. Missing usage fields never alter returned text.
