# 可变数据目录

[English](../../docs/island/data-layout.md) · [共享架构](README_cn.md)

IsOS、EOS 和 EI 各自拥有独立的可变数据目录。源码和便携安装使用 `data/<owner>/`，其中 `<owner>` 为 `isos`、`eos` 或 `ei`。系统安装使用 `/var/lib/island/<owner>/`，并将开机期间有效的套接字和锁放在 `/run/island/<owner>/`。`etc/` 下的配置和 `share/` 下的发行资源不属于可变数据。

| 目录 | 内容 | 运维操作 |
| --- | --- | --- |
| `state/` | 官方持久记录和归属方记忆 | 备份；不要当作缓存清理。 |
| `user/` | 官方记录之外的用户提供或创建的文件 | 备份；仅由用户决定删除。 |
| `attachments/` | 附属于记录或运行的文件 | 与关联记录一起保存。 |
| `cache/` | 可重新生成的下载和派生数据 | 仅在归属方停止后清理。 |
| `log/` | 诊断日志和结构化事件 | 按日志保留策略处理。 |
| `tmp/` | 短期工作文件 | 归属方停止后可清理。 |
| `run/` | 实时套接字、锁和进程文件 | 归属方启动时清理过期条目。 |

仅在需要时创建目录。名为 `cache` 的目录必须可重新生成；保存的图、归属方记忆和用户文件不能放在其中。仓库测试证据保存在被忽略的 `data/test/<run-id>/`；开发工具缓存保存在 `data/cache/tools/`。原先根目录 `logs/test/` 的证据和 `ei/data/ei/` 的运行文件已迁入该目录树。

## 升级源码或便携安装

先停止所有归属方并备份安装目录。在安装根目录运行 `python3 scripts/migrate_var_data.py .`。该工具处理旧 `var/lib/<owner>/` 文件和旧 `ISOS_HOME/data/` 布局。它检查符号链接和特殊文件，仅当现有目标内容一致时才接受它，先复制到临时目标，验证 SHA-256 摘要，再公布副本。旧文件保持原样。中断的复制可能留下 `.migration-pending` 目录；检查并删除该目录后重试。现有缓存、套接字和临时文件不复制；旧 `var/log/` 文件仍保留在旧目录。启动各归属方，确认保存的记录和用户文件可用；确认新安装后再删除旧文件。

如果 IsOS 以前使用 XDG 默认路径而非 `ISOS_HOME`，以实际 XDG 数据根目录运行 `python3 scripts/migrate_var_data.py --xdg "$XDG_DATA_HOME"`；通常默认值是 `~/.local/share`。该命令将旧 `<XDG 数据>/isos/` 记录复制到 `<XDG 数据>/island/isos/`，并将托管 EI 状态复制到 `<XDG 数据>/island/ei/`。

系统部署也应遵循停止、备份、验证和保留旧数据的顺序。确认已安装服务使用标准 `/var/lib/island/<owner>/` 和 `/var/log/island/<owner>/` 根目录后，以有权写入这些目录的身份运行 `python3 scripts/migrate_var_data.py --system /`。该工具把旧归属方文件复制到相应角色目录，把旧归属方日志复制到 `log/`；不会修改 `/var/cache/island` 或 `/run/island`。重试时接受内容一致的副本，遇到内容不同的目标则停止。确认新路径可用后再清理旧文件。使用自定义根目录的安装需按相同映射人工审核复制。
