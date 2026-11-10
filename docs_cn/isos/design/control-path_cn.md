# 模拟机械臂控制与监控路径

当前 16 节点[演示图](../../../config/graph-pool/demo.nodes.json)围绕同一模拟机械臂声明两条路径。每条连接使用注册端口和准确数据类型。IsOS 保存并准入图；EI 和 EOS 各自拥有其运行阶段。

起始图将按钮命名为 **Forward: move arm**，将读取节点命名为 **Return: arm state**。监控设置明确指定 EI 以 10 Hz 刷新，UI 以 5 Hz 刷新。在新画布中，控制链位于上方，从左向右；观测链位于下方，从右向左。两条路径的排布仅属于 UI 布局；拖动节点后，本地会保留自定义位置。两条链之间没有连接。EOS 在本地为返回链读取同一个模拟机械臂采样器，不创建第二条执行器路径。

## 控制

`isos/ui/button/trigger` → `ei/io/input/trigger` → `ei/cognition/intent/forwarder` → `ei/governance/safety/gate` → `eos/interface/ingress/trigger` → `eos/skill/demo/joint_delta` → `eos/dispatcher/arm/delta` → `eos/hardware/mock/arm`。

按钮发送带运行围栏的 `button_activation@v0.2`，不包含运动目标。EI 验证并传递激活事件。EOS 生成有界 `arm_joint_delta@v0.2`，经 Dispatcher 准入后更新模拟适配器。保存的图不能创建第二条执行器路径。

## 观测

独立观测源 `eos/hardware/mock/arm_observer` → `eos/dispatcher/arm/monitor` → `eos/skill/demo/arm_status` → `eos/interface/egress/arm_status` → `ei/io/input/arm_status` → `ei/cognition/state/arm_status` → `ei/governance/monitor/arm_status` → `isos/ui/monitor/arm`。

这条路径传递 `arm_monitor_state@v0.2`。EOS 在本地以 100 Hz 采样并检查机械臂，由 EI 以最高 20 Hz 轮询最新状态；UI 以最高 5 Hz 刷新。EI 和 UI 的轮询独立于 EOS 本地安全检查；UI 读取缓慢不会阻塞控制路径。只有 EOS 根据过期或无效的本地采样阻止运动。Stop 和运行切换会清空显示的位置。

2026-11-10 的双链检查遍历了已保存的图库文件，确认有 16 个节点、14 条连接和两个各含 8 个节点的连通分量。`ISOS_AETHER_BIN_DIR=$PWD/target/debug cargo test -p isos --no-fail-fast` 通过（284 个单元测试和 18 个集成测试）；`cargo fmt --all -- --check` 与 `python3 scripts/check_doc_links.py --all` 通过。IsOS 完整测试输出保存在忽略目录 `data/test/async-two-chain-20261110/`。

[按钮事件](../interfaces/ui/node-events_cn.md)、[监控状态](../interfaces/ui/node-monitor_cn.md)与[数据类型](../interfaces/node/data-types_cn.md)定义外部边界。

[English](../../../docs/isos/design/control-path.md)
