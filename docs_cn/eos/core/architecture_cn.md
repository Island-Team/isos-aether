# EOS 架构

<!-- doc-navigation:start -->
<details>
<summary>目录：eos/core</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [EOS 核心](README_cn.md)
  - [EOS 路线图](roadmap_cn.md)
- 下级目录：
  - [dispatcher/](dispatcher/)
  - [gateway/](gateway/)
  - [hardware/](hardware/)
  - [interface/](interface/)
  - [process/](process/)
  - [skill/](skill/)
  - [skills/](skills/)

</details>
<!-- doc-navigation:end -->

[English](../../../docs/eos/core/architecture.md) · [EOS](../README_cn.md) · [接口](../interfaces_cn.md#readme_cn)

EOS 是执行与物理安全核心。它读取 IsOS 已应用图中自己的投影，只构建已声明适配器，经 Dispatcher 准入命令，发布观测健康状态，并监管直接的工作进程子进程。持久归属服务通过私有 Unix socket 提供 Run、Status、Stop、激活及监测读取。当前字段和行为归[硬件](../interfaces/node/hardware/mock_cn.md#hardware_mock_cn--arm)、[机械臂控制](../interfaces_cn.md#arm_control_cn)、[机械臂监测](../interfaces_cn.md#arm_monitor_cn)和[归属方健康](../../isos/interfaces/server/eos_cn.md#owner_health_cn)契约所有。

## 当前运行边界

在图 `0.2.1` 中，控制链为接口入口 → 演示技能 → 机械臂 Dispatcher → 六关节模拟适配器。独立的只读链从机械臂观测源开始，经监测 Dispatcher、状态技能和接口出口返回。EOS 在 Run 前验证八个节点及连接。归属服务验证投影与工作程序精确摘要，启动所需工作进程，在 Status 时检查，并在 Stop 时关闭准入、回收进程。已应用图保持只读。模拟适配器不连接实体设备。

Dispatcher 在物理写入前检查命令身份、期限、能力、健康、采样新鲜度、限制和安全状态。它持久预留命令，不自动重放结果不确定的释放。本地观测故障会阻止运动，直到新鲜有效采样恢复。Stop 关闭准入，先停止已准入执行器，再终止所选直接子进程。[硬件设计](hardware/hardware_cn.md)与[调度设计](dispatcher/dispatcher_cn.md)说明当前机制。

归属服务还可使完整模拟链与已选原生和软件包节点并行运行。Setup 准入受支持的类型化路线；Run 将其绑定到已保存修订与运行 ID。归属服务监管模拟和原生工作进程，通过 IsOS 检查已选 v2 服务健康，并在工作进程之间交付获准观测输出。当前 v2 服务 ABI 能产生观测，但没有类型化输入回调。现有行为见[所有者交换](../../isos/interfaces/server/eos_cn.md#node_contributions_cn)，剩余资格门槛见[跨后端路线图](roadmap_cn.md#跨后端图执行)。

未来方向见[路线图](roadmap_cn.md)。

## Rust crate 职责

`aether-system` 负责 EOS 归属服务与工作进程运行时，包括 Dispatcher 准入和私有进程 I/O。`aether-node` 在 `src/<layer>/<group>/` 下实现内置节点；`aether-runtime` 库保留当前工作进程使用的已安装原生 ABI 文件。`aether-node-profile` 负责解析图、注册节点类型和验证连接，让 Setup 与运行时读取同一组节点契约。`aether-hal` 定义适配器边界，`aether-lib` 保存小型共享时间、事件和有界 I/O 基础功能。服务端和运行时依赖这些库，而不改变图或线路身份的归属。
