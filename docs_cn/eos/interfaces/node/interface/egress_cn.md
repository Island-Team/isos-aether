<a id="interface_egress_cn"></a>
<a id="interface_egress_cn--interfaceegress-分组"></a>
# interface/egress 分组

<!-- doc-navigation:start -->
<details>
<summary>目录：eos/interfaces/node/interface</summary>

浏览本目录的文档及下级目录。

- [上级目录](../README_cn.md)
- 同级文档：
  - [interface/ingress 分组](ingress_cn.md)

</details>
<!-- doc-navigation:end -->

[节点目录](../README_cn.md) · [English](../../../../../docs/eos/interfaces/node/interface/egress.md#interface_egress)

此分组包含以下 EOS 节点类型。各节点的公共契约见本页下文。

- [arm_status](egress_cn.md#interface_egress_cn--arm-status)
- [vision](egress_cn.md#interface_egress_cn--vision)
- [vision_flow](egress_cn.md#interface_egress_cn--vision-flow)

<a id="interface_egress_cn--arm-status"></a>
<a id="arm-status"></a>

<a id="interface_egress_cn--eosinterfaceegressarmstatusv02"></a>
## eos.interface.egress.arm_status@v0.2

当前 IsOS 节点库中的内置节点类型。

<a id="interface_egress_cn--节点作用"></a>
### 节点作用

向 EI 发布已检查的机械臂状态。

- 输入：`observation`：`arm_monitor_state@v0.2`，进程内。
- 输出：`observation`：`arm_monitor_state@v0.2`，EI Unix socket。

<a id="interface_egress_cn--所有者与生命周期"></a>
### 所有者与生命周期

桥接发布有界且受运行围栏约束的观测；Stop 结束实时发布。

准确载荷字段及拒绝规则见本页所链接的 EOS 所有者契约。

参见 [arm-monitor_cn](../../../interfaces_cn.md#arm_monitor_cn)。

<a id="interface_egress_cn--vision"></a>
<a id="vision"></a>

<a id="interface_egress_cn--eos-v2-vision-接口"></a>
## EOS v2 Vision 接口

<a id="interface_egress_cn--图中的-vision-传输图-021"></a>
### 图中的 Vision 传输（图 0.2.1）

`eos.interface.egress.vision_flow@v0.2` 是 EOS 拥有的标准接口节点。其必需的 `frame` 输入接收已声明相机设备的 `image_frame@v0.2` 进程内事件，数据类为 `image`；`frame` 输出通过受运行身份约束的 Unix socket 向 IsOS UI Vision 监视节点提供相同类型。相机节点选择服务与设备来源。Apply 和 Run 验证相机到传输的连线。

EOS 所有者只对活动运行、修订和精确的传输到相机路线准入 `vision_subscribe`，并返回该运行的工作进程套接字及独立的 64 位十六进制准入令牌。IsOS 在该套接字上发送 `{op:"vision_subscribe",run_id,revision,node,vision_token}`。原生工作进程对每次新采集推送 `[u32 大端 JSON 头部长度][JSON 头部][u32 大端 JPEG 长度][JPEG 字节]`。头部包含运行、修订、来源路径、输出序列、样本、尺寸和 `encoding:"jpeg"`。头部和 JPEG 上限分别为 4096 字节与 20 MiB；工作进程最多允许四个订阅者，部分写入超过 150 ms 时断开慢速订阅者。Stop 关闭工作进程及订阅；推送画面时控制套接字仍可用。

来源输出端口声明预期频率。目标输入端口可以声明期望范围；处理节点自行声明其输出频率。图 Vision 传输没有独立的默认采集或渲染限速。载荷上限与运行围栏继续生效。

当前推送数据通道仅接收 JPEG 画面。演示图中的 MuJoCo 相机按原生 1280×720 设置输出 JPEG。其他 `image_frame@v0.2` 编码仍属于该数据类型，但不会通过此二进制 Vision 订阅交付。

<a id="interface_egress_cn--vision-flow"></a>
<a id="vision-flow"></a>

<a id="interface_egress_cn--eosinterfaceegressvisionflowv02"></a>
## eos.interface.egress.vision_flow@v0.2

当前 IsOS 节点库中的内置节点类型。

<a id="interface_egress_cn--节点作用-1"></a>
### 节点作用

将相机帧传至 IsOS Vision。

- 输入：`frame`：`image_frame@v0.2`，进程内事件。
- 输出：`frame`：`image_frame@v0.2`，Unix socket 流。

<a id="interface_egress_cn--所有者与生命周期-1"></a>
### 所有者与生命周期

只有图中声明的相机来源可输入此桥接。EOS 校验每帧并绑定当前运行；Stop 结束流。

准确载荷字段及拒绝规则见本页所链接的 EOS 所有者契约。

参见 [vision_cn](egress_cn.md#interface_egress_cn--eosinterfaceegressvisionflowv02)。
