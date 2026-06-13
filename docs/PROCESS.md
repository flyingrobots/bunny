# Bunny — Process

Bunny uses the METHOD lightweight process framework adapted for deterministic library development.

---

## Cycle Doctrine

The development process flows in a continuous cycle:
```text
Issue/Backlog -> Branch -> Design Doc -> RED (Failing tests) -> GREEN (Passing code) -> PR -> Ship Sync
```

### 1. Backlog Capture
All work, refactoring, and bug fixes start as GitHub issues. Lane labels categorize priorities:
* `lane:inbox`: Unprocessed cards, raw ideas.
* `lane:asap`: High-priority backlog. Pull from here next.
* `lane:bad-code`: Technical debt, code smell tracking.
* `lane:cool-ideas`: Future exploratory features.
* `lane:graveyard`: Permanently retired backlog items.

### 2. Design Doc
Before writing code, draft a design RFC under `docs/design/<legend>_<slug>.md`. The design must define:
* The target criteria (exact, falsifiable goals).
* The implementation details.
* Verification plan.

### 3. Red & Green Execution
Write failing test cases inside integration test folders before writing implementation code. Implement until the tests pass and code quality guidelines are satisfied.

### 4. PR & CI Validation
All code merges through pull requests. The PR is gated by:
* Green CI runs on Linux, macOS, and Windows.
* Formatting validation (`cargo fmt --check`).
* Clippy quality gate (`cargo clippy --workspace --all-targets -- -D warnings`).
* Portability compilation checks (`wasm32-unknown-unknown`).

### 5. Ship Sync (Post-Merge Checklist)
After a PR merges to `main`, run this checklist on `main`:
1. Update `docs/BEARING.md` (Recent ships, active slice, caveats).
2. Update `CHANGELOG.md` with SemVer-guided adjustments.
3. Clean/delete resolved issues from the GitHub lane trackers.

---

## Branch Naming Conventions

| Branch Type | Pattern | Example |
| --- | --- | --- |
| Feature Work | `feature/slug` | `feature/linalg-fixed-vectors` |
| Documentation | `docs/slug` | `docs/crate-readmes` |
| Maintenance | `maint-slug` | `maint-update-lockfile` |
| Triage | `triage-slug` | `triage-backlog-cleanup` |

> [!NOTE]
> Operational branches (`maint-`, `triage-`) use a hyphen instead of a slash namespace (`feature/`, `docs/`, `cycles/`) to avoid Git reference resolution directory/file collisions. For example, if a branch named `maint` or `triage` is ever created, Git forbids creating any branch starting with `maint/` or `triage/` due to filesystem directory conflicts in `.git/refs/heads/`. Hyphenating short-lived operational branches keeps the namespace flat and safe from these collisions.

