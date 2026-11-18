# hardware/camera 分组

<!-- doc-navigation:start -->
<details>
<summary>目录：eos/interfaces/node/design/hardware</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [hardware/base 分组](base_cn.md)
  - [hardware/gripper 分组](gripper_cn.md)
  - [hardware/manipulator 分组](manipulator_cn.md)
  - [hardware/mock 分组](mock_cn.md)
  - [hardware/torso 分组](torso_cn.md)

</details>
<!-- doc-navigation:end -->

[English](../../../../../../docs/eos/interfaces/node/design/hardware/camera.md)

以下概述当前 EOS 节点类型及其运行时职责。准确载荷与拒绝规则见所链接的所有者接口。

## eos.hardware.camera.device@v0.2

软件包候选节点。软件包提供的相机观测设备。 输入：无执行器输入。 输出：`frame`：`image_frame@v0.2`。 所选服务从声明的来源产生已验证帧；相机选择属于设备，不属于 Vision 传输节点。 接口：[general-hardware_cn](../../../../interfaces_cn.md#general_hardware_cn)。

<a id="相机适配器节点"></a>

## 相机适配器节点

**位置：**`isos/eos_hardware/camera/<instance-id>` · **类型族：**`isos.eos_hardware.camera@v0.2`。注册类型是可复用定义；部署时选择实例 ID 和非空实例说明。

**状态：** 历史 `isos.eos_*` 设计参考。此类型不是当前内置 EOS 节点库条目。

### 用途与边界

一个相机实例拥有一个适配器，输出类型化帧、健康状态和生命周期状态。声明的 RGB 必须可用；配准深度是可选能力，缺失时必须明确标记不可用。

### 端口与设置

输入：无。输出：`frame: CameraFrame`、`health: Health`、`status: Status`。能力：`rgb8`，以及可选的 `aligned_depth_u16`。所属接口页定义精确字段类型、单位、默认值、校验和示例。注册类型须有非空 `NodeType.description`。

### 生命周期与算法

验证节点配置和适配器族，为设备构造唯一适配器，将样本写入有界最新值缓存，只发布通过校验且新鲜的帧。错误帧只降级本相机，不伪造替代帧。 节点状态、拒绝原因和关联 ID 写入共享部署日志。
