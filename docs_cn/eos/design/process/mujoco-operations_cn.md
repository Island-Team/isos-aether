# 在 EOS v2 中运行 MuJoCo

[English](../../../../docs/eos/design/process/mujoco-operations.md) · [节点 IDL](../../interfaces/node-contributions_cn.md) · [硬件契约](../../interfaces/hardware_cn.md) · [模拟器接口](../../interfaces/mujoco_cn.md) · [R06 停止契约](../../interfaces/simulator-stop_cn.md) · [架构](../architecture_cn.md)


## 准备与运行

先构建 Aether Rust 可执行文件和 IsOS，再从[模拟器仓库](https://github.com/Island-Team/isos-mujoco-sim)构建 v2 包。将 `MUJOCO_ROOT` 指向包含 `lib/libmujoco.so.3.16.0` 的 MuJoCo 3.16.0 安装目录。链接本地包并启动 IsOS 工作台：

```sh
export MUJOCO_ROOT=/absolute/path/to/mujoco-3.16.0
isos init --mode dev
isos pkg link island/mujoco-sim 0.1.1 /absolute/path/to/bundle --manifest /absolute/path/to/bundle/release.json
isos doctor
isos serve
```

在本仓库构建根 Cargo 工作区与 `ui`，在 IsOS 中配置仓库内产物路径，然后运行 `isos doctor`。MuJoCo 软件包仍在仓库外部。


对于已就绪的受管理 MuJoCo 子进程，例行 IsOS Stop 或 Restart 使用 EOS 托管关闭栅栏。只有双执行器停止、子进程回收和清理都得到证明，选中回执 confirmed 才解除这个临时栅栏；它不会创建或清除真正的 `safety.estop` 锁存及既有机器人抑制。回执 unknown 时运动继续禁止。发出新运动前，检查 IsOS 所选终态和新运行的规划器可见 binding。既有急停或结果不明的运动仍须遵循各自的所有者恢复流程；不得清除 `.control` 或重放旧命令 ID 来使 Restart 通过。参见[托管关闭契约](../../interfaces/simulator-stop_cn.md#与急停分离的例行关闭状态)。

EOS 只读取已应用的图。IsOS 解析并验证可执行文件及场景；EOS 启动 MuJoCo，只连接图中声明的适配器。R06 停止路径在子进程终止前栅栏阻断已准入执行器；证明失败时报告 unknown，尚未覆盖的实时故障见[停止设计](../runtime/simulator-stop_cn.md)。包缺失、链接资源发生变化、场景无效或类型端口不兼容，都会在硬件启动前使准入失败。dispatcher 仍是底盘和机械臂动作的唯一命令入口。



若选定图声明底盘或机械臂，而配套停止套接字仍在启动，EOS 会等到严格证明通过后才报告就绪。若到启动期限仍不可用，请查看本次运行的 `eos_start_failed` 与后端日志；此时没有运动准入。只有相机的图可在没有停止 generation 时保持只读。

## 查看相机

预设中的 EOS 相机门节点是将 EOS `CameraFrame` 转为有界 `VisionFrame` 的声明来源。IsosUI Vision 面板可用时，在面板中选择这个活跃且类型兼容的门节点作为预览来源；预设本身不提供 UI 节点绑定。浏览器通过只读 WebSocket 接收 JPEG 帧并显示实际送达帧率。初始目标帧率是相机声明的帧率；Vision 设置可以限制 FPS、图像尺寸和 JPEG 质量。之后由 UI 所有者建立的来源绑定也须使用门节点类型化输出，不能绕过 EOS 命令准入。

用 `isos status` 比较已保存和正在运行的修订，并用 `isos logs eos` 或 IsosUI 日志面板查看启动及相机故障。按 `run_id` 找到当前运行的首条 `eos_ready` 或 `eos_start_failed`；`eos_ready` 确认 EOS 本地就绪，IsOS 状态才确认部署就绪。Rust EOS 生产者向 stderr 写有界 JSONL。IsOS 归属 `data/isos/log/deployments/<deployment>/` 下的部署日志库；受管理的 MuJoCo 输出写入 `data/eos/log/<deployment>/capture/simulator.output.txt`，同目录也有私有适配器宿主诊断；这些都不是共享 EOS 事件。有关联 ID 的停止在 `eos_stopping`、任何 `eos_stop_unconfirmed` 和 `eos_stopped` 上使用同一 `correlation_id`。`isos deployment stop` 只停止图进程，`isos stop` 则关闭整个 manager 和工作台。

## 受管理 Stop 首次使用

[模拟器 Stop 设计](../runtime/simulator-stop_cn.md#验证)描述当前重放。受治理的底盘和机械臂运动之后，Stop 必须提供一份包含两个部件的物理结果、排空子进程组、移除 socket，并发布选定管道的终态凭据。停止 socket 丢失、子进程丢失或响应缺失时，结果保持不确定；在新的运行通过验证前，运动继续被阻止。

## 基准测试隔离

包内基准程序在受管理 EOS 子进程之外使用单独直接启动的模拟器，进行种子重置、步进、读取真实状态和评分。受管理严格子进程拒绝该套接字，因为 Reset 和 Step 会在 Dispatcher 外改变引擎。协议见[包仓库](https://github.com/Island-Team/isos-mujoco-sim)。EOS 客户端只使用图中声明的相机采集、观测和经 Dispatcher 准入的动作；它们不会得到基准测试状态。修改模拟器或节点声明后，运行包内 `ctest --test-dir build --output-on-failure` 以及 IsOS/EOS 集成测试。

**相关：** [IsOS 包流程](../../../../docs/isos/start/README.md) · [EOS 硬件设计](../runtime/hardware_cn.md)
