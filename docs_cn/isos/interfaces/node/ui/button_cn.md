# IsOS UI 按钮分组

<!-- doc-navigation:start -->
<details>
<summary>目录：isos/interfaces/node/ui</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [IsOS UI 监控分组](monitor_cn.md)
  - [IsOS UI 视觉分组](vision_cn.md)

</details>
<!-- doc-navigation:end -->

[English](../../../../../docs/isos/interfaces/node/ui/button.md) · [节点目录](../README_cn.md) · [客户端事件请求](../../api/client_cn.md#button)

`isos.ui.button.trigger@v0.2` 是 `isos/ui/button/<id>` 路径下由 IsOS 拥有的输入节点。`activation` 输出以事件方式携带 `button_activation@v0.2`。载荷包含新的 `event_id` 与 `kind:"activate"`，不含运动目标。节点没有设置。编写的连线须经过已声明的 EI 安全路径和 EOS Dispatcher 路线，运动才可能获准。

这是 UI→EI 图端口，通过带运行围栏的 IsOS 到 EI Unix 套接字事件私有传输。独立的 UI→EOS 操作员控制通过 EOS 回环桥接提交类型化原语请求，并由 EOS Dispatcher 准入。该请求是与选定原语绑定关联的控制端点，不是可执行图边，也不是此按钮的另一输出。

IsOS 仅对活动运行和精确的保存修订接受激活。同一事件 ID 重复提交时返回已记录状态，不会再次运动。客户端使用同一事件 ID 单独读取最终结果。无效路径、过期身份及缺失路线在交付所有者前被拒绝。[事件 API](../../api/client_cn.md#button)定义准确的请求、响应和错误码。
