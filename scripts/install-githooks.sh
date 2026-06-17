#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "${repo_root}" ]]; then
  echo "Code Dojo: not inside a Git repository" >&2
  exit 1
fi

cd "${repo_root}"

if [[ ! -d ".githooks" ]]; then
  echo "Code Dojo: .githooks directory not found" >&2
  exit 1
fi

chmod +x .githooks/pre-commit .githooks/commit-msg .githooks/pre-push

git config core.hooksPath .githooks

echo "Code Dojo: installed repo-local hooks at .githooks"
echo "Code Dojo: run 'cargo run --locked -p xtask -- code-dojo --all' to test the full gate"
