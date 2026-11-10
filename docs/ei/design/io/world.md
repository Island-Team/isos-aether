# EOS world interface

[中文](../../../../docs_cn/ei/design/io/world.md) · [Southbound overview](eos-exchange.md)


|---|---|---|
| `robots/{robot}/world/state` | `world.snapshot@1` | fused entities and relations |
| `robots/{robot}/detection/state` | `detection.snapshot@1` | one model result with typed `human:*` and `obj:*` identities |
| `robots/{robot}/world/asr/state` | `asr.snapshot@1` | recognized external speech |


World and detection are distinct. World state may fuse observations over time, while detection retains the producing device/model source and `source_sequence`. Detection attributes are schema-bounded semantic facts; raw tensors, masks, boxes, landmarks, embeddings, audio, and provider payloads remain inside EOS.

