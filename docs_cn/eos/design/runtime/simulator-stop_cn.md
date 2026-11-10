# EOS 模拟器 Stop 设计

[English](../../../../docs/eos/design/runtime/simulator-stop.md) · [所有者契约](../../interfaces/simulator-stop_cn.md) · [硬件](hardware_cn.md) · [进程监督](process_cn.md) · [用例](../../../../tests/fixtures/simulator_stop/README_cn.md)

EOS 直接管理 MuJoCo 子进程，Dispatcher 是唯一物理执行器路线。IsOS 请求 Stop 并等待 EOS 的选定终态凭据。图修订或进程运行发生变化时，先前命令被围栏阻止。

## 关闭与物理围栏

运行依次经历 `Running → Closing → Stopping → Terminal`。进入 `Closing` 会阻止新运动并关闭公开和私有运动入口。Dispatcher 记录已准入的底盘和机械臂目标。每代子进程只有一个 `ManagedMujocoStopCoordinator`；受控 Stop、紧急 Stop 和关闭共用其配套安全路线。配套请求推进一个引擎 epoch，并同时返回底盘和机械臂结果。同时发生的 Stop ID 保留各自 journal 身份，但共用物理请求。超时、格式错误的响应、部件失败或持久性凭据丢失均归为不确定，并保持运动抑制。

EOS 在允许受管理运动前验证主 socket、配套 socket 和选定代。配套安全路线独立于普通运动及渲染工作。EOS 在向 MuJoCo 发送 TERM 或 KILL 前先分类物理结果，随后排空子进程组、核算后代进程、移除 socket，并通过选定管道发出一份有界终态凭据。仅回收主进程不能证明进程组已排空。强制清理不能把不确定的物理结果升级为已确认 Stop。

所有者截止时间有界：先关闭准入，再分类物理 Stop，之后终止子进程并排空进程组，最终分类完成后才写终态凭据。IsOS 将缺失或不确定的凭据视为降级，并退休该运行的授权。[模拟器 Stop 契约](../../interfaces/simulator-stop_cn.md)规定精确结果字段及时间界限。

## 验证

使用已验证的 `island/mujoco-sim@0.1.1` 包，重放有效的底盘与机械臂命令后执行 Stop。检查一次物理停止 epoch、两个部件结果、进程组排空、socket 移除及选定管道终态凭据。再测试普通运动阻塞、配套 socket 丢失、响应损坏、MuJoCo 死亡、EOS 死亡及后代进程滞留；每种失败都应保持不确定或降级，并禁止新运动释放。运行[用例检查](../../../../tests/fixtures/simulator_stop/README_cn.md)、串行 Rust 测试、格式化、严格 Clippy 和双语链接检查。把完整输出保存在 `data/test/<run-id>/`。
