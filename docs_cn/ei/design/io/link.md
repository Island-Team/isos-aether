
[English](../../../../docs/ei/design/io/link.md)

English · 核心概览 · EOS 2 operation 示例

Link 为每个配置的稳定 robot alias 管理一个 EOS gRPC client。它发布完整的机身与资源 generation，读取并 watch typed world resource，按需捕获 camera frame，执行有版本的 primitive，并提供 monitor 使用的直接 stop 路径。EOS driver 和 provider 保持为内部实现。

## Resource 与 camera capture

ListResources 提供支持的 typed world、detection 和 speech resource。WatchResource 通知变化的 URI；Link 在初始 read 前先启动 watch，收到通知时重新读取，并定时有界重读以覆盖丢失的通知。只发布通过 schema 校验的 snapshot。Camera frame 通过 CaptureFrames 按需获取。

## Primitive operation



Link 将 rejected、failed、cancelled、timed out 和 outcome unknown acknowledgement 作为非成功返回。Submission RPC 成功本身不代表物理 action 已完成。若 watch 在终态前结束，或响应 identity 不一致，Link 会抛出 transport/contract error。

仅对可取消的 action class 使用原 operation ID 显式调用 CancelOperation。Link 最多等待五秒到达 terminal cancellation boundary。超时、断连或 OUTCOME_UNKNOWN 后不会自动重新提交。此时需要基于新状态 reconciliation 或操作员介入。

Submit 响应或 Watch 流丢失时，Link 使用同一个 grant 和 ID 查询 GetOperation。只有规范且匹配的终态结果才能确定该调用；查询失败使结果保持不确定，并封锁后续写入直到重新发现。Stop 与 emergency 使用固定的 safety/monitor 路径；图中的 stage 边只承载证据，不能命令 gateway。

## Transport 状态

初始化前，依赖 discovery 的 property 会抛错，而不会返回看似权威的空数据。强制 initialize 会原子替换完整 generation。未知 resource、version、target、camera 和 skill 均 fail closed。Link error 会保留 gRPC status 与 detail，由数据源 owner 负责 stream reconnection，关闭时释放 channel。

**相关页面：** 南向数据流、registry、safety 和 EOS 兼容记录。

## 注册节点行为

**输入：** gateway；admitted 与 emergency_stop 为受治理的内部路径。**输出：** discovery、resource、result、health。[阶段数据流设计](../pipeline.md)描述 EI 运行值。图端口、设置及连线优先级和频率的契约归 IsOS 所有。

**算法与故障：** 解析 IsOS 提供的端点，发现资源与能力，验证精确身份后原子发布代际。断线时标记代际过期并阻止新动作；依赖阶段停止后关闭通道。
