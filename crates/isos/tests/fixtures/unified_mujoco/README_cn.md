# 统一 MuJoCo 图夹具

[English](README.md)

`current_draft.json` 是图格式 `0.2.0` 的草稿，只有一个 `isos` core。它声明已验证的 MuJoCo 进程、EOS 相机、底盘和机械臂适配器、dispatcher 绑定及其类型化边。`unified_mujoco_supervisor_replay.py` 填入私有主目录路径，加入当前 EI 阶段 layer，然后对已链接的 MuJoCo 发行包执行 Validate、公开 Apply、就绪检查、控制、日志、Restart、故障准入及已确认的 Stop。第五个参数传入 `pick-place` 或 `navigation`，分别重放两个软件包场景。软件包锁定为 `island/mujoco-sim@0.1.1`。
