# EOS 演示机械臂激活

[English](../../../docs/eos/interfaces/arm-control.md) · [EOS 接口](README_cn.md) · [硬件设计](../design/runtime/hardware_cn.md)

## 概览

EI 通过私有 Unix socket 调用 EOS 所有者的 `POST /v1/activate`。所有者检查当前运行，并把事件转发给其直接子进程。EOS 接口节点把激活传给内置演示技能；技能产生六关节增量，Dispatcher 准入后交给模拟机械臂适配器。浏览器和后端包不能直接调用适配器。

## 认证

所有者 socket 仅本地可用，权限为 `0600`。IsOS 控制启动、运行 ID、修订版本及 EI 到 EOS 的连接。EOS 所有者要求与当前运行完全一致的运行 ID 和修订版本。工作进程 socket 同样只在本地使用，权限为 `0600`。Stop 在子进程退出之前关闭准入。

## 接口

`button_activation@v0.2` 为 JSON 对象，必填 `event_id`（1–64 个 ASCII 字母、数字、`_`、`-` 或 `.`）及值为 `activate` 的 `kind`。`arm_joint_delta@v0.2` 为包含 `joint_deltas_rad` 的 JSON 对象，恰好六个有限数值，单位为弧度。内置技能产生 `[0.05, -0.04, 0.03, -0.02, 0.01, -0.01]`。每次增量的绝对值不超过 0.25 rad；Dispatcher 拒绝让模拟关节超出图中 `max_joint_position_rad` 或 ±3.14 rad（取较小值）的结果。模拟机械臂从六个零值开始，在同一次工作进程运行期间保留位置。其[观测返回路径](arm-monitor_cn.md)持续采样，遇到过期或无效样本时关闭运动准入。

四个控制图节点为 `eos.interface.ingress.trigger@v0.2`、`eos.skill.demo.joint_delta@v0.2`、`eos.dispatcher.arm.delta@v0.2` 和 `eos.hardware.mock.arm@v0.2`。入口输入使用 `ei/activation-to-eos/1` Unix socket 桥接；入口输出与技能输入使用进程内的 `eos/activation-local/1`；其余边使用进程内的 `eos/arm-delta/1`。所有端口使用事件传递；激活归类为事件数据，增量归类为命令数据。模拟节点是机械臂适配器，具备已注册的 `mock_arm` 安全适配器和 `joint_positions` 能力。设置目录公开已编译的 `mock-arm` 适配器族和 `arm.delta@1.0.0` 原语。

## 路由

## POST /v1/activate

EI 发送 JSON 正文，字段必须恰好为 `schema_version`（`1`）、`core`（`eos`）、`op`（`activate`）、`run_id`、`revision`（64 个小写十六进制字符）、`source_node`（`isos/ui/button/<id>`，ID 为 1–48 个 ASCII 字符）、`event_id` 和 `kind`（`activate`）。EOS 信任 EI 已根据完整图准入该来源，并检查其仍为规范的 IsOS 按钮路径。未知或无效字段会被拒绝。每个事件 ID 在运行期日志中同步保留后才返回 `accepted`；单执行器工作线程随后处理运动。同一次运行中重复的 ID 返回同一接收回执，不再次移动。成功返回 HTTP 200，拒绝返回 422。传输结果不确定时报告 `outcome_unknown`，不得使用新的事件 ID 自动重试。

```json
{
  "schema_version": 1,
  "core": "eos",
  "op": "activate",
  "run_id": "run_1",
  "revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "source_node": "isos/ui/button/trigger",
  "event_id": "click-1",
  "kind": "activate"
}
```

```json
{
  "schema_version": 1,
  "core": "eos",
  "ok": true,
  "run_id": "run_1",
  "revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "event_id": "click-1",
  "command_id": "arm-click-1",
  "status": "accepted"
}
```

拒绝的请求包含 `ok:false` 和 `error` 代码：`invalid_run_identity`、`stale_or_missing_run`、`invalid_activation`、`worker_unavailable`、`outcome_unknown`、`joint_limit` 或 `event_capacity`。拒绝响应不包含关节数组。工作进程每次运行最多保留 4096 个已保留事件 ID；达到上限后仍接受已知的重复 ID，但拒绝新 ID 且不会运动。工作进程不可用或结果不确定时返回 `503`，未知路由返回 `404`，正文超过 1 MiB 返回 `413`；工作进程把转发消息限制为 4 KiB。v1 路由和节点、数据的 `@v0.2` 标识分别独立演进。

## POST /v1/result

```json
{
  "schema_version": 1,
  "core": "eos",
  "op": "result",
  "run_id": "run_1",
  "revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "event_id": "click-1"
}
```

EI 用恰好包含 `schema_version`（`1`）、`core`（`eos`）、`op`（`result`）、当前 `run_id`、`revision` 和 `event_id` 的正文读取相关结果。成功响应包含相同运行标识、`command_id` 和 `status`，状态为 `accepted`、`running`、`succeeded`、`failed`、`cancelled` 或 `outcome_unknown`；仅成功状态包含 `joint_positions_rad`。未知 ID 返回 `event_not_found`，停止或旧运行返回 `stale_or_missing_run`。EOS 监控采样独立于结果读取，慢读取者只取得最新观测。

工作进程退出后，所有者从运行日志读取 `/v1/result`。仅有保留或运行记录、没有终态记录时返回 `outcome_unknown`。使用同一运行日志重启时，工作进程恢复终态记录及位置，将中断命令标记为不确定，并在存在不确定结果时禁止进一步运动。执行队列受每次运行 4096 个事件的上限约束；重复 ID 不再入队。
