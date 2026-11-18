# IsOS UI 监控分组

<!-- doc-navigation:start -->
<details>
<summary>目录：isos/interfaces/node/ui</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [IsOS UI 按钮分组](button_cn.md)
  - [IsOS UI 视觉分组](vision_cn.md)

</details>
<!-- doc-navigation:end -->

[English](../../../../../docs/isos/interfaces/node/ui/monitor.md) · [节点目录](../README_cn.md) · [客户端监控](../../api/client_cn.md#monitor)

`isos.ui.monitor.arm@v0.2` 是 `isos/ui/monitor/<id>` 路径下由 IsOS 拥有的只读节点。必需的 `observation` 输入经已声明的 EOS 观测与 EI 治理路径携带 `arm_monitor_state@v0.2`。它不产生执行器命令，也不根据已提交的按钮事件推断物理动作成功。

读取要求活动运行、保存修订及完整的已准入返回链。就绪观测包含六个有限的弧度关节位置；降级、过期和已停止观测不含位置并给出原因。[监控客户端契约](../../api/client_cn.md#monitor)定义读取路线、新鲜度和失败状态。
