# EI reader identity

[中文](../../../docs_cn/ei/interfaces/reader-features.md) · [Setup exchange](setup.md) · [Pipeline](../design/pipeline.md)

The selected `ei-config` identity reports product `0.2.1`, graph `0.2.0`, node type `0.2`, `supported_graph_versions:["0.2.0"]`, `supported_node_type_versions:["0.2"]`, sorted registered stage types, and setup schema `2`. Its features include `setup-v2`, `catalog`, `graph-0.2-ei-projection`, `health-only-owner-v1`, `activation-owner-v1`, and `arm-monitor-owner-v1`.

The selected `ei` runtime probe reports `managed-config-v1`, `eos-gateway-client-v2`, `ei-evidence-scheduler-v1`, `vision-evidence-ingress-v1`, and `vision-inspection-uds-v1`. IsOS compares required features and selected executable bytes before Apply, Run, and Restart. A probe is read-only and cannot grant process or actuator authority.

EI rejects unsupported graph envelopes at `/graph/version`, unknown stage types, invalid placement or routing, and malformed configuration with bounded diagnostics. EOS service versions are checked as independent wire contracts.
