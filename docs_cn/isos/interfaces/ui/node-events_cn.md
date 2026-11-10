# 按钮激活事件

[English](../../../../docs/isos/interfaces/ui/node-events.md) · [节点类型](../node/README_cn.md)

IsosUI 通过 `POST /api/isos/v1/nodes/events` 将注册按钮节点的一次通用激活发送给 IsOS。IsOS 在转发到私有 EI 所有者之前验证本地 Host 和 Origin、活动运行、已保存修订以及经过 EI 和 EOS 的准确声明路径。浏览器不能选择机械臂、关节目标、所有者地址或执行器授权。

```json
{
  "schema_version": "0.2.0",
  "source_node": "isos/ui/button/trigger",
  "output": "activation",
  "data_type": "button_activation@v0.2",
  "run_id": "run-1",
  "active_revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "payload": { "event_id": "click-1", "kind": "activate" }
}
```

所有字段必填，未知字段会被拒绝。`source_node` 有四个 ASCII 路径段，每段最多 48 字节，只使用字母、数字、`_` 或 `-`。`event_id` 为 1–64 个 ASCII 字母、数字、`_`、`-` 或 `.`；`run_id` 使用相同语法，最多 128 字节。修订为 64 个小写十六进制字符。请求体最多 2048 字节。不同点击使用新事件 ID；结果不确定的点击保留原 ID。

节点图必须有一条获准路径，从按钮经过 EI 输入、认知、安全，再到 EOS 入口、技能、调度器和模拟机械臂。IsOS 在 EI 所有者套接字上发送 `{core:"ei",op:"activate",run_id,revision,source_node,event_id,kind}`。EOS 预留命令后，成功提交返回事件 ID、命令 ID 和 `accepted` 状态。重复 ID 返回已记录状态，不会再次运动。浏览器的提交请求不等待机械臂运动完成。

`POST /api/isos/v1/nodes/events/result` 读取关联结果，不会重新提交运动。准确请求是 `{schema_version:"0.2.0",source_node,run_id,active_revision,event_id}`，身份与提交相同。IsOS 验证活动运行、已保存节点图和声明路径，向 EI 查询结果，并在回复前重新检查运行。响应包含 `event_id`、`command_id` 和 `status`：`accepted`、`running`、`succeeded`、`failed`、`cancelled` 或 `outcome_unknown`。只有 `succeeded` 包含六个有限的 `joint_positions_rad` 值。IsosUI 读取此接口，直到得到终态或达到有界等待时间；后续查询继续使用相同事件 ID。

提交字段无效返回 `400 node_event_invalid`，请求体过大返回 `413 node_event_oversized`，运行或图过期返回 `409 run_fence_stale` 或 `graph_fence_stale`，路径无效返回 `422 node_route_invalid` 或 `node_edges_invalid`。结果查询无效时按情况返回 `400 node_result_invalid`、`413 node_result_oversized`、`422 node_route_invalid` 或 `503 owner_result_unavailable`、`owner_result_invalid`。所有者传输故障或提交结果不确定时返回 `503 owner_outcome_unknown`；考虑再次提交前，使用同一事件 ID 查询结果。Stop 在工作进程退出前关闭准入。
