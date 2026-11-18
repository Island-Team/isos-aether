# 机械臂原语绑定节点

<!-- doc-navigation:start -->
<details>
<summary>目录：eos/interfaces/node/design/dispatcher</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [dispatcher/arm 分组](arm_cn.md)
  - [底盘原语绑定节点](base_cn.md)
  - [共享控制调度器节点](control_cn.md)

</details>
<!-- doc-navigation:end -->

[English](../../../../../../docs/eos/interfaces/node/design/dispatcher/manipulator.md)


<a id="机械臂原语绑定节点"></a>

## 机械臂原语绑定节点

**位置：**`isos/eos_dispatcher/manipulator/<instance-id>` · **类型族：**`isos.eos_dispatcher.manipulator@v0.2`。注册类型是可复用定义；部署时选择实例 ID 和非空实例说明。

**状态：** 历史 `isos.eos_*` 设计参考。此类型不是当前内置 EOS 节点库条目。

### 用途与边界

节点命名一个带版本的机械臂操作，并通过显式边连接兼容机械臂；设备仍由硬件适配器拥有。

计划中的 IsosUI EOS 面板把这个**活动实例**显示为一项可选原语控制。面板显示类型与实例描述、唯一目标、声明的运动模式及 EOS 实时健康；调用经现有 `primitive_typed` bridge，并显示关联结果。图连线是绑定，不是可执行调用。

### 端口与设置

设置：`primitive_name`、`primitive_version`、`movement_type`。输出：`command: ManipulatorMovement`；无输入。所属接口页定义精确字段类型、单位、默认值、校验和示例。注册类型须有非空 `NodeType.description`。

### 生命周期与算法

验证绑定目录、边目标、关节顺序和类型化命令头，再请求 Dispatcher 准入。机械臂运动失败或结果不确定时，经拥有设备的适配器停止。 节点状态、拒绝原因和关联 ID 写入共享部署日志。
