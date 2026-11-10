# Q01 EOS Vision 所有者首次使用

[English](../../../../docs/eos/design/runtime/vision-provenance.md) · [Vision 接口](../../interfaces/vision_cn.md) · Q01 计划


1. 在 IsOS `[vision_evidence].gate_path` 选定一个活动 `isos.eos_interface.camera_gate@v0.2`。来源须是已注册的 `isos.eos_hardware.camera@v0.2`，并有已准入的 EOS 本地 `frame:camera_frame` 路由。验证并应用保存图。图作者不能写 `service_bridge` 边。
2. 用 `aether-runtime --capabilities` 与 `aether-http-bridge --capabilities` 检查选定二进制文件，须分别包含 `vision-provenance-v1` 和 `vision-owner-uds-v1`。IsOS 在替换运行中的部署前重新校验可执行文件和软件包的精确字节。
3. 通过 IsOS 启动托管 EOS。在私有运行时 socket 上发送 `{"op":"vision_attest","gate_path":"<选定 gate>"}`，可得到 IsOS 保存图 `active_revision`、EOS `projection_revision`、部署 `run_id`、新的 `runtime_stream_id`、来源和 gate 路径，以及首帧前的 `gate_generation:0`。gate 不健康或 EOS 停止时返回不可用。独立运行的 EOS 不能声称托管来源。
6. 用 `route_status` 查看 `cap_wait_count`、`replaced_count` 和 `gap_count`。通过 IsOS 停止，并确认原有执行器停止先于 MuJoCo 终止。EOS 在停止动作前封闭 Vision 发布；bridge 停止时关闭所有者订阅并删除运行 socket。每次有界轮询都会重新读取私有 verifier 和就绪证明，因此授权撤销后，已有订阅在发送下一帧 JPEG 前结束。新的 EOS 子进程更换 `runtime_stream_id`；新的 IsOS 运行还会更换 Vision 密钥。

局部检查可运行 `cargo test -p aether-http-bridge --bin aether-http-bridge`、`cargo test -p aether-runtime --test process managed_gate_attests_saved_and_projection_revisions_and_exact_jpeg` 与 `cargo fmt --all -- --check`。这些测试覆盖 EOS 边界，本身不代表跨所有者 Q01 回放已合格。所有者绑定实现后，完整选定版本回放及篡改用例记录在 Q01 验证证据中。

## 实现证据

2026-11-02，此 EOS 候选在 Linux 7.0.0-38-generic x86-64、Rust 工作区版本 `0.2.0` 上运行。完整本地输出保存在已忽略的 `data/test/20261102-q01-vision-eos/`。在 EOS 工作树根目录执行 `python3 tests/fixtures/q01_vision_provenance/check_cases.py`、`python3 scripts/check_doc_links.py --all`、`cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`RUST_TEST_THREADS=1 cargo test --workspace --no-fail-fast` 及 `cargo build --workspace`。局部 `cargo test -p aether-http-bridge` 和 `cargo test -p aether-runtime --test process` 覆盖所有者密钥与版本拒绝、首帧前证明、托管 gate 摘要和浏览器兼容性。默认并行工作区运行中，五个既有 `backend_child` 进程组用例相互干扰；完整串行工作区回放通过。这只是 EOS 所有者证据。在合并此主题前，仍需跨所有者选定版本 Q01 验证及独立实现审查。

**独立审查修复，2026-11-02：** runtime 现在以来源健康 epoch 隔离已准备的交付；所有者 bridge 每次有界轮询都重新检查选定 verifier 与 `ready:true` 证明。私有 IPC 的 `changed:true` generation 若重复或倒退，会在发送下一帧 JPEG 前被拒绝。确定性反例测试覆盖准备与提交之间的健康失效、EOS 保持就绪时删除已有所有者订阅的 verifier、`ready:false`，以及相同／更低的 generation。完整输出位于已忽略的 `data/test/20261102-q01-vision-eos-review-fixes/`。可复现命令为 `RUST_TEST_THREADS=1 cargo test -p aether-runtime --bin aether-runtime vision::tests::invalidation_between_preparation_and_commit_rejects_old_frame`、`RUST_TEST_THREADS=1 cargo test -p aether-http-bridge --bin aether-http-bridge vision_ws::tests::owner_socket`、`cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`RUST_TEST_THREADS=1 cargo test --workspace --no-fail-fast`、`cargo build --workspace`、`python3 tests/fixtures/q01_vision_provenance/check_cases.py` 及 `python3 scripts/check_doc_links.py --all`；全部通过。MuJoCo 现场测试仍须显式启用，在工作区测试中被忽略。这仍是 EOS 单方证据，待独立复审及跨所有者 Q01 验证。

