# 相机预览网关节点

[English](../../../../docs/eos/design/interface/gate.md) · [EOS 架构](../architecture_cn.md) · [vision](../../interfaces/vision_cn.md) · [hardware](../../interfaces/hardware_cn.md)

**位置：**`isos/eos_interface/gate/<instance-id>` · **类型族：**`isos.eos_interface.gate@v0.2`。注册类型是可复用定义；部署时选择实例 ID 和非空实例说明。

**状态：** 已注册用于图格式 `0.2.0`。

## 用途与边界

将已准入的相机帧转换为有界只读预览，供 IsosUI Vision 使用。它不能发送相机控制或执行器命令。

## 端口与设置

输入：`frame: CameraFrame`（图像，进程内）。输出：`preview: VisionFrame`（图像，WebSocket）。无节点专用设置。所属接口页定义精确字段类型、单位、默认值、校验和示例。注册类型须有非空 `NodeType.description`。

## 生命周期与算法

绑定一个兼容相机来源，验证并限制帧尺寸和时效，编码或转发最新允许的预览，在背压下丢弃过期帧。查看器断开不影响相机采集和安全。 节点状态、拒绝原因和关联 ID 写入共享部署日志。
