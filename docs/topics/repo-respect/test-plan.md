# Repo-Respect Receipt Gate Test Plan

This document defines how Bunny verifies the repo-respect receipt gate.

The current topic chapter is [`README.md`](README.md). This plan records the
receipt-gate requirements, executable xtask evidence, and remaining coverage
gaps.

## Scope

This plan covers the `xtask repo-respect` command, its branch and staged check
modes, receipt content validation, receipt trailer validation, and shared xtask
Git subprocess isolation.

It does not cover social review judgment. The receipt gate can prove that a
receipt exists, has required sections, and is connected to commits; it cannot
prove that the human prose is complete or wise.

## Test Goals

- Prove that required receipt fields are present and contain real content.
- Prove that placeholders and comments do not satisfy required fields.
- Prove that receipt coverage includes path states that are easy to miss,
  especially deleted and typechanged files.
- Prove that staged checks read receipt content from the Git index.
- Prove that branch checks validate only pull-request-side non-merge commits.
- Prove that nested Git subprocesses ignore inherited external indexes.

## Requirements

The table is the human-readable view. The fenced TOML block immediately after
it is the contract graph consumed by `cargo run --locked -p xtask --
topic-docs`. Tooling reads only the structured block, not visual Markdown table
formatting.

| ID | Requirement | Current Source |
| --- | --- | --- |
| RR-REQ-001 | Pull requests that change non-receipt files must carry a receipt with all required fields. | `README.md#receipt-files` |
| RR-REQ-002 | Receipt field content must be real contributor prose or lists, not placeholders, comments, or embedded heading mentions. | `README.md#receipt-files` |
| RR-REQ-003 | Non-merge pull-request commits must carry a full-path Repo-Respect-Receipt trailer. | `README.md#commit-trailers` |
| RR-REQ-004 | Staged and branch checks must include deleted and typechanged paths when deciding receipt coverage. | `README.md#check-modes` |
| RR-REQ-005 | Staged checks must validate receipt content from the Git index, not from unstaged worktree edits. | `README.md#check-modes` |
| RR-REQ-006 | Branch trailer validation must inspect only commits on the pull-request side of the merge base. | `README.md#check-modes` |
| RR-REQ-007 | xtask Git subprocesses must remove inherited external Git index paths while preserving repository-owned index paths. | `README.md#git-index-isolation` |

```toml
[[requirement]]
id = "RR-REQ-001"
summary = "Pull requests that change non-receipt files must carry a receipt with all required fields."
status = "active"

[[requirement]]
id = "RR-REQ-002"
summary = "Receipt field content must be real contributor prose or lists, not placeholders, comments, or embedded heading mentions."
status = "active"

[[requirement]]
id = "RR-REQ-003"
summary = "Non-merge pull-request commits must carry a full-path Repo-Respect-Receipt trailer."
status = "active"

[[requirement]]
id = "RR-REQ-004"
summary = "Staged and branch checks must include deleted and typechanged paths when deciding receipt coverage."
status = "active"

[[requirement]]
id = "RR-REQ-005"
summary = "Staged checks must validate receipt content from the Git index, not from unstaged worktree edits."
status = "active"

[[requirement]]
id = "RR-REQ-006"
summary = "Branch trailer validation must inspect only commits on the pull-request side of the merge base."
status = "active"

[[requirement]]
id = "RR-REQ-007"
summary = "xtask Git subprocesses must remove inherited external Git index paths while preserving repository-owned index paths."
status = "active"
```

## Fixtures

The tests create throwaway Git repositories in the system temporary directory.
Each fixture repository has an `origin/main` reference so branch and staged
checks exercise the same merge-base logic used by pull requests.

Receipt text is built in test fixtures instead of read from checked-in sample
files. That keeps the oracle focused on parser behavior and avoids testing the
repository's current receipts as fixtures.

## Test Cases

| ID | Category | Requirements | Oracle | Test |
| --- | --- | --- | --- | --- |
| RR-TP-001 | Receipt template | RR-REQ-001 | Generated templates contain every required receipt heading. | `xtask/src/repo_respect.rs::tests::receipt_template_contains_required_field_headings` |
| RR-TP-002 | Missing fields | RR-REQ-001 | Missing required fields produce validation failures naming the missing field. | `xtask/src/repo_respect.rs::tests::receipt_validation_requires_all_fields` |
| RR-TP-003 | Placeholder rejection | RR-REQ-002 | Generated placeholder-only sections fail required-field validation. | `xtask/src/repo_respect.rs::tests::receipt_validation_rejects_placeholder_sections` |
| RR-TP-004 | Heading anchoring | RR-REQ-002 | A field name mentioned inside another field's prose does not satisfy the missing field. | `xtask/src/repo_respect.rs::tests::receipt_validation_requires_exact_field_headings` |
| RR-TP-005 | HTML comments | RR-REQ-002 | Multiline HTML comments do not satisfy required-field content. | `xtask/src/repo_respect.rs::tests::receipt_validation_ignores_multiline_html_comments` |
| RR-TP-006 | Deleted path coverage | RR-REQ-004 | Deleted tracked paths appear in branch and staged changed-path sets. | `xtask/src/repo_respect.rs::tests::changed_path_lists_include_deleted_files` |
| RR-TP-007 | Typechanged path coverage | RR-REQ-004 | Typechanged tracked paths appear in branch and staged changed-path sets. | `xtask/src/repo_respect.rs::tests::changed_path_lists_include_typechanged_files` |
| RR-TP-008 | Staged receipt source | RR-REQ-005 | Invalid staged receipt content fails even when the worktree receipt is valid. | `xtask/src/repo_respect.rs::tests::staged_check_validates_receipt_content_from_index` |
| RR-TP-009 | Trailer enforcement | RR-REQ-003 | Non-merge branch commits without a receipt trailer are reported. | `xtask/src/repo_respect.rs::tests::branch_commit_message_failures_report_missing_trailers` |
| RR-TP-010 | Merge-base range | RR-REQ-006 | Commits that exist only on the updated base side do not create trailer failures. | `xtask/src/repo_respect.rs::tests::branch_commit_message_failures_ignore_new_base_commits` |
| RR-TP-011 | Git index isolation | RR-REQ-007 | External inherited Git indexes are removed while repository-owned indexes are preserved. | `xtask/src/git_helpers.rs::tests::external_git_index_is_removed` |

```toml
[[case]]
id = "RR-TP-001"
requirements = ["RR-REQ-001"]
evidence = "test"
test = "xtask/src/repo_respect.rs::tests::receipt_template_contains_required_field_headings"
oracle = "Generated templates contain every required receipt heading."
tier = "fast"
status = "implemented"

[[case]]
id = "RR-TP-002"
requirements = ["RR-REQ-001"]
evidence = "test"
test = "xtask/src/repo_respect.rs::tests::receipt_validation_requires_all_fields"
oracle = "Missing required fields produce validation failures naming the missing field."
tier = "fast"
status = "implemented"

[[case]]
id = "RR-TP-003"
requirements = ["RR-REQ-002"]
evidence = "test"
test = "xtask/src/repo_respect.rs::tests::receipt_validation_rejects_placeholder_sections"
oracle = "Generated placeholder-only sections fail required-field validation."
tier = "fast"
status = "implemented"

[[case]]
id = "RR-TP-004"
requirements = ["RR-REQ-002"]
evidence = "test"
test = "xtask/src/repo_respect.rs::tests::receipt_validation_requires_exact_field_headings"
oracle = "A field name mentioned inside another field's prose does not satisfy the missing field."
tier = "fast"
status = "implemented"

[[case]]
id = "RR-TP-005"
requirements = ["RR-REQ-002"]
evidence = "test"
test = "xtask/src/repo_respect.rs::tests::receipt_validation_ignores_multiline_html_comments"
oracle = "Multiline HTML comments do not satisfy required-field content."
tier = "fast"
status = "implemented"

[[case]]
id = "RR-TP-006"
requirements = ["RR-REQ-004"]
evidence = "test"
test = "xtask/src/repo_respect.rs::tests::changed_path_lists_include_deleted_files"
oracle = "Deleted tracked paths appear in branch and staged changed-path sets."
tier = "fast"
status = "implemented"

[[case]]
id = "RR-TP-007"
requirements = ["RR-REQ-004"]
evidence = "test"
test = "xtask/src/repo_respect.rs::tests::changed_path_lists_include_typechanged_files"
oracle = "Typechanged tracked paths appear in branch and staged changed-path sets."
tier = "fast"
status = "implemented"

[[case]]
id = "RR-TP-008"
requirements = ["RR-REQ-005"]
evidence = "test"
test = "xtask/src/repo_respect.rs::tests::staged_check_validates_receipt_content_from_index"
oracle = "Invalid staged receipt content fails even when the worktree receipt is valid."
tier = "fast"
status = "implemented"

[[case]]
id = "RR-TP-009"
requirements = ["RR-REQ-003"]
evidence = "test"
test = "xtask/src/repo_respect.rs::tests::branch_commit_message_failures_report_missing_trailers"
oracle = "Non-merge branch commits without a receipt trailer are reported."
tier = "fast"
status = "implemented"

[[case]]
id = "RR-TP-010"
requirements = ["RR-REQ-006"]
evidence = "test"
test = "xtask/src/repo_respect.rs::tests::branch_commit_message_failures_ignore_new_base_commits"
oracle = "Commits that exist only on the updated base side do not create trailer failures."
tier = "fast"
status = "implemented"

[[case]]
id = "RR-TP-011"
requirements = ["RR-REQ-007"]
evidence = "test"
test = "xtask/src/git_helpers.rs::tests::external_git_index_is_removed"
oracle = "External inherited Git indexes are removed while repository-owned indexes are preserved."
tier = "fast"
status = "implemented"
```

## Determinism Obligations And Evidence

The receipt gate touches Git, so its determinism obligation is about stable
repository-state interpretation:

- tests create isolated temporary repositories;
- test fixtures define `origin/main` explicitly;
- changed-path checks compare Git path sets, not human command output;
- receipt validation asserts stable failure categories instead of rendered CLI
  prose;
- Git index isolation tests compare path-classification behavior directly.

There is no randomness, wall-clock assertion, filesystem directory-order
oracle, network access, stdout scraping, or dependency on a user's global Git
configuration.

## Known Failures

The current executable surface has no known failing repo-respect cases.

## Edge Cases And Unusual Inputs

Current tests cover:

- missing receipt fields;
- placeholder-only receipt sections;
- multiline HTML comments;
- field names embedded in another field's prose;
- deleted tracked files;
- typechanged tracked files;
- staged receipt content that differs from worktree receipt content;
- missing non-merge commit trailers;
- base-only commits after `origin/main` moves;
- external and repository-owned Git index paths.

## Stress And Fuzz

No fuzz target exists for receipt parsing today. If receipt parsing grows beyond
field-heading validation, add property tests that generate field order,
comment, placeholder, and prose combinations, then freeze any minimized failure
as an ordinary xtask regression.

## Open Gaps

| Gap | Blocking Work |
| --- | --- |
| End-to-end hook fixture for pre-commit and commit-msg together. | A slower integration-test harness that can install temporary hooks safely. |
| Direct full Code Dojo failure fixture for missing receipt coverage. | A stable test harness around `code_dojo::run_full`. |
