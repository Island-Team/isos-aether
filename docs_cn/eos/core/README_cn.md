# EOS 核心

<!-- doc-navigation:start -->
<details>
<summary>目录：eos/core</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [EOS 架构](architecture_cn.md)
  - [EOS 路线图](roadmap_cn.md)
- 下级目录：
  - [dispatcher/](dispatcher/)
  - [gateway/](gateway/)
  - [hardware/](hardware/)
  - [process/](process/)
  - [skills/](skills/)

</details>
<!-- doc-navigation:end -->

[English](../../../docs/eos/core/README.md) · [EOS](../README_cn.md)

先阅读当前[架构总览](architecture_cn.md)，了解执行归属和生命周期；未来工作见[路线图](roadmap_cn.md)。共用运行时机制涵盖[进程所有权与关闭](process/process_cn.md)、[Dispatcher 准入与调度](dispatcher/dispatcher_cn.md)、[硬件适配器与帧来源](hardware/hardware_cn.md)、[浏览器和 gRPC 网关](gateway/grpc-gateway_cn.md)以及 [Rust 技能宿主](skills/host-rust_cn.md)。当前节点库分组见下方索引。外部消费的精确字段由[接口文档](../interfaces_cn.md#readme_cn)定义。[ARM64 构建](../../isos/start/quick-start_cn.md#arm64-build)、[配置](../../isos/start/quick-start_cn.md#configure-eos)和[排障](../../isos/start/quick-start_cn.md#debug-eos)等操作任务见快速入门。

当前节点库设有[接口层](README_cn.md#eos-interface-layer)、[技能层](README_cn.md#eos-skill-layer)、[调度层](README_cn.md#eos-dispatcher-layer)和[硬件层](README_cn.md#eos-hardware-layer)索引。旧的 `isos.eos_*` 兼容页面保留其类型族标识，不定义当前的 `eos.*` 节点。

## 文档结构验证

使用 `python3 scripts/check_doc_links.py --all` 检查所有者阅读路径（639 个 Markdown 文件，相对链接错误为零），并运行 `git diff --check`（通过）。完整命令输出保存在被忽略的 `data/test/2026-11-14-doc-layout/checks.log`。

<a id="eos-hardware-layer"></a>

## EOS hardware 层

hardware 层把图节点适配到选定设备。内置 mock 分组服务于演示机械臂路径。其他分组是软件包贡献的候选节点，仅在对应精确软件包安装并通过验证后出现于节点库。各适配器负责设备专属行为；共用运行时边界见[核心硬件设计](hardware/hardware_cn.md)。

分组介绍概述 [mock](../interfaces/node/design/hardware/mock_cn.md)、[camera](../interfaces/node/design/hardware/camera_cn.md#相机适配器节点)、[base](../interfaces/node/design/hardware/base_cn.md#移动底盘适配器节点)、[manipulator](../interfaces/node/design/hardware/manipulator_cn.md#机械臂适配器节点)、[gripper](../interfaces/node/design/hardware/gripper_cn.md) 和 [torso](../interfaces/node/design/hardware/torso_cn.md) 的位置。节点页面描述行为；精确公共契约由[硬件接口](../interfaces_cn.md#readme_cn--eos-hardware-contracts)定义。

<a id="eos-dispatcher-layer"></a>

## EOS dispatcher 层

Dispatcher 层是 EOS 的控制边界。arm 分组根据当前运行身份和运动限制准入关节增量命令，再把已接受的命令传给选定的模拟执行器。monitor 节点承载返回观测路径，不建立另一条执行器路线。共用准入、调度及路由机制见[核心 Dispatcher 设计](dispatcher/dispatcher_cn.md)。

[arm 分组](../interfaces/node/design/dispatcher/arm_cn.md)包含两个当前节点设计。精确端口和载荷规则见 [Dispatcher 接口](../interfaces_cn.md#readme_cn--eos-dispatcher-contracts)。

<a id="eos-interface-layer"></a>

## EOS interface 层

interface 层将 EOS 图路线连接到外部输入和输出边界。ingress 接收声明的触发信号；egress 暴露机械臂状态和经过验证的视觉帧。图连线与运行身份确定各传输的来源和目的地。这些节点不选择硬件，也不绕过 Dispatcher 准入。

节点行为见 [ingress 分组](../interfaces/node/design/interface/ingress_cn.md)及 [egress 分组](../interfaces/node/design/interface/egress_cn.md)。精确边界值由[接口契约](../interfaces_cn.md#readme_cn--eos-interface-contracts)定义；共用传输机制见[网关设计](gateway/grpc-gateway_cn.md)。

<a id="eos-skill-layer"></a>

## EOS skill 层

skill 层包含 EOS 图中的软件变换。当前 demo 分组在按钮到模拟机械臂的控制及返回链中生成关节增量并处理机械臂状态。技能通过类型化图端口连接；执行器准入仍由 Dispatcher 负责。

节点设计见 [demo 分组](../interfaces/node/design/skill/demo_cn.md)。[技能接口](../interfaces_cn.md#readme_cn--eos-skill-contracts)定义外部节点契约，[Rust 技能宿主设计](skills/host-rust_cn.md)描述共用运行时托管。

<a id="eos-legacy-layer"></a>

## 旧 EOS 节点设计

分组章节描述具有 `isos.eos_*` 身份的已保存图兼容类型，不定义当前 `eos.<layer>.<group>` 节点库中的节点。参见[调度绑定](../interfaces/node/design/dispatcher/base_cn.md#底盘原语绑定节点)、[控制](../interfaces/node/design/dispatcher/control_cn.md#共享控制调度器节点)、[机械臂绑定](../interfaces/node/design/dispatcher/manipulator_cn.md#机械臂原语绑定节点)、[硬件底盘](../interfaces/node/design/hardware/base_cn.md)、[摄像头](../interfaces/node/design/hardware/camera_cn.md)、[机械臂](../interfaces/node/design/hardware/manipulator_cn.md)。
