#!/usr/bin/env python3
from __future__ import annotations

import argparse
import subprocess
import sys

from common import git_root


def main() -> int:
    parser = argparse.ArgumentParser(description="Code Dojo Rust repository policy checks")
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--staged", action="store_true", help="check staged files only")
    group.add_argument("--all", action="store_true", help="check tracked and untracked files")
    args = parser.parse_args()

    root = git_root()
    mode = "--staged" if args.staged else "--all"
    cmd = ["cargo", "run", "--locked", "-p", "xtask", "--", "code-dojo-rust", mode]
    print("Code Dojo:", " ".join(cmd), flush=True)
    return subprocess.run(cmd, text=True, cwd=root).returncode

if __name__ == "__main__":
    raise SystemExit(main())
