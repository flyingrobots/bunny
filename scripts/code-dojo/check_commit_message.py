#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
from pathlib import Path

SUBJECT_RE = re.compile(r"^[a-z0-9_.-]+(?:/[a-z0-9_.-]+)?: [a-z][^.]*[^.]$")
MERGE_PREFIXES = ("Merge ", "Revert ")
AI_MARKERS = (
    "Co-Authored-By: ChatGPT",
    "Co-authored-by: ChatGPT",
    "AI-Assisted: true",
    "AI-Authored: true",
    "Generated-By: ChatGPT",
    "Generated-By: OpenAI",
)


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: check_commit_message.py <commit-msg-file>", file=sys.stderr)
        return 2

    path = Path(sys.argv[1])
    text = path.read_text(encoding="utf-8", errors="replace")
    lines = [line.rstrip() for line in text.splitlines()]
    non_comment = [line for line in lines if line.strip() and not line.startswith("#")]

    if not non_comment:
        print("Code Dojo: empty commit message", file=sys.stderr)
        return 1

    subject = non_comment[0]
    if subject.startswith(MERGE_PREFIXES):
        return 0

    failures: list[str] = []

    if len(subject) > 72:
        failures.append(f"subject is {len(subject)} characters; keep it <= 72")

    if not SUBJECT_RE.match(subject):
        failures.append("subject must look like '<scope>: <imperative summary>'")

    if subject.endswith("."):
        failures.append("subject must not end with a period")

    # Discourage catch-all commits.
    vague = {"fix", "update", "changes", "misc", "wip", "cleanup", "stuff"}
    summary = subject.split(":", 1)[-1].strip().lower()
    if summary in vague:
        failures.append("subject is too vague; name the causal change")

    ai_assisted = any(marker in text for marker in AI_MARKERS)
    has_receipt = any(line.startswith("Repo-Respect-Receipt:") for line in non_comment)
    if ai_assisted and not has_receipt:
        failures.append("AI-assisted commits require 'Repo-Respect-Receipt: <id-or-path>' trailer")

    if failures:
        print("Code Dojo: commit message rejected", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        print("\nExample: bunny-num: define checked Q32x32 multiplication", file=sys.stderr)
        return 1

    print("Code Dojo: commit message clean")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
