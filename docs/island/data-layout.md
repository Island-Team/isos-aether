# Mutable data layout

[中文](../../docs_cn/island/data-layout_cn.md) · [Shared architecture](README.md)

IsOS, EOS, and EI each own a separate mutable directory. Source and portable installations use `data/<owner>/`, where `<owner>` is `isos`, `eos`, or `ei`. System installations use `/var/lib/island/<owner>/` and place boot-lifetime sockets and locks under `/run/island/<owner>/`. Configuration under `etc/` and shipped resources under `share/` are separate from mutable data.

| Folder | What belongs here | Operator action |
| --- | --- | --- |
| `state/` | Official durable records and owner memory | Back up; do not clear as a cache. |
| `user/` | User-supplied or user-created files outside official records | Back up; delete only by user choice. |
| `attachments/` | Files attached to records or runs | Preserve with the related records. |
| `cache/` | Downloads and derived data that can be rebuilt | Clear only while the owner is stopped. |
| `log/` | Diagnostic and structured events | Retain according to the log policy. |
| `tmp/` | Short-lived work files | Clear after the owner stops. |
| `run/` | Live sockets, locks, and process files | The owner removes stale entries during startup. |

Folders are created only when needed. A directory named `cache` is always rebuildable; never put saved graphs, owner memory, or user files there. Repository test evidence lives under ignored `data/test/<run-id>/`; developer tool caches live under `data/cache/tools/`. The earlier root `logs/test/` evidence and `ei/data/ei/` runtime files were moved into this tree.

## Upgrading a source or portable installation

Stop all owners and back up the installation before migrating. From the installation root, run `python3 scripts/migrate_var_data.py .`. The utility handles legacy `var/lib/<owner>/` files and the old `ISOS_HOME/data/` layout. It checks for symlinks and special files, accepts an existing destination only when its contents match, copies durable files to temporary destinations, verifies their SHA-256 digests, and publishes the copy. It leaves the old files intact. An interrupted copy can leave a `.migration-pending` directory; inspect and remove that directory before retrying. Existing caches, sockets, and temporary files are not copied; old `var/log/` files remain available in the old tree. Start the owners, verify saved records and user files, then remove old files only when the new installation is confirmed.

If IsOS used its former XDG default instead of `ISOS_HOME`, run `python3 scripts/migrate_var_data.py --xdg "$XDG_DATA_HOME"` with the actual XDG data root; the usual default is `~/.local/share`. This copies old `<XDG data>/isos/` records into `<XDG data>/island/isos/` and managed EI state into `<XDG data>/island/ei/`.

System deployments need the same stop, backup, verification, and retention sequence. After confirming the installed service uses the standard `/var/lib/island/<owner>/` and `/var/log/island/<owner>/` roots, run `python3 scripts/migrate_var_data.py --system /` with permission to write those directories. It copies old owner files into their role folders and old owner logs into `log/`; it does not alter `/var/cache/island` or `/run/island`. Existing matching copies are accepted on retry, while differing destinations stop migration. Verify the new locations before retiring old files. Installations with custom roots require an operator-reviewed copy using the same folder mapping.
