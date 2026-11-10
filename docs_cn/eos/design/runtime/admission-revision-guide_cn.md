# 受治理执行器准入：开发与验证

[English](../../../../docs/eos/design/runtime/admission-revision-guide.md) · [所有者契约](../../interfaces/grpc_cn.md#r03-执行器受治理准入目标契约) · [设计](admission-revision_cn.md) · [用例](../../../../tests/fixtures/r03_admission_revision/README_cn.md)

IsOS 为选定的图修订签发运行范围授权。EOS 在 Dispatcher 释放执行器命令前验证授权、活动绑定、健康、截止时间和调用方身份。属于其他运行或修订的请求会被拒绝。journal 为每个 operation ID 保存一份结果；响应丢失或结果不确定都不允许自动重新执行物理动作。

## 首次使用重放

1. 构建 Rust 工作区，并使用包含独立的 `isos`、`ei`、`eos` core，节点类型为 `<core>.<layer>.<group>.<node>@v0.2` 的已验证图启动 IsOS。
2. 由 IsOS 启动 EOS，确认受管理授权和活动图修订只交给选定子进程。
3. 通过公开 EOS 接口提交有效的底盘或机械臂命令。用同一个 operation ID 读取直到终态；确认只有一条 journal 记录和一次适配器释放。
4. 用无效授权、调用方、目标、修订、负载及过期截止时间重试。每个请求必须在适配器释放前失败，也不得泄露其他调用方的结果。
5. 在命令待处理期间测试取消、进程故障、journal 故障和 Stop。验证释放围栏、持久结果分类，以及[模拟器停止契约](../../interfaces/simulator-stop_cn.md)规定的物理停止顺序。

运行 `cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`、串行工作区测试、[用例检查](../../../../tests/fixtures/r03_admission_revision/README_cn.md)和 `python3 scripts/check_doc_links.py --all`。把完整输出保存在 `data/test/<run-id>/`，并记录准确修订、命令、环境和用例到测试的映射。
