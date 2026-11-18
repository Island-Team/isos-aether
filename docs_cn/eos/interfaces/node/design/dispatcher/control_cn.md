# 共享控制调度器节点

<!-- doc-navigation:start -->
<details>
<summary>目录：eos/interfaces/node/design/dispatcher</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [dispatcher/arm 分组](arm_cn.md)
  - [底盘原语绑定节点](base_cn.md)
  - [机械臂原语绑定节点](manipulator_cn.md)

</details>
<!-- doc-navigation:end -->

[English](../../../../../../docs/eos/interfaces/node/design/dispatcher/control.md)


<a id="共享控制调度器节点"></a>

## 共享控制调度器节点

**位置：**`isos/eos_dispatcher/control/<instance-id>` · **类型族：**`isos.eos_dispatcher.control@v0.2`。注册类型是可复用定义；部署时选择实例 ID 和非空实例说明。

**状态：** 历史 `isos.eos_*` 设计参考。此类型不是当前内置 EOS 节点库条目。

### 用途与边界

唯一的执行器准入方在后端执行前检查身份、健康、新鲜度、期限、限值和安全状态。图节点表示该准入方及其类型化路径，不创建第二个调度器。

### 端口与设置

输入：`request: ControlRequest`。输出：`base_command: BaseMovement`、`manipulator_command: ManipulatorMovement`、`status: Status`。所属接口页定义精确字段类型、单位、默认值、校验和示例。注册类型须有非空 `NodeType.description`。

### 生命周期与算法

规范化请求，拒绝无效或过期任务，保留命令身份，记录准入，只执行一次，然后观测或停止。路由输入上的优先级只影响交付顺序，不绕过这些检查。 节点状态、拒绝原因和关联 ID 写入共享部署日志。
