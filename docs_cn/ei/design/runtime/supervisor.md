# 受监督 task 生命周期

[English](../../../../docs/ei/design/runtime/supervisor.md) · [核心总览](modules.md)


## 注册与状态

`SubsystemSpec` 包含唯一名称和接收 `(ready, stop)` 的 async entry。entry 初始化后设置 `ready`，通常运行到 `stop`。`_Runtime` 保存 task、新 ready event、状态、首次 ready 时间、累计 restart/failure 与 last error。输入中的重复名称会在内部映射中折叠，因此 组合层必须保证唯一。

## 启动与就绪


task 可能 set ready 后在 watchdog 下次采样前退出；readiness 可短暂成功，但 `is_healthy()` 还要求所有 task 存活。健康快照保持注册顺序，首次 ready 前 uptime 为零， 负值截为零，并包含 restart 次数和最后异常表示。

## 失败与重启


Watchdog 串行处理 restart，一个 task 的 backoff 会延迟发现其他已结束 task。没有最大 restart、依赖级联、jitter、状态 checkpoint 或可选失败分类；领域 entry 必须自行保证 初始化原子性并恢复持久状态。

## 停止与边界条件

持久化 EI 所有者还为每个选定运行监督一个机械臂监测轮询线程，它独立于仅报告健康状态的工作进程。Run 验证监测路径和关节名称后启动该线程，按配置速率轮询 EOS，并保存一份最新的已验证快照。IsOS 读取者只检查该缓存。EOS 轮询失败时保留上次快照供诊断，但在有效刷新前读取不可用；序号倒退会被拒绝。Stop 等待轮询线程结束并清除缓存，然后才允许新运行。[所有者交换契约](../../interfaces/setup.md)定义响应字段和错误。使用桩的边界验证命令：`uv run --project ei --extra dev pytest ei/tests/ei/test_owner_service.py -q`。

`stop()` 将全部 runtime 标为 `STOPPING`，设置共享 event，取消 watchdog，并以 `return_exceptions=true` gather 子系统。内部没有 graceful timeout，也不会主动取消 entry， 所以不响应 stop 的 entry 会阻塞关闭；gather 异常被消费。backoff 中的 watchdog 会在 respawn 前观察 stop。

重复 `start()`/`stop()` 没有显式生命周期保护，组合层各调用一次。supervisor 本身不 强制 provider-before-consumer 启动或 dependent-before-provider 关闭；这些性质来自组合 方式、entry 的 readiness 协议、共享 stop 和 `core.launch` 的显式资源关闭。
