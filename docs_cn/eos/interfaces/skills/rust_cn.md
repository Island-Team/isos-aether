# Rust 技能可执行程序契约

[English](../../../../docs/eos/interfaces/skills/rust.md) · [SkillService](../skills_cn.md) · [开发指南](../../design/runtime/skills/host-rust-guide_cn.md)

EOS 在受限子进程中运行已选中的 Rust 技能可执行程序。作者实现 `aether_skill::Skill` trait，并在 `main` 中调用 `aether_skill::serve`。技能的 `Descriptor` 声明名称、SemVer 版本、说明、对象类型的参数和结果 JSON Schema、primitive 以及可重入性。一个可执行程序最多注册 256 个技能。名称使用小写字母、数字、`.`、`_` 和 `-`；分隔符后必须跟字母或数字。

编写接口见 [`crates/aether-skill/src/lib.rs`](../../../../crates/aether-skill/src/lib.rs)。`Skill::execute` 接收命令 ID、对象参数和 `Host`。`Host::call_primitive` 是唯一的执行器调用路径。返回值为一次 `Completion`，其结果是对象。`Host::is_cancelled` 表示 Stop 请求。EOS 在发布目录前验证描述符、Schema 和 primitive 声明，并在每次执行时验证参数和 broker 准入。

## 图选择

EOS 进程节点可设置 `settings.skill_executables`，最多包含 16 个明确条目。每项包含 `package: {id, version}` 和以 `.skill` 结尾的相对路径 `executable`。IsOS 在 Apply 及 Run/Restart 前验证软件包、精确路径和摘要；它将 `isos/skill-selection.json` 写入选定 generation，并以 `EOS_SKILL_SELECTION_FILE` 向 EOS 传递路径。仅安装软件包不会启用技能。

选择文件包含整数 `schema_version: 1`、保存图的 SHA-256 `active_revision`，以及按软件包 ID、版本和可执行程序路径排序的 `executables` 数组。每项包含 `package_id`、`package_version`、绝对路径 `package_root` 和 `payload_root`、`receipt_sha256`、`release_sha256`、`payload_sha256`、相对路径 `executable` 和 `executable_sha256`。摘要均为小写十六进制 SHA-256。EOS 拒绝文件变化、路径逃逸、链接、非普通文件、不安全权限、不允许的 ELF 依赖和重复技能身份。它私有暂存已验证字节，并在发布目录前通过受限 worker 检查每个可执行程序。

运行时公布功能 `rust-skill-v1`。非空选择要求 IsOS 候选预检具备该功能。预检报告回显软件包身份、可执行程序路径、摘要和技能数量；SkillService 不暴露软件包代码路径或授权凭据。

## 进程与协议

每次检查和执行都启动新的 `aether-runtime --skill-worker` 子进程。运行时只传递私有 `SOCK_SEQPACKET` socket 和暂存的可执行程序文件描述符。子进程清空环境、验证对端和描述符，设置资源限制、Landlock 和 seccomp，然后执行 Rust 程序。进程协议版本为 `2`，包含 `Inspect`、`Invoke`、`PrimitiveReply`、`Catalog`、`PrimitiveCall`、`Completion` 和 `Failure` 消息。消息有大小上限；大消息使用密封的内存文件。EOS 管理取消、有界回收、broker 授权和运行状态。

技能进程不能打开网络 socket 或创建其他进程。每个 primitive 请求均经过已准入描述符、图绑定、dispatcher 和期限检查。Stop 请求向进程发送信号；若进程未退出，EOS 会在限定时间内终止它。EOS 在终止模拟器子进程前停止已准入执行器。

## 编写与验证

可运行的 [`echo` 示例](../../../../crates/aether-skill/examples/echo.rs)展示 trait 与完成结果。技能构建为 Linux 可执行程序，最终文件名以 `.skill` 结尾，并使用 Cargo 锁定构建。使用 Cargo 构建二进制文件，将其放入经 IsOS 验证的技能 release，并通过图选择精确路径。

执行 `cargo fmt --all -- --check`、`cargo test -p aether-skill` 和 `cargo test -p aether-runtime --features test-hooks --test rust_skill_worker --test rust_skill_catalog`。进程测试覆盖目录准入、变更 payload 拒绝、经 broker 的 primitive 调用、完成和子进程回收。
