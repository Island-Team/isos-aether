#!/usr/bin/env bash
# Emergency fallback for this checkout. Bypasses owner Stop and physical-stop proof.
set -euo pipefail

ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
exec python3 - "$ROOT" "$@" <<'PY'
import os
import pathlib
import signal
import sys
import time

root = pathlib.Path(sys.argv[1]).resolve()
args = sys.argv[2:]
if args not in ([], ["--dry-run"]):
    raise SystemExit("usage: ./kill.sh [--dry-run]")
dry_run = bool(args)
print("WARNING: emergency force-exit bypasses owner Stop and cannot prove physical safety.", file=sys.stderr)

names = {"isos", "aether-profile", "aether-runtime", "aether-http-bridge", "aether-config-server", "ei-runtime", "ei"}
run_dir = pathlib.Path(os.environ.get("ISOS_STOP_RUN_DIR", root / "data/isos/run")).resolve()

def within(path, directory):
    try:
        path.relative_to(directory)
        return True
    except ValueError:
        return False

def process_info(pid):
    try:
        fields = ((pathlib.Path("/proc") / str(pid) / "stat").read_text()).rsplit(") ", 1)[1].split()
        return int(fields[1]), fields[19], fields[0]
    except (OSError, IndexError, ValueError):
        return None

def process_group(pid):
    try:
        fields = ((pathlib.Path("/proc") / str(pid) / "stat").read_text()).rsplit(") ", 1)[1].split()
        return int(fields[2])
    except (OSError, IndexError, ValueError):
        return None

def identity(pid):
    proc = pathlib.Path("/proc") / str(pid)
    try:
        info = process_info(pid)
        if info is None or info[2] == "Z":
            return None
        start = info[1]
        raw = (proc / "cmdline").read_bytes().split(b"\0")
        argv = [os.fsdecode(item) for item in raw if item]
        exe = os.readlink(proc / "exe").removesuffix(" (deleted)")
        cwd = pathlib.Path(os.readlink(proc / "cwd"))
    except (OSError, IndexError, ValueError):
        return None
    if not argv:
        return None
    exe_path = pathlib.Path(exe)
    if not within(exe_path, root) and not within(cwd, root):
        return None
    name = exe_path.name
    if name in names and within(exe_path, root):
        if name in {"aether-profile", "ei-runtime", "ei"} and "owner" in argv:
            # An owner from another test workspace is not this deployment.
            if "--socket" not in argv:
                return None
            index = argv.index("--socket") + 1
            if index >= len(argv) or not within(pathlib.Path(argv[index]), run_dir):
                return None
        return start, " ".join(argv)
    if name.startswith("python") and within(cwd, root):
        if "-m" in argv and "ei.config_cli" in argv and "owner" in argv and "--socket" in argv:
            index = argv.index("--socket") + 1
            if index < len(argv) and within(pathlib.Path(argv[index]), run_dir):
                return start, " ".join(argv)
    return None

def targets():
    found = {}
    for entry in pathlib.Path("/proc").iterdir():
        if not entry.name.isdecimal():
            continue
        pid = int(entry.name)
        if pid in (os.getpid(), os.getppid()):
            continue
        value = identity(pid)
        if value is not None:
            found[pid] = value
    return found

selected = targets()
all_processes = {}
for entry in pathlib.Path("/proc").iterdir():
    if entry.name.isdecimal():
        pid = int(entry.name)
        info = process_info(pid)
        if info is not None and info[2] != "Z":
            all_processes[pid] = info
descendants = {}
frontier = set(selected)
while frontier:
    children = {pid for pid, (ppid, _, _) in all_processes.items() if ppid in frontier}
    children -= set(selected) | set(descendants) | {os.getpid(), os.getppid()}
    for pid in children:
        descendants[pid] = all_processes[pid][1]
    frontier = children

for pid, (_, command) in sorted(selected.items()):
    print(f"{pid}: {command}")
for pid in sorted(descendants):
    print(f"{pid}: descendant of this deployment")
if dry_run:
    raise SystemExit(0)

tracked_groups = {process_group(pid) for pid in {**selected, **descendants}}
tracked_groups.discard(None)
tracked_groups.discard(process_group(os.getpid()))

def group_members():
    found = {}
    for entry in pathlib.Path("/proc").iterdir():
        if not entry.name.isdecimal():
            continue
        pid = int(entry.name)
        if pid in (os.getpid(), os.getppid()):
            continue
        info = process_info(pid)
        if info is not None and info[2] != "Z" and process_group(pid) in tracked_groups:
            found[pid] = info[1]
    return found

def send(pid, start, sig):
    current = process_info(pid)
    if current is None or current[1] != start or current[2] == "Z":
        return
    try:
        os.kill(pid, sig)
    except ProcessLookupError:
        pass
    except PermissionError:
        print(f"permission denied for PID {pid}; rerun with sudo", file=sys.stderr)
        raise SystemExit(1)

def alive(pid, start):
    info = process_info(pid)
    return info is not None and info[1] == start and info[2] != "Z"

manager = {pid: start for pid, (start, command) in selected.items() if command.split()[0].endswith("/isos")}
owners = {
    pid: start for pid, (start, command) in selected.items()
    if pid not in manager and " owner " in f" {command} " and "--socket" in command
}
workers = {pid: start for pid, (start, _) in selected.items() if pid not in manager and pid not in owners}

def stop_group(group):
    for pid, start in group.items():
        send(pid, start, signal.SIGTERM)
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline and any(alive(pid, start) for pid, start in group.items()):
        time.sleep(0.1)
    for pid, start in group.items():
        send(pid, start, signal.SIGKILL)
    deadline = time.monotonic() + 2
    while time.monotonic() < deadline and any(alive(pid, start) for pid, start in group.items()):
        time.sleep(0.1)

stop_group(manager)
stop_group(owners)
stop_group({**workers, **descendants})
stop_group(group_members())
remaining = [pid for pid, start in {**owners, **workers, **descendants, **manager}.items() if alive(pid, start)]
remaining += [pid for pid in targets() if pid not in selected]
remaining += list(group_members())
if remaining:
    raise SystemExit(f"ERROR: processes remain: {sorted(set(remaining))}")
print("Emergency exit complete. Inspect hardware before restarting.", file=sys.stderr)
PY
