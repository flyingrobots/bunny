from __future__ import annotations

import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

CORE_CRATES = {
    "bunny-num",
    "bunny-linalg",
    "bunny-geom",
    "bunny-query",
    "bunny-broadphase",
    "bunny-mesh",
}
GENERATOR_CRATES = {"bunny-wesley"}
TOOLING_CRATES = {"xtask"}
STRICT_CLIPPY_PACKAGES = (
    "bunny-num",
    "bunny-linalg",
    "bunny-geom",
    "bunny-query",
    "bunny-broadphase",
    "bunny-mesh",
    "bunny-codec",
    "bunny-contract",
)
WASM_PACKAGES = (
    "bunny-num",
    "bunny-linalg",
    "bunny-geom",
    "bunny-contract",
    "bunny-query",
    "bunny-broadphase",
    "bunny-mesh",
    "bunny-codec",
)

RUST_FILE_SUFFIX = ".rs"

@dataclass(frozen=True)
class Violation:
    path: Path
    line: int
    rule: str
    message: str

    def format(self) -> str:
        loc = f"{self.path}:{self.line}" if self.line else str(self.path)
        return f"{loc}: [{self.rule}] {self.message}"


def run(cmd: list[str], *, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, text=True, check=check)


def git_root() -> Path:
    try:
        out = subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip()
        return Path(out)
    except subprocess.CalledProcessError:
        return Path.cwd()


def git_files(staged: bool) -> list[Path]:
    root = git_root()
    if staged:
        cmd = ["git", "diff", "--cached", "--name-only", "--diff-filter=ACMR"]
    else:
        cmd = ["git", "ls-files"]
    try:
        out = subprocess.check_output(cmd, text=True, cwd=root)
    except subprocess.CalledProcessError:
        return []
    return [root / line.strip() for line in out.splitlines() if line.strip()]


def rust_files(files: Iterable[Path]) -> list[Path]:
    root = git_root()
    result: list[Path] = []
    for path in files:
        try:
            rel = path.relative_to(root)
        except ValueError:
            rel = path
        parts = set(rel.parts)
        if path.suffix != RUST_FILE_SUFFIX:
            continue
        if "target" in parts or ".git" in parts:
            continue
        if "vendor" in parts or "third_party" in parts:
            continue
        if not path.exists():
            continue
        result.append(path)
    return result


def rel(path: Path) -> Path:
    root = git_root()
    try:
        return path.relative_to(root)
    except ValueError:
        return path


def crate_name(path: Path) -> str | None:
    r = rel(path)
    parts = r.parts
    if len(parts) >= 2 and parts[0] == "crates":
        return parts[1]
    if parts and parts[0] == "xtask":
        return "xtask"
    return None


def crate_category(path: Path) -> str:
    name = crate_name(path)
    if name in CORE_CRATES:
        return "core"
    if name in GENERATOR_CRATES:
        return "generator"
    if name in TOOLING_CRATES:
        return "tooling"
    return "unknown"


def is_core(path: Path) -> bool:
    return crate_category(path) == "core"


def read_lines(path: Path) -> list[str]:
    return path.read_text(encoding="utf-8", errors="replace").splitlines()


def has_waiver(lines: list[str], index: int, rule: str) -> bool:
    """Return true when current or previous two lines contain a specific dojo waiver."""
    start = max(0, index - 2)
    needle = f"dojo: allow {rule}"
    for i in range(start, index + 1):
        text = lines[i]
        if needle in text:
            # Force a reason. Waiver without explanation is just ceremonial fog.
            return "--" in text and len(text.split("--", 1)[1].strip()) >= 8
    return False


def add_violation(violations: list[Violation], path: Path, line: int, rule: str, message: str) -> None:
    violations.append(Violation(rel(path), line, rule, message))


def print_violations(violations: list[Violation]) -> int:
    if not violations:
        print("Code Dojo: clean")
        return 0
    print("Code Dojo: violations found", file=sys.stderr)
    for violation in violations:
        print("  " + violation.format(), file=sys.stderr)
    return 1


def strip_line_comment(line: str) -> str:
    # Good enough for policy scanning. The compiler remains the real parser.
    if "//" in line:
        return line.split("//", 1)[0]
    return line


def source_line_count(lines: list[str]) -> int:
    count = 0
    for line in lines:
        stripped = line.strip()
        if not stripped or stripped.startswith("//"):
            continue
        count += 1
    return count

