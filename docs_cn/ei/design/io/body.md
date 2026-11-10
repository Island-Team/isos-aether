# EOS 本体发现

[English](../../../../docs/ei/design/io/body.md) · [南向接口概览](eos-exchange.md)


机器人描述符携带 `id`、`name`、`model`、`description`、运行 `state`、`health`、急停状态、可选电量百分比及机器人作用域原语绑定。`robot.stop`、`safety.estop` 等机器人作用域绑定使用空设备 ID 调用。

每个设备描述符携带 `id`、`class`、`part`、`description`、`state`、`health` 及设备作用域原语绑定。后端名称、驱动版本、端点、原始遥测、配置内部信息和媒体均不跨越该边界。


发现失败不会替换上一份完整代次。成功重新发现会一并移除已消失的设备、资源、技能和原语目标。监控器使用 `GetStatus.started_at` 作为 EOS 进程代次身份，并执行有界重新发现，避免健康状态和绑定长期陈旧。
