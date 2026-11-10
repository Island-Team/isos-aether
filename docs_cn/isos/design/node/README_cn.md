# 已注册 IsOS 类型目录

**核心**拥有执行，**层**组织主要领域，**组**命名类型族，**节点**是配置后的实例。[注册类型](../../interfaces/node/type_cn.md)固定位置、端口、能力、设置和生命周期；保存实例选择值。

- [受管组件](process/managed-component_cn.md)描述 IsOS 拥有的进程类型族：已验证软件包入口作为 IsOS 直接子进程监管，并检查归属方就绪及停止。
- [视觉面板消费者](ui/vision-panel_cn.md)描述 UI 所属节点如何引用输出端口并选择渲染方法。IsosUI 拥有布局和订阅；IsOS 不渲染负载。

软件包贡献类型使用相同的[通用节点契约](../../interfaces/node/README_cn.md)及[软件包验证](../../interfaces/packages/README_cn.md)。本目录解释角色；字段级权威仍归这些契约。

## 所属 core 选择器实现证据

首批所属 core 选择器通过 setup `options` 投影 EI 规划器模型绑定，以及 EOS 适配器、MuJoCo 设备和模拟器节点选项。IsOS 校验所属 core 的响应后再交给 UI。EOS 和 EI 在 `validate` 与 `render` 时重新检查实际生效的选择，包括 EI 默认值。缺少所需的所属 core 选项组或生效选项不可用时，UI 阻止保存和运行。草稿所属 core 类型目录与选项预览运行同一个本地选定的 setup 适配器，因此处于旧世代的持久化归属方不会阻止草稿编辑。预览只读，不能替代 Apply 准入；Apply 仍向实时归属方重新验证。

在仓库根目录运行 `cargo fmt --all && cargo fmt --all -- --check`、`cargo test -p isos --bin isos --offline`、`cargo test -p aether-runtime --bin aether-profile --offline`、`cargo clippy -p isos -p aether-runtime --bins --offline -- -D warnings`、`cargo build --workspace --bins --offline`、`ei/.venv/bin/python -m pytest ei/tests/ei/test_config_cli.py -q`、`cd ui && npm run typecheck && npm test -- --run && npm run build`、`python3 scripts/check_doc_links.py --all`，以及 `ei/.venv/bin/python -m unittest discover -s crates/isos/tests -p unified_owner_replay_cli.py -v`。2026-11-09 的运行全部通过：349 项 IsOS 测试、16 项 EOS profile 测试、16 项 EI setup 测试、215 项 UI 测试、双语链接零错误，以及四项真实所属 core 回放。完整输出保存在被忽略的 `data/test/20261109-owner-options-*.log`。

[English](../../../../docs/isos/design/node/README.md)

## 严格端口契约验证记录

运行 `cargo fmt --all -- --check`、`ISOS_AETHER_BIN_DIR="$PWD/target/debug" cargo test -p isos`、`cargo test -p aether-node-profile`、`cd ui && npm run typecheck && npm test && npm run build`、`python3 scripts/check_doc_links.py --all` 和 `python3 crates/isos/tools/check_isos_docs.py`。2026-11-09 的 Rust 与 UI 测试、两种 UI 构建、格式检查和双语文档审计均通过。完整命令输出保存在被忽略的 `data/test/20261109-strict-node-ports/`。非致命 Clippy 检查通过，但现有的 IsOS 未使用代码警告阻碍了 `-D warnings` 检查。

## 相连按钮演示验证记录

首个连通按钮切片包含八个节点和七条类型化连接。当前演示图还包含下文所述的八节点观测路径。IsOS 准入带运行栅栏的按钮事件，EI 将其依次传过输入、认知和安全节点，EOS 将其转换为有界六关节增量并传给模拟机械臂。内置类型目录替换了旧的仅健康演示 ID。在仓库根目录构建 `isos` 和 `aether-runtime` 二进制，然后运行 `ISOS_AETHER_BIN_DIR="$PWD/target/debug" RUST_TEST_THREADS=1 cargo test --workspace --all-targets`、`cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets`、`cd ui && npm run typecheck && npm test && npm run build`、`python3 scripts/check_doc_links.py --all` 和 `python3 crates/isos/tools/check_isos_docs.py`。

2026-11-09 的隔离回放执行了初始化 → 验证 → Apply → Run → 就绪 → 控制 → 日志 → 已确认 Stop。第一次点击返回 `[0.05,-0.04,0.03,-0.02,0.01,-0.01]` 弧度；重试同一事件 ID 不会再次移动；新 ID 累积增量；旧修订返回 409。完整命令输出保存在被忽略的 `data/test/20261109-arm-button-integration/`。

## 模拟机械臂监控回放记录

16 节点起始图新增八个只读返回节点，并绑定控制链使用的同一模拟机械臂。组合回放运行了 `cargo fmt --all -- --check`、`ISOS_AETHER_BIN_DIR="$PWD/target/debug" RUST_TEST_THREADS=1 cargo test --workspace --all-targets`、`ei/.venv/bin/python -m pytest ei/tests -q`、`cd ui && npm run typecheck && npm test && npm run build` 以及双语链接检查。隔离环境的 `config init` → 验证 → Apply → Run → 就绪 → 按钮激活 → 监控读取 → 日志 → 已确认 Stop 回放在移动前返回六个零位，移动后返回准确的六关节位置，Stop 后监控值清空。随后直接替换现用保存图，浏览器点击后在节点和监控面板均显示相同位置。完整输出与载荷位于被忽略的 `data/test/20261109-arm-monitor-integration/`。
