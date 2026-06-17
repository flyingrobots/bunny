#!/usr/bin/env python3
from __future__ import annotations

import argparse
import os
import subprocess
import sys
from pathlib import Path

from common import STRICT_CLIPPY_PACKAGES, WASM_PACKAGES, git_root


def run(cmd: list[str]) -> int:
    print("Code Dojo:", " ".join(cmd))
    result = subprocess.run(cmd, text=True)
    return result.returncode


def cargo_available(root: Path) -> bool:
    return (root / "Cargo.toml").exists()


def main() -> int:
    parser = argparse.ArgumentParser(description="Run the full Code Dojo gate")
    parser.add_argument("--all", action="store_true", help="check tracked and untracked files")
    parser.add_argument("--ci", action="store_true", help="CI mode")
    args = parser.parse_args()

    root = git_root()
    os.chdir(root)

    rc = run([sys.executable, "scripts/code-dojo/check_files.py", "--all"])
    if rc:
        return rc

    vector_cmd = [sys.executable, "scripts/code-dojo/check_determinism_manifest.py"]
    vector_cmd.append("--enforce")
    rc = run(vector_cmd)
    if rc:
        return rc

    if not cargo_available(root):
        print("Code Dojo: Cargo.toml is required for the full gate", file=sys.stderr)
        return 1

    commands = [
        ["cargo", "fmt", "--all", "--", "--check"],
        [
            "cargo",
            "clippy",
            "--locked",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ],
        [
            "cargo",
            "clippy",
            "--locked",
            *strict_clippy_package_args(),
            "--all-features",
            "--",
            "-D",
            "clippy::unwrap_used",
            "-D",
            "clippy::expect_used",
            "-D",
            "clippy::panic",
            "-D",
            "clippy::todo",
            "-D",
            "clippy::unimplemented",
            "-D",
            "clippy::indexing_slicing",
        ],
        ["cargo", "deny", "check"],
        ["cargo", "test", "--locked", "--workspace", "--all-targets", "--all-features"],
    ]

    commands.append(["rustup", "target", "add", "wasm32-unknown-unknown"])
    commands.append(
        [
            "cargo",
            "check",
            "--locked",
            *package_args(),
            "--target",
            "wasm32-unknown-unknown",
            "--all-features",
        ]
    )

    for cmd in commands:
        rc = run(cmd)
        if rc:
            return rc

    print("Code Dojo: full gate clean")
    return 0


def package_args() -> list[str]:
    args: list[str] = []
    for package in WASM_PACKAGES:
        args.extend(["-p", package])
    return args


def strict_clippy_package_args() -> list[str]:
    args: list[str] = []
    for package in STRICT_CLIPPY_PACKAGES:
        args.extend(["-p", package])
    return args


if __name__ == "__main__":
    raise SystemExit(main())
