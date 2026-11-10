# 读取器预检设计

[English](../../../../docs/eos/design/runtime/reader-features.md) · [归属方契约](../../../../docs/eos/interfaces/reader-features.md)

EOS 公布每个所选二进制文件接受的图版本、注册节点类型与功能。IsOS 使用这些精确观测决定保存的部署能否运行。profile 工具、runtime 与 config server 从同一个注册表取得内建类型 ID。包贡献类型仅通过传给 setup 的已验证 sidecar 加入；二进制探针不宣称包已经安装。

Validate 与 Apply 时，IsOS 固定保存图的修订，验证包锁定与所选二进制文件，根据图节点和边推导所需功能，并直接调用每个二进制文件。它在归属方渲染或进程变化前检查精确身份、类型 ID 与所需功能。Run 与 Restart 再次检查不可变代和所选二进制文件的哈希。Apply 选择新一代。

运行 `cargo build --workspace` 构建 EOS 二进制文件，再运行 `python3 tests/reader_features_cli.py`。重放检查当前图 `0.2.0` 的探针身份、setup 身份一致性，以及仓库内 mock profile 的验证。完整输出保存在忽略 Git 的 `data/test/<run-id>/`。
