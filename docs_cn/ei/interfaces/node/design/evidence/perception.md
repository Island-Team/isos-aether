# perception 节点设计

<!-- doc-navigation:start -->
<details>
<summary>目录：ei/interfaces/node/design/evidence</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)

</details>
<!-- doc-navigation:end -->

[English](../../../../../../docs/ei/interfaces/node/design/evidence/perception.md)

## 注册节点行为

输入为 `resource`、`result` 和可选 `hearing`，输出为 `observations`。感知验证来源身份与事件结构，保持事件顺序，按键合并可替换状态并计数省略。错误证据被拒绝并记录，不能变成空成功。[阶段数据流](../../../../core/runtime/pipeline.md)描述 EI 内部值；图端口和设置由 IsOS 定义。


## ei.evidence.perception.admission@v0.2

将已验证的资源包接纳为认知层观测。 [节点接口](../../evidence/perception_cn.md#evidence_perception_cn--admission)
