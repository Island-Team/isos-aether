# link 节点设计

<!-- doc-navigation:start -->
<details>
<summary>目录：ei/interfaces/node/design/io</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [io / input 节点组](input.md)

</details>
<!-- doc-navigation:end -->

[English](../../../../../../docs/ei/interfaces/node/design/io/link.md)

## 注册节点行为

**输入：** gateway；admitted 与 emergency_stop 为受治理的内部路径。**输出：** discovery、resource、result、health。[阶段数据流设计](../../../../core/runtime/pipeline.md)描述 EI 运行值。图端口、设置及连线优先级和频率的契约归 IsOS 所有。

**算法与故障：** 解析 IsOS 提供的端点，发现资源与能力，验证精确身份后原子发布代际。断线时标记代际过期并阻止新动作；依赖阶段停止后关闭通道。


## ei.io.link.gateway@v0.2

接收所有者注入的资源包与健康更新。 [节点接口](../../io/link_cn.md#io_link_cn--gateway)
