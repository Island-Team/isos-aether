# IslandOS 服务

<!-- doc-navigation:start -->
<details>
<summary>目录：isos/core</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [IsOS 核心](README_cn.md)
  - [IsOS 架构](architecture_cn.md)
  - [IsOS 命令行](cli_cn.md)
  - [IsOS 配置](config_cn.md)
  - [本地核心日志](logger_cn.md)
  - [IsOS 软件包](pkg_cn.md)
  - [IsOS 路线图](roadmap_cn.md)
  - [IsOS 服务端](server_cn.md)
- 下级目录：
  - [ui/](ui/)

</details>
<!-- doc-navigation:end -->

[English](../../../docs/isos/core/service.md) · [核心索引](README_cn.md)

## 独立服务定义

[English](../../../docs/isos/core/service.md) · [管理器 IPC](../interfaces/manager_cn.md#manager_protocol)

本页定义仅所有者可用的管理器使用的服务定义，**不是 HTTP 路由**。服务在部署图外，只通过显式 CLI 或管理器操作启动。`isos serve` 和 `isos run` 不隐式启动它们。管理器关闭时停止其服务。当前定义为 TOML `version="1.0.0"`；该版本独立于 IsOS HTTP 模式。

```toml
version = "1.0.0"
id = "example"
command = ["/absolute/verified/launcher"]
cwd = "/absolute/verified"
depends_on = []
restart = "on_failure"
max_restarts = 2
start_timeout_ms = 30000
stop_timeout_ms = 5000

[artifact]
path = "/absolute/verified/launcher"
sha256 = "<64 lowercase hexadecimal digits>"

[readiness]
kind = "tcp"
target = "127.0.0.1:18003"
```

| 字段 | 类型 / 单位 | 用途与验证 | 缺省值 |
| --- | --- | --- | --- |
| `version`, `id` | 字符串 / 定义版本与稳定 ID | 精确解析版本与服务身份。 | 必填。 |
| `command`, `cwd` | 字符串数组与绝对路径 / 进程输入 | 显式启动输入，不从软件包 ID 推断。 | 必填。 |
| `depends_on` | 字符串数组 / 服务 ID | 独立服务间排序；循环拒绝。 | 无依赖时为空。 |
| `restart`, `max_restarts` | 枚举与整数 / 策略、次数 | 限制自动恢复。 | 必填。 |
| `start_timeout_ms`, `stop_timeout_ms` | 正整数 / 毫秒 | 限制就绪与终止时间。 | 必填。 |
| `artifact.path`, `.sha256` | 绝对路径与 64 位十六进制 / 字节 | 绑定选中可执行文件。 | 必填。 |
| `readiness.kind`, `.target` | 字符串 / 探测 | 所有者在成功前检查声明就绪。 | 必填。 |

服务定义不提供软件包特定解析器，也不绕过图节点的通用软件包贡献路径。

## 已选包服务

`0.2.1` 发布模式可以为 EOS 硬件节点声明包运行服务。Apply 只读取已选的 `adapter.package` 与 `adapter.settings.service_instance` 组合，校验部署本地包配置，并将其精确摘要写入不可变生成清单。Run 与 Restart 在启动前比较该摘要。IsOS 按包 ID、版本和服务实例合并节点，在独立会话中将每个已验证入口作为直接子进程启动，并在报告就绪前记录带运行栅栏的收据。EOS 获得按节点路径索引的私有套接字交接信息。旧 `0.2.0` 进程节点继续使用原有生命周期。

Stop 先请求所有者关闭，再终止包服务。缺少所有者证明或包子进程意外退出仍为未知结果。服务收据包含 PID 启动时间和运行身份，因此重启后的管理器不会向重用的 PID 发送信号，也不会执行过期 Stop。此路径目前支持 Linux Unix 套接字；其他平台的加载器留待后续实现。

新 Run 启动前，IsOS 仅在通过 PID 和启动时间确认旧运行记录的所有子进程均已退出时，才读取剩余输出并清除旧服务收据。仍存活的子进程会保留收据并阻止新 Run。启动失败时只清理属于该候选运行的收据。

2026-11-18 的过期收据恢复检查记录在忽略目录 `data/test/20261118-stale-package-receipt/`。复现命令为 `ISOS_AETHER_BIN_DIR=$PWD/target/debug cargo test -p isos --bin isos`、`cargo clippy -p isos --bin isos -- -D warnings` 和 `cargo fmt --all -- --check`。确认收据记录的子进程已退出后，执行 Stop、以当前修订 Apply 已保存图并 Run；本地回放使用从源码安装的 MuJoCo 包启动了 EOS 和 EI。
收据专项测试、Clippy 和格式检查通过。完整 IsOS 测试中 362 项通过、2 项失败：demo 固件预期与事先存在的本地 `rgb8` 修改不一致；子进程重启计时测试单独重跑时通过。

2026-11-12 服务监督切片的证据保存在忽略目录 `data/test/20261112-package-service-supervisor/`。复现命令为 `ISOS_AETHER_BIN_DIR=<source-checkout>/target/debug cargo test -p isos --bin isos`、`cargo clippy -p isos --bin isos -- -D warnings`、`cargo fmt --all -- --check` 以及 `python3 scripts/check_doc_links.py --all`。全部通过（320 项测试通过，两项忽略）。真实包的 Apply → Run → Stop 验证属于组合集成切片，不属于这个独立监督测试。

已选服务的 stdout 和 stderr 写入权限为 `0600` 的私有文件，再以有界、已清除控制字符的 IsOS `package_service_output` 事件导入，事件带收据中的运行 ID、精确包版本、实例、输出流和代次。Status 与 Stop 读取新增字节；管理器重启后可从收据偏移量继续读取，不会断开子进程的输出描述符。Apply 代次还对声明的外部文件依赖（包括本地原生库）保存指纹，因此文件字节变化会在服务启动前阻止 Run。

2026-11-12 日志与外部依赖后续切片的证据位于忽略目录 `data/test/20261112-package-service-supervisor-followup/`。复现命令为 `cargo build -p aether-runtime --bin aether-profile`、`ISOS_AETHER_BIN_DIR=$PWD/target/debug cargo test -p isos --bin isos`、`cargo clippy -p isos --bin isos -- -D warnings`、`cargo fmt --all -- --check` 和 `python3 scripts/check_doc_links.py --all`。独立 IsOS 测试通过 322 项，忽略两项；目录测试会比较 EOS 实时类型，因此需要本地 EOS 二进制文件。

2026-11-12 审查修复要求每个已选设备回答有帧格式的 `health` 请求，且包含 `ok`、`accepted`、`strict-socket-v1` 和精确的 IsOS 服务代次，然后启动或状态才能报告就绪。Stop 在有界期限内发送 TERM 或 KILL，等待退出并回收子进程。读取包配置时使用 `O_NOFOLLOW`，并将打开文件的 inode 和设备号与已检查的私有文件比较。证据位于忽略目录 `data/test/20261112-supervisor-review-fixes/`。

包服务 Unix 套接字使用 `/tmp/isos-package-<uid>/` 下的短 Linux 运行路径。IsOS 创建并检查权限为 `0700` 的所属用户私有目录，将部署运行目录、运行 ID 和精确服务版本哈希为套接字文件名，并只在运行收据及 EOS 交接信息中记录所得路径。Stop 删除套接字和 `.stop` 伴随套接字。因此即使 `ISOS_HOME` 或验证路径很长，Unix 套接字地址也不会超过内核限制。长路径专项测试覆盖地址长度和私有目录权限。

2026-11-12 短套接字证据位于忽略目录 `data/test/20261112-short-package-socket/`。复现命令为 `ISOS_AETHER_BIN_DIR=$PWD/target/debug cargo test -p isos --bin isos`、`cargo clippy -p isos --bin isos -- -D warnings`、`cargo fmt --all -- --check` 和 `python3 scripts/check_doc_links.py --all`。IsOS 测试通过 327 项，忽略两项；Clippy、格式和链接检查通过。

## 受管理组件节点

[English](../../../docs/isos/core/service.md) · [图契约](../../node/interfaces/graph_cn.md#instance) · [进程贡献](../interfaces/package/README_cn.md#process)

注册进程类型把 IsOS 管理的组件放在 `isos/process/component` 下。类型声明决定允许的能力、设置、端口及生命周期；实例提供稳定身份、依赖、精确软件包锁定和声明设置值。IsOS 仅解析经过验证的入口，并只监护自己的直接子进程。其他核心的软件包贡献进程交给该所有者。不接受任意图命令或按包 ID 特制的启动器。

就绪由所有者验证。就绪失败或停止不确定不能变成成功运行。实例必须使用当前已注册的类型 ID。
