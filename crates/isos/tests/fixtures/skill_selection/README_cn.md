# IsOS 技能选择用例

[English](README.md) · [软件包契约](../../../../../docs_cn/isos/interfaces/packages/process_cn.md) · [Rust 技能契约](../../../../../docs_cn/eos/interfaces/skills/rust_cn.md)

当前选择使用格式为 `0.2.0`、仅包含 `isos` core 的已保存图，以及类型为 `isos.process.component@v0.2` 的 EOS 进程节点。节点的 `settings.skill_executables` 精确固定已验证的 `kind:"skill"` 软件包和 `.skill` 路径。IsOS 将与修订绑定的私有选择写入生成清单，并仅将路径传给 EOS。EOS 验证并暂存每个可执行文件后才发布目录。

[`cases.json`](cases.json)、[`preflight_cases.json`](preflight_cases.json) 和 [`retirement_cases.json`](retirement_cases.json) 描述图、软件包、探测及 Stop 的结果。JSON 示例中的摘要和路径是合成值；运行时测试生成真实收据和私有文件。校验脚本检查用例表与示例：

```sh
python3 crates/isos/tests/fixtures/skill_selection/check_cases.py
python3 crates/isos/tests/fixtures/skill_selection/preflight_cases.py
```

所有者测试覆盖修订绑定的生成、软件包变更拒绝、仅向 EOS 传递选择、候选预检及已确认 Stop。`crates/aether-runtime/tests/` 中的工作进程和目录测试覆盖已验证暂存与有界生命周期。将完整集成回放输出和准确的二进制修订记录到忽略的 `data/test/<run-id>/`。
