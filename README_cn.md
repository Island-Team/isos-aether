# IslandOS · IsOS

**构建、运行并理解具身智能体。**

[![源码版本](https://img.shields.io/badge/source-0.2.1-175c79)](Cargo.toml) [![发布状态](https://img.shields.io/badge/status-release%20qualification-e8a33d)](docs_cn/isos/start/quick-start_cn.md) [![图格式](https://img.shields.io/badge/graph-0.2.0-467b65)](docs_cn/isos/interfaces/README_cn.md) [![许可证](https://img.shields.io/badge/license-Apache--2.0-465a75)](LICENSE)

[English](README.md) · [快速开始](docs_cn/isos/start/README_cn.md) · [架构](docs_cn/isos/design/architecture_cn.md) · [文档](docs_cn/README_cn.md)

<p align="center"><img src="docs/assets/island-team-hero.png" width="720" alt="IslandOS 连接感知、智能、受控执行与机械臂"></p>

IslandOS 将保存的部署图连接到机器人执行与认知。IsOS 通过一个命令和两种界面，使观测、决策、已准入动作及其结果的路径可供检查。

## 从一个源码仓库开始

产品模式是默认值。本仓库在被 Git 忽略的 `config/isos/launch.toml` 中保存本机选择；这台设备选用 `dev`。无需安装发布版本，从源码构建并运行：

```sh
./install_dev.sh
./isos doctor
./isos web
```

开发模式下，`./isos` 和 `./isos.sh` 使用 `target/debug/isos`、`target/debug/aether-*`、`ei/bin` 和 `ui/dist/local`。修改源码后重新构建相关组件并重启管理器。单次使用产品模式可执行 `ISOS_MODE=product ./isos web`，或在 `config/isos/launch.toml` 的 `[launch]` 下设置 `mode = "product"`。产品模式需要先用 `./install.sh --prefix "$HOME/.local/isos"` 安装发布版本；`install.sh` 始终构建并暂存产品发布版本。安装包与升级见[安装指南](docs_cn/isos/start/quick-start_cn.md)。

| 操作命令 | 作用 |
| --- | --- |
| `isos` | 打开 `config/isosui.toml` 选定的界面（默认 `web`）。 |
| `isos web` · `isos tui` | 打开 Web 或终端客户端；必要时启动 IsOS 服务。关闭客户端不会停止服务。 |
| `isos doctor` | 检查版本、启动模式、管理器状态、接口支持及本地资源。 |
| `isos update` · `isos upgrade [version]` | 列出可用版本而不更改安装，或验证并暂存升级。 |

IsOS 负责 Apply、Run、日志及部署 Stop。只有 EOS 准入执行器命令。[首次使用指南](docs_cn/isos/start/README_cn.md)介绍完整部署流程。在 TUI 中，`1`–`4` 切换视图，`f` 刷新，`r` 和 `s` 需再次按键确认 Run 和 Stop，`q` 关闭客户端。

## 三个核心，一个操作界面

| 核心 | 职责 | 继续阅读 |
| --- | --- | --- |
| **IsOS** | 验证软件包、保存图修订、Apply，并监护所有者与共享资源。 | [架构](docs_cn/isos/design/architecture_cn.md) · [入门](docs_cn/isos/start/README_cn.md) |
| **EOS** | 运行硬件适配器、健康监测、Dispatcher 准入及其直接仿真器子进程。 | [架构](docs_cn/eos/design/architecture_cn.md) · [入门](docs_cn/eos/start/README_cn.md) |
| **EI** | 运行认知阶段及其本地安全门。 | [架构](docs_cn/ei/design/architecture.md) · [指南](docs_cn/ei/README_cn.md) |

React Web 客户端与 TypeScript Ink TUI 均位于 [`ui/`](docs_cn/isosui/README_cn.md)，且都只连接 IsOS API。新部署使用图格式 `0.2.0`、唯一的 `isos` core 和已注册的 `isos.<layer>.<name>@v0.2` 类型。源码、图、线协议与软件包版本相互独立；MuJoCo 是单独验证的 `island/mujoco-sim@0.1.1` 软件包。[所有者契约](docs_cn/island/architecture_cn.md#跨所有者契约)定义各边界。

## 配置

| 用途 | 文件 | 选择与优先级 |
| --- | --- | --- |
| 启动模式 | [`config/isos/launch.example.toml`](config/isos/launch.example.toml) | 默认产品模式；被忽略的 `config/isos/launch.toml` 或 `ISOS_MODE` 可为本机选择开发模式。 |
| 产品安装 | [`config/isos/install.toml`](config/isos/install.toml) | `./install.sh --prefix` 优先于 `ISOS_INSTALL_ROOT`，再到 TOML 前缀。发布仓库和渠道有对应的 `ISOS_RELEASE_*` 覆盖变量。 |
| IsOS 运行时 | [`config/isos/isos.toml`](config/isos/isos.toml) | `ISOS_CONFIG` 选择绝对路径的 TOML 文件。进程环境变量覆盖选定 TOML，后者覆盖 [`isos.example.toml`](config/isos/isos.example.toml)。旧 `active.env` 不再读取。 |
| EI 运行时 | [`config/ei/`](config/ei/) | 部署 TOML 覆盖出厂 TOML；普通设置保存在 TOML，API 密钥和开发用 SSH 值通过根目录 [`.env.example`](.env.example) 模板 作为环境备用来源。 |
| Web 与 TUI | [`config/isosui.toml`](config/isosui.toml) | `ISOSUI_CONFIG` 选择绝对路径文件；`ISOSUI_DEFAULT_INTERFACE`、`ISOSUI_LOCALE` 和客户端时序环境变量覆盖相应字段。 |

已安装启动器默认提供产品内置配置路径，除非显式设置覆盖项。模式、端口、数据位置和迁移见[配置指南](docs_cn/isos/design/app/configuration_cn.md)。

## 开发与验证

`crates/isos/` 包含管理器，`crates/aether-*/` 包含 EOS Rust 运行时，`ei/` 包含 Python 运行时，`ui/` 包含两种客户端。从根目录运行 Rust 格式检查、Clippy 和针对性测试；在 `ei/` 运行平台构建流程及检查；在 `ui/` 运行 `npm ci && npm run typecheck && npm test && npm run build`。UI 构建生成本地与托管 Web 包以及 TUI。遵循[开发文档](docs_cn/README_cn.md)、[仓库约定](docs_cn/island/conventions.v1.4.0_cn.md)和[智能体指南](AGENTS.md)，并保持中英文文档一致。
