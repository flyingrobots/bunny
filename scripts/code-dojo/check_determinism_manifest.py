#!/usr/bin/env python3
from __future__ import annotations

import argparse
from pathlib import Path

from common import CORE_CRATES, git_root, print_violations, Violation

GOLDEN_NAMES = {
    "golden_vectors.rs",
    "determinism.rs",
    "fixed_q32x32_vectors.rs",
    "geometry_degenerates.rs",
}


def main() -> int:
    parser = argparse.ArgumentParser(description="Check deterministic test receipts for core crates")
    parser.add_argument("--enforce", action="store_true", help="fail when core crates lack golden-vector tests")
    args = parser.parse_args()

    root = git_root()
    violations: list[Violation] = []

    for crate in sorted(CORE_CRATES):
        crate_dir = root / "crates" / crate
        if not crate_dir.exists():
            continue
        tests_dir = crate_dir / "tests"
        found = False
        if tests_dir.exists():
            for path in tests_dir.rglob("*.rs"):
                if path.name in GOLDEN_NAMES:
                    found = True
                    break
                text = path.read_text(encoding="utf-8", errors="replace")
                if "golden" in text.lower() and "determin" in text.lower():
                    found = True
                    break
        if not found:
            violations.append(
                Violation(
                    Path("crates") / crate,
                    0,
                    "determinism-tests",
                    "core crate should include deterministic golden-vector or degeneracy tests",
                )
            )

    if violations:
        if args.enforce:
            return print_violations(violations)
        print("Code Dojo: deterministic test warnings")
        for violation in violations:
            print("  " + violation.format())
    else:
        print("Code Dojo: deterministic test receipts present")

    return 0

if __name__ == "__main__":
    raise SystemExit(main())
