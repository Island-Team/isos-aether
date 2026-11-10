# EI 读取器身份

[English](../../../docs/ei/interfaces/reader-features.md) · [Setup 交换](setup.md) · [阶段管线](../design/pipeline.md)

选定的 `ei-config` 身份报告产品版本 `0.2.1`、图格式 `0.2.0`、节点类型版本 `0.2`、`supported_graph_versions:["0.2.0"]`、`supported_node_type_versions:["0.2"]`、排序后的已注册阶段类型，以及 setup 模式 `2`。其特性包含 `setup-v2`、`catalog`、`graph-0.2-ei-projection`、`health-only-owner-v1`、`activation-owner-v1` 和 `arm-monitor-owner-v1`。

选定的 `ei` 运行时探针报告 `managed-config-v1`、`eos-gateway-client-v2`、`ei-evidence-scheduler-v1`、`vision-evidence-ingress-v1` 和 `vision-inspection-uds-v1`。IsOS 在 Apply、Run 和 Restart 前比较所需特性及选定可执行文件字节。探针只读，不能授予进程或执行器权限。

EI 在 `/graph/version` 拒绝不支持的图 envelope，并以有界诊断拒绝未知阶段类型、无效位置或路线以及畸形配置。EOS 服务版本作为独立线协议契约检查。
