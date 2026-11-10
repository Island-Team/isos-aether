# 首次使用

[English](../../../docs/isos/start/quick-start.md) · [节点契约](../interfaces/node/README_cn.md)

起始图包含三个核心和 16 个相连节点。IsOS 按钮的激活事件依次经过 EI 输入、认知、安全，以及 EOS 入口、演示技能、调度器和模拟六关节机械臂。独立的只读路径将同一机械臂的状态经 EOS 和 EI 返回 IsOS 监控节点。

1. 构建 IsOS、`aether-profile` 和 `aether-runtime`，并按 `ei/pyproject.toml` 安装 EI 程序。将 `ISOS_AETHER_BIN_DIR` 与 `ISOS_EI_BIN_DIR` 指向选定的可执行文件目录。
2. 运行 `isos config init`。已保存的演示图位于所选 IsOS 主目录的 `config/graph-pool/demo.nodes.json`；部署只保存图库名称绑定。
3. 验证并 Apply 该图库文件。Apply 的 `--revision` 必须是其当前字节的精确 SHA-256；`config apply --file` 只接受图库中的文件。
4. 用 `isos web` 打开工作台（或用 `isos serve` 在前台启动服务器）。这会启动 IsOS 和空闲的归属方目录服务，但不会启动 EI 或 EOS 部署工作进程。拖动节点只修改浏览器保存的画布布局。准备执行时，在工作台使用 **保存并运行**，或明确运行 `isos start` / `isos run`。随后查看 `isos status --json`：EI 应为 `running`，EOS 应为 `ready`，EOS 模拟适配器应为 `ready`。
5. 在工作台点击按钮节点上的 **Activate**。按钮先显示已接受的事件 ID，然后单独读取关联结果；每个获准且成功的不同激活事件使模拟机械臂按固定的小关节增量移动。检查最终位置、独立的机械臂监控快照和归属方日志。然后运行 `isos deployment stop` 停止两个核心。`isos stop` 只停止 IsOS 管理器，独立的核心服务仍运行。

修改选定的核心二进制后，先重新 Apply 图，再 Run，以更新经过验证的启动输入摘要。IsOS 退出后 EI 和 EOS 可继续运行；重启 IsOS 并读取状态即可重新连接。某个核心失败时会单独报告，健康核心保持运行。参见[运行时设计与重放](../design/runtime/owner-health_cn.md)及[图 API](../interfaces/http/graph_cn.md)。

仅编辑节点图而不运行部署时，请使用 `isos web`。`isos run` 是会明确启动两个部署工作进程的前台命令；结束其终端会同时结束该工作台服务器。独立启动的 `isos web` 服务器会在编辑节点图期间保持可用。
