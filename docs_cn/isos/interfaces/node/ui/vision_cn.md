# IsOS UI 视觉分组

<!-- doc-navigation:start -->
<details>
<summary>目录：isos/interfaces/node/ui</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [IsOS UI 按钮分组](button_cn.md)
  - [IsOS UI 监控分组](monitor_cn.md)

</details>
<!-- doc-navigation:end -->

[English](../../../../../docs/isos/interfaces/node/ui/vision.md) · [节点目录](../README_cn.md) · [视觉流](../../api/client_cn.md#vision)

`isos.ui.vision.monitor@v0.2` 是 `isos/ui/vision/<id>` 路径下由 IsOS 拥有的只读节点。必需的 `frame` 输入类型为 `image_frame@v0.2`，经 `eos.interface.egress.vision_flow@v0.2` 连接兼容的 EOS 相机。相机负责来源选择；EOS 接口节点传输画面。Apply 和 Run 拒绝不完整或未声明路线的路径。

订阅绑定一个活动监控路径、运行 ID、保存修订、传输路径及来源路径。Stop 或任何绑定变化使流失效。[视觉客户端契约](../../api/client_cn.md#vision)定义文本订阅与二进制 JPEG 画面封套。
