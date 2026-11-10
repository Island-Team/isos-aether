#!/usr/bin/env python3
"""Append one canonical core log record, or capture a command's output as records."""

import argparse
import json
import sys
from datetime import datetime, timezone
from pathlib import Path


def record(level: str, source: str, message: str) -> str:
    return json.dumps(
        {
            "timestamp": datetime.now(timezone.utc).isoformat(timespec="milliseconds").replace("+00:00", "Z"),
            "level": level.lower(),
            "source": source,
            "message": message.rstrip("\r\n"),
        },
        ensure_ascii=False,
        separators=(",", ":"),
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("path", type=Path)
    parser.add_argument("level", choices=("trace", "debug", "info", "warn", "error"))
    parser.add_argument("source")
    parser.add_argument("message", nargs="?")
    parser.add_argument("--stdin", action="store_true")
    args = parser.parse_args()
    if args.stdin == (args.message is not None):
        parser.error("provide a message or --stdin")
    with args.path.open("a", encoding="utf-8") as output:
        if args.stdin:
            for line in sys.stdin:
                output.write(record(args.level, args.source, line) + "\n")
                output.flush()
                sys.stdout.write(line)
                sys.stdout.flush()
        else:
            output.write(record(args.level, args.source, args.message) + "\n")


if __name__ == "__main__":
    main()
