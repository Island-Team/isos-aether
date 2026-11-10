# EI 阶段图夹具

[English](README.md)

`valid_ei.json` 包含图格式 `0.2.0` 中注册的七个 EI 阶段节点及其本地证据边。图只有一个 `isos` core，使用 `ei_*` layer 和当前 `isos.ei_*.*@v0.2` 节点类型 ID。gateway 端口为 `16200`，与所选 EOS gRPC 端点一致。

`unified_owner_replay_cli.py` 将这些 layer 加入全新的 IsOS starter 图，然后执行真实 EI 和 EOS 归属方验证、绑定修订的渲染，以及不改变保存图的拒绝测试。构建所选二进制文件并安装 EI Python 依赖后，运行 `PYTHONPATH=crates/isos/tests python3 -m unittest -q unified_owner_replay_cli`。
