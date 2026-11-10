# Rust EOS 服务

[English](../../../../docs/eos/design/runtime/services.md) · [运行时基础](foundation_cn.md) · [Rust skill 契约](../../interfaces/skills/rust_cn.md)

根 Cargo 工作区以源码版本 `0.2.1` 构建 EOS 服务。IsOS 应用包含独立 `isos`、`ei` 和 `eos` core 的 `0.2.0` 图，并编排 EOS 所有者。持久所有者监管其运行时工作进程。当前演示工作进程拥有已声明的模拟机械臂适配器和 Dispatcher 准入。旧通用 `aether-runtime` 路径拥有 HAL 适配器、持久 operation journal 及直接管理的 MuJoCo 子进程。HTTP bridge 与 TUI 使用运行时仅所有者可访问的 Unix 控制 socket；它们不拥有执行器路线。配置服务发布一份已验证的只读图快照。HTTP 服务只绑定 loopback。

| 可执行文件 | 调用 | 职责 |
| --- | --- | --- |
| `aether-profile` | `validate --file <graph>` | 准入前验证图和已注册节点类型。 |
| `aether-runtime` | `--profile <graph> --journal-dir <dir> --socket <path>` | 拥有适配器、Dispatcher、journal、gRPC listener、MuJoCo 子进程和受治理的 Stop。受管理运行由 IsOS 提供运行、软件包和授权输入。 |
| `aether-http-bridge` | `--socket <path> --listen <loopback:port>` | 代理 `/api/eos/v2/control` 有界控制 JSON、`/api/eos/v2/health` 健康和授权视觉读取。 |
| `aether-config-server` | `--profile <graph> --listen <loopback:port>` | 提供 `/api/eos/v2/config`、`/config/validate`、`/types`、`/data-types`、`/adapter-families` 和 `/primitive-specs`；没有写入端点。 |

`aether-control` 提供 bridge 与 TUI 共享的有界客户端。控制请求上限为 64 KiB、响应上限为 16 MiB，socket 读取截止时间为两秒。`capture` 以 `rgb8_base64` 返回 RGB8，以可选的 `depth_u16_le_base64` 返回深度；`observe` 返回类型化底盘或机械臂数据。bridge 在转发前验证请求大小和身份。Dispatcher 始终是唯一执行器准入与释放路线。

## Rust skill worker

运行时只加载 IsOS 选中并验证的原生 Rust `.skill` 可执行文件。它私有暂存已验证字节，每次检查或调用都启动新的受限 `aether-runtime --skill-worker` 子进程。worker 接收私有协议 socket 与暂存可执行文件描述符，然后施加资源限制、Landlock 和 seccomp。Primitive 调用经宿主 broker 和 Dispatcher 返回；worker 不持有直接适配器句柄。EOS 负责取消、有界回收和实时执行状态。详见 [Rust skill 契约](../../interfaces/skills/rust_cn.md)。

## 首次使用与检查

在仓库根目录运行 `cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings` 和 `RUST_TEST_THREADS=1 cargo test --workspace --no-fail-fast`。使用 `isos serve` 启动 IsOS 管理器，再用 `isos run` 运行已应用部署；通过 `isos status --json` 和 `isos logs eos --lines 200` 检查。使用 `isos deployment stop` 停止部署，或使用 `isos stop` 停止管理器。直接调用服务适用于聚焦的边界测试；生产图所有权和软件包解析由 IsOS 负责。

2026-11-07 的统一 MuJoCo 重放使用 Rust 服务验证了 Run、RGB 相机采集、primitive 完成、Restart、确认 Stop 和子进程故障拒绝。命令与原始输出位于忽略目录 `data/test/20261107-final/unified-mujoco-replay-fixed2/`。每次新重放都应保存准确命令、源码修订、软件包 pin 和环境。除成功的控制与观察外，也要验证故障和 Stop 行为。
