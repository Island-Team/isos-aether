# EOS 世界接口

[English](../../../../docs/ei/design/io/world.md) · [南向接口概览](eos-exchange.md)


|---|---|---|
| `robots/{robot}/world/state` | `world.snapshot@1` | 融合实体与关系 |
| `robots/{robot}/detection/state` | `detection.snapshot@1` | 单次模型结果及带类型的 `human:*`、`obj:*` 身份 |
| `robots/{robot}/world/asr/state` | `asr.snapshot@1` | 已识别的外部语音 |


世界与检测含义不同。世界状态可随时间融合观测，检测则保留生产设备/模型来源与 `source_sequence`。检测属性是受 Schema 约束的语义事实；原始张量、掩码、检测框、关键点、嵌入、音频及供应商载荷留在 EOS 内部。

