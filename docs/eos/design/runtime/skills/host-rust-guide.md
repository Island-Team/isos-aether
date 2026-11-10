# EOS skill host verification

[中文](../../../../../docs_cn/eos/design/runtime/skills/host-rust-guide_cn.md) · [Status](host-rust.md)

Run `cargo test -p aether-runtime --bin aether-runtime --test arm_activation` to verify the built-in skill and real owner/worker socket boundary. The replay covers readiness, two distinct activations, duplicate replay, stale identity rejection, and Stop. It also verifies that the worker accepts a second admitted button ID while rejecting a malformed source. Complete output is saved under ignored `data/test/20261109-eos-arm-demo/`.
