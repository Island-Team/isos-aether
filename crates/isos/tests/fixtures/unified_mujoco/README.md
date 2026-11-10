# Unified MuJoCo graph fixture

[中文](README_cn.md)

`current_draft.json` is a graph format `0.2.0` draft with one `isos` core. It declares the verified MuJoCo process, EOS camera, base and manipulator adapters, dispatcher bindings, and their typed edges. `unified_mujoco_supervisor_replay.py` fills the private home path, adds the current EI stage layers, then exercises Validate, public Apply, readiness, control, logs, Restart, failure admission, and confirmed Stop against the linked MuJoCo release. Pass `pick-place` or `navigation` as the fifth argument to replay each package scene. The package pin is `island/mujoco-sim@0.1.1`.
