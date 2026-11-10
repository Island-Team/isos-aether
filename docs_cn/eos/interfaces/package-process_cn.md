# 向 EOS 移交软件包进程

> 历史契约：当前按钮到模拟机械臂图未实现此接口。当前数据及命令路径参见[机械臂控制](arm-control_cn.md)；设置、Run、Status 和 Stop 参见[所有者生命周期](owner-health_cn.md)。


[English](../../../docs/eos/interfaces/package-process.md) · [接口目录](README_cn.md) · [MuJoCo 适配器](mujoco_cn.md) · [JSONL 事件](events_cn.md)


```json
{
  "schema_version": 1,
  "node_path": "isos/eos_process/mujoco/simulator",
  "node_type": "isos.eos_process.mujoco@v0.2",
  "package": {
    "id": "island/mujoco-sim",
    "version": "0.1.1"
  },
  "entrypoint": "/var/lib/island/isos/state/packages/island/mujoco-sim/0.1.1/bin/simulator",
  "resources": {
    "scene": "/var/lib/island/isos/state/packages/island/mujoco-sim/0.1.1/assets/scene.xml"
  },
  "lifecycle_adapter": "mujoco-v2",
  "release_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "payload_sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
}
```

`schema_version` 是无单位整数，仅接受 `1`，无默认值。`node_path` 是非空图路径字符串，`node_type` 是非空注册类型字符串；两者都必须与所选图节点一致。`package.id` 和 `package.version` 是无单位非空字符串，必须与节点的精确软件包固定版本一致。`entrypoint` 是无单位绝对路径字符串，须为可执行的常规文件。`resources` 是资源名到无单位绝对路径字符串的映射；所选适配器定义允许的名称并验证文件。当前 `mujoco-v2` 适配器仅接受 `scene`，它须为与所选场景一致的常规文件。IsOS 只在验证软件包与所选资源后解析路径；EOS 在启动前再次验证。`lifecycle_adapter` 是无单位字符串，指定 EOS 适配器；v1 仅注册 `mujoco-v2`，它验证 MuJoCo 设备、套接字和场景绑定。IsOS 须在 Apply 或 Run 前拒绝其他适配器。`release_sha256` 和 `payload_sha256` 是无单位、64 字符小写 SHA-256 十六进制字符串；IsOS 从已验证的回执提供，EOS 在 `package_admitted` 事件中携带。所有字段必填；拒绝未知顶层字段；JSON 文件最多 64 KiB；缺失或无效数据会在启动子进程前使启动失败。后续 schema 版本及适配器系列需要显式读取器支持。

IsOS 通过 `--run-id` 提供运行 ID，并通过 `--log-dir` 提供 EOS 日志目录。软件包子进程的 stdout 和 stderr 写入 `package-process.log`；它们是原始输出，不是权威 EOS 事件。EOS 在自身 stderr 上按照[事件契约](events_cn.md)输出有界 JSONL 生命周期与失败事件。IsOS 验证生产者和运行身份后才将事件加入运行合并日志。就绪需要所选适配器的实时健康证明。停止时，EOS 先阻止已准入执行器并取得物理停止证明，再终止和回收子进程。缺少证明时终态为 unknown，不能报告为 confirmed。
