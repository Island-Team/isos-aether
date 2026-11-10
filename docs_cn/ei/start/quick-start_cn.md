# 启动与操作 EI

[English](../../../docs/ei/start/quick-start.md) · [架构](../design/architecture.md) · [配置接口](../interfaces/setup.md) · [运行时接口](../interfaces/runtime.md)

EI 通常由 IsOS 在保存图、软件包和所有者配置交换通过验证后启动。受管配置只接收独立的 EI 核心投影。在按钮演示中，EI 所有者将受运行身份约束的激活事件依次传过输入、意图和安全节点，再送往 EOS；受管工作进程提供就绪状态和生命周期。本页说明源码检出的启动及所有者状态定位；线协议细节见接口页。

## 准备

从仓库根目录运行 `./install.sh` 构建并安装产品。仅构建 EI wheel 时运行 `cd ei && ./scripts/platform/dispatch.sh build`。作者编写的 EI TOML 位于根目录 `config/ei/`，工厂默认值位于 `share/ei/defaults/ei/`。安装版本仍使用 `etc/ei/` 与 `etc/island.toml`。通过 `EI_ETC` 或 `ei/bin/ei --etc <path>` 选择其他 EI 配置根目录。`--mode` 为本次调用选择 `config/ei/modes.toml` 中的模式。

API 密钥放在仓库根目录已忽略的 `.env` 或导出的环境变量中。[`.env.example`](../../../.env.example)只列变量名，不含凭据。已有进程变量优先于 `.env`；凭据不应进入 TOML 或日志。

## 启动

托管部署按照 IsOS 的[首次使用指南](../../isos/start/README_cn.md)启动 IsOS，应用已验证图，然后执行 Run。IsOS 选择 EI 程序、向 `ei-config` 发送有界请求并监管运行时。配置健康检查成功不能单独证明运行时或 EOS 已就绪。

在仓库根目录执行 `ei/bin/ei --mode sandbox` 可进行源码模式诊断。相同的源码启动器会解析配置并写入启动、网关和关闭事件。`--managed` 仅供 IsOS 拥有的运行使用，并拒绝内部启动的沙盒。可用选项见 `ei/bin/ei --help` 与 `ei/bin/ei-config --help`。

## 检查、恢复与停止

独立运行的诊断位于 `data/ei/log/local/`；托管运行位于 `data/ei/log/<deployment>/`。所有 EI 诊断统一写入共享机器可读信封的 `ei.jsonl`。IsOS 从本地文件读取日志面板；EI 不通过所有者接口转发日志。`EI_LOG_DIR` 和 `EI_DEPLOYMENT` 可选择私有诊断根目录与部署。有限时本地检查可运行 `timeout 8s ei/bin/ei --mode sandbox`，再运行 `tail -n 30 data/ei/log/local/ei.jsonl`。

若 EOS 发现缺失或不完整，先检查 EOS 健康与 EI 链路诊断，再重试；EI 必须取得完整发现代才能准入命令。模型、召回或可选技能源可以降级，但不能放宽安全约束。托管运行应通过 IsOS 执行 Stop，使所有者记录子进程退出及子系统证据。超时、断连或取消在 EOS 提供终态结果前都不能证明物理结果。备份 `data/ei/state/` 和用户文件；仅在 EI 停止后清理缓存或临时数据。

## 开发检查

在 `ei/` 运行 `uv sync --extra dev --extra vec`，随后执行 `uv run ruff check .`、`uv run pyright`、`PYTHONPATH=src:libexec uv run lint-imports --cache-dir ../data/cache/tools/import-linter-ei`，以及针对性或完整的 `uv run pytest`。修改文档后在仓库根目录运行 `python3 scripts/check_doc_links.py --all`。将完整验证输出保存到已忽略的 `data/test/<run-id>/`。
