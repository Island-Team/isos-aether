# EOS 硬件运行时设计

<!-- doc-navigation:start -->
<details>
<summary>目录：eos/core/hardware</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [开发 EOS 硬件适配器](hardware-guide_cn.md)

</details>
<!-- doc-navigation:end -->

[English](../../../../docs/eos/core/hardware/hardware.md) · [硬件契约](../../interfaces/node/hardware/mock_cn.md#hardware_mock_cn--arm) · [Dispatcher](../dispatcher/dispatcher_cn.md)

## 所有权与数据流

IsOS 管理保存的图修订、已验证的软件包资源及 Apply。当前 `0.2.1` 图包含独立的 `isos`、`ei` 和 `eos` core。EOS 只读取自己的投影，并在每次工作进程运行时创建一次声明的 `eos.hardware.mock.arm@v0.2` 适配器。历史 `isos.eos_hardware.*@v0.2` 路径保留为兼容边界；新软件包摄像头观测使用当前已注册的 `eos.hardware.camera@v0.2` 路径。注册类型决定位置、设置、端口、能力和生命周期；实例提供身份和值。EOS 管理实时状态与健康，Dispatcher 管理命令准入和安全。

Rust 硬件层传递类型化的摄像头、底盘和机械臂采样及命令。内置与软件包适配器通过各自声明的实现进入同一准入边界。IsOS 验证所选软件包的字节与资源；EOS 验证适配器、管理实时健康，并在确认 Stop 前关闭 Dispatcher 准入。软件包服务端点仅供选定运行私用，不能形成第二条命令路由。


## 准入与故障

EOS 在就绪前校验节点位置、适配器族、设置、图引用、类型化边、健康时序、坐标系 ID、运动模式与界限。畸形或过期样本仅降级所属节点。后端不能创建第二条执行器路线。所有运动携带活动修订、绑定、类型化载荷、期限与持久命令 ID 经 Dispatcher 准入。

Stop 关闭运动准入，请求已准入执行器停止，记录确认或未知结果，再终止并回收选定的服务子进程。适配器或子进程丢失会降级依赖节点并禁止运动。实体适配器须执行写入边界停止及看门狗行为；mock 测试不能替代硬件资格验证。

## 后端选择

当前 Rust 运行时准入 CXX mock 硬件适配器、已验证的 v1 软件包摄像头观测节点，以及通过 Dispatcher 命令准入并提供运行绑定 Stop 证明的选定 v2 软件包服务设备。IsOS 安装或链接精确 `v0.2.x` 软件包，在图格式 `0.2.1` 中选定进程及摄像头节点，并验证可执行文件、资源和共享库。EOS 只读已应用图，直接监督软件包进程，并通过私有观测路线运行声明的摄像头适配器。软件包底盘、机械臂、夹爪和躯干命令在就绪前必须具备已验证的 v2 服务移交、Dispatcher 准入及实体 Stop 证明。

v2 原生工作进程在图路线计划之外处理 Dispatcher 命令回执。所有者准入命令后，工作进程仅调用目标执行器回调，并将唯一且经校验的 `command_result` 返回所有者。图输出和观测回调不能使用这个保留端口，避免回执形成第二条执行器或图路线。

注册节点类型决定位置、能力、设置、端口及生命周期；实例提供身份和选定值。IsOS 在 Apply 前检查精确软件包固定版本与类型化边；EOS 在就绪前检查适配器设置与健康。Dispatcher 准入所有执行器命令，并负责安全与 Stop。Run、观测与 Stop 见[软件包观测契约](../../../isos/interfaces/server/eos_cn.md#package_observer_cn)。软件包专属设置属于该软件包仓库。

当前演示图使用已注册的 EOS 模拟机械臂适配器和[激活契约](../../interfaces_cn.md#arm_control_cn)。六关节状态只属于当前工作进程运行；Dispatcher 在更新前准入技能产生的增量。

所有者将只读 `ArmObserver` 实例绑定到图中零输入的硬件层观测源路径，并将该路径传给工作进程。Monitor 请求经该实例读取唯一模拟机械臂适配器的采样器，再将[类型化机械臂观测](../../interfaces_cn.md#arm_monitor_cn)送入声明的监控调度器、状态技能和接口出口节点。工作进程默认以 100 Hz 采样，并在每次准入运动后立即采样。图可通过模拟机械臂的高级设置覆盖 10 ms 采样间隔及 50 ms 新鲜度阈值。每个观测携带观测源路径、运行隔离、单调采样时间、序号、按图排列的关节名称和状态。所有者通过只读私有路由公开最新样本；样本过期或无效时阻止新运动，直到有效样本恢复就绪。

## 异步演示执行与监控

EOS 工作进程在按运行 ID 命名的日志中同步记录命令保留后返回接收回执。单一执行线程按序执行已接收命令；结果查询使用事件 ID 取得当前或最终结果。重复事件 ID 不会产生第二次运动。Stop 关闭新准入并等待执行线程退出；进程丢失后的未确认结果保持不确定，不自动重放。机械臂采样线程独立维护最新观测及序号，多个读取者只读取该快照，不驱动采样。详细字段见[机械臂控制契约](../../interfaces_cn.md#arm_control_cn)。

<a id="读取器预检设计"></a>

## 读取器预检设计

EOS 公布每个所选二进制文件接受的图版本、注册节点类型与功能。IsOS 使用这些精确观测决定保存的部署能否运行。profile 工具、runtime 与 config server 从同一个注册表取得内建类型 ID。包贡献类型仅通过传给 setup 的已验证 sidecar 加入；二进制探针不宣称包已经安装。

Validate 与 Apply 时，IsOS 固定保存图的修订，验证包锁定与所选二进制文件，根据图节点和边推导所需功能，并直接调用每个二进制文件。它在归属方渲染或进程变化前检查精确身份、类型 ID 与所需功能。Run 与 Restart 再次检查不可变代和所选二进制文件的哈希。Apply 选择新一代。

运行 `cargo build --workspace` 构建 EOS 二进制文件，再运行 `python3 tests/reader_features_cli.py`。重放检查当前图 `0.2.1` 的探针身份、setup 身份一致性，以及仓库内 mock profile 的验证。完整输出保存在忽略 Git 的 `data/test/<run-id>/`。

<a id="图-vision-首次使用"></a>

## 图 Vision 首次使用

将已声明相机的 `frame` 输出连接至 `eos.interface.egress.vision_flow@v0.2`，再将传输节点连接至 `isos.ui.vision.monitor@v0.2`。应用图、运行、等待就绪，然后打开视觉页面。检查显示的来源和尺寸与已接受画面一致。停止后确认图像清除。EOS 私有 `vision_frame` 交换按运行和修订围栏；畸形画面与 worker 故障不会返回图像。

### 相机 Vision 数据类验证（2026-11-15）

图中 Vision 输入使用数据类 `image`，与软件包相机的 `image_frame@v0.2` 输出一致。从仓库根目录运行 `cargo fmt --all -- --check`、`cargo clippy -p aether-node-profile --all-targets -- -D warnings`、`cargo test -p aether-node-profile` 和 `python3 scripts/check_doc_links.py --all`。这些检查均通过；完整输出保存在被忽略的 `data/test/20261115-vision-camera-image-class/` 下。

### 移除验证（2026-11-14）

旧的相机 gate／JPEG bridge 路由已移除，同时保留图中的 `vision_frame` 交换。在仓库根目录执行 `cargo test -p aether-http-bridge --test bridge -- --test-threads=1`、`cargo check -p isos`、`cargo fmt --all -- --check` 和 `python3 scripts/check_doc_links.py --all` 均通过；完整输出位于忽略目录 `data/test/20261114-legacy-vision-removal/`。EI 所有者测试（`uv run pytest -q tests/ei tests/system/test_reader_features.py tests/benchmarks/test_ei_q03.py`）676 项通过、6 项跳过；UI 类型检查和测试套件 224 项通过。
