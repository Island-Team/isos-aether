<a id="hardware_mock_cn"></a>
<a id="hardware_mock_cn--hardwaremock-分组"></a>
# hardware/mock 分组

<!-- doc-navigation:start -->
<details>
<summary>目录：eos/interfaces/node/hardware</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [hardware/base 分组](base_cn.md)
  - [hardware/camera 分组](camera_cn.md)
  - [hardware/gripper 分组](gripper_cn.md)
  - [hardware/manipulator 分组](manipulator_cn.md)
  - [hardware/torso 分组](torso_cn.md)

</details>
<!-- doc-navigation:end -->

[节点目录](../README_cn.md) · [English](../../../../../docs/eos/interfaces/node/hardware/mock.md#hardware_mock)

此分组包含以下 EOS 节点类型。各节点的公共契约见本页下文。

- [arm](mock_cn.md#hardware_mock_cn--arm)
- [arm_observer](mock_cn.md#hardware_mock_cn--arm-observer)
- [arm_output](mock_cn.md#hardware_mock_cn--arm-output)

<a id="hardware_mock_cn--arm"></a>
<a id="arm"></a>

<a id="hardware_mock_cn--eos-模拟机械臂硬件边界"></a>
## EOS 模拟机械臂硬件边界

图 `0.2.1` 将 `eos.hardware.mock.arm@v0.2` 注册为六关节 Manipulator/Adapter 节点。其输入从 EOS 调度器接收 `arm_joint_delta@v0.2`。节点声明六个有序关节名称、`mock-arm` 适配器族、`joint_positions` 能力、健康时序、观测边界和运动限值。工作进程在每次运行开始时把关节位置初始化为零，并为每个已准入且不同的增量执行一次累积。

IsOS 在 Apply 前准入完整图；EOS 在 Run 前验证四节点投影。调度器在模拟适配器更新状态前检查事件身份、单次增量限值和结果关节位置。EOS 拒绝 `movement.limits.max_joint_position_rad` 超过 3.14 rad 模拟安全界限的图；已准入的数值原样传给工作进程。Stop 在子进程退出前关闭准入。[机械臂控制契约](../../../interfaces_cn.md#arm_control_cn)定义载荷、所有者路由、终态结果和错误。此演示不构建摄像头、底盘、C++ 适配器或实体机械臂。

<a id="hardware_mock_cn--arm-observer"></a>
<a id="arm-observer"></a>

<a id="hardware_mock_cn--eoshardwaremockarmobserverv02"></a>
## eos.hardware.mock.arm_observer@v0.2

当前 IsOS 节点库中的内置节点类型。

<a id="hardware_mock_cn--节点作用"></a>
### 节点作用

独立读取唯一的模拟机械臂采样器。

- 输入：无图输入。
- 输出：`observation`：`arm_monitor_state@v0.2`，进程内。

<a id="hardware_mock_cn--所有者与生命周期"></a>
### 所有者与生命周期

观测来源绑定当前运行；此节点不能接纳或移动执行器。

准确载荷字段及拒绝规则见本页所链接的 EOS 所有者契约。

参见 [arm-monitor_cn](../../../interfaces_cn.md#arm_monitor_cn)。

<a id="hardware_mock_cn--arm-output"></a>
<a id="arm-output"></a>

<a id="hardware_mock_cn--eoshardwaremockarmoutputv02"></a>
## eos.hardware.mock.arm_output@v0.2

当前 IsOS 节点库中的内置节点类型。

<a id="hardware_mock_cn--节点作用-1"></a>
### 节点作用

将模拟机械臂观测传给监视链。

- 输入：`observation`：`arm_monitor_state@v0.2`，进程内。
- 输出：`observation`：`arm_monitor_state@v0.2`，进程内。

<a id="hardware_mock_cn--所有者与生命周期-1"></a>
### 所有者与生命周期

用于已保存图的兼容类型；仅转发观测，不创建另一条执行器路径。

准确载荷字段及拒绝规则见本页所链接的 EOS 所有者契约。

参见 [arm_cn](mock_cn.md#hardware_mock_cn--arm)。
