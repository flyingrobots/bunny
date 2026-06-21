# Repo-Respect Receipt Gate

This chapter defines Bunny's current repo-respect receipt contract.

Repo-respect is a repository process gate, not a runtime library feature. It
keeps pull requests auditable by requiring contributors to write down the local
context they used, the files they touched, the checks they ran, and the risks
they still know about.

## Status

This is a living topic chapter. The receipt gate is implemented by the `xtask`
`repo-respect` command and is invoked by Code Dojo, local Git hooks, and CI.

The current verification design is [`test-plan.md`](test-plan.md).

## Receipt Files

Every pull request that changes non-receipt files must add or update at least
one Markdown receipt under `.repo-respect/receipts/`.

Each receipt must include content for the required fields:

- `Task:`
- `Files read:`
- `Files edited:`
- `Topic docs:`
- `Generated artifacts:`
- `Checks run:`
- `Known risks:`
- `Human reviewer:`

Placeholder-only sections, empty sections, and HTML comments do not satisfy a
field. Field headings count only when they appear as actual headings, not when
they are mentioned inside another field's prose.

## Commit Trailers

Every non-merge commit must include a trailer that points at a full receipt
path:

```text
Repo-Respect-Receipt: .repo-respect/receipts/<id>.md
```

Bare IDs are invalid. The path must live under `.repo-respect/receipts/` and
end in `.md`.

## Check Modes

`repo-respect check --staged` is the pre-commit mode. It validates the changed
paths that are already on the branch plus staged paths in the Git index. Staged
receipt content is read from the index, not from the worktree, so unstaged
receipt edits cannot mask an invalid staged receipt.

`repo-respect check --branch` is the full-gate mode. It validates branch changes
against `origin/main` and validates receipt trailers for non-merge commits on
the pull-request side of the merge base. Commits that exist only on the updated
base branch are not part of the contributor's trailer obligation.

Both modes include deleted and typechanged paths when deciding whether the pull
request needs receipt coverage.

## Git Index Isolation

Git hooks may provide `GIT_INDEX_FILE` to point at a temporary external index.
Nested Git subprocesses that need repository state must not accidentally reuse
that external index. Bunny's shared xtask Git helper removes inherited external
index paths while preserving index paths inside the repository's own `.git`
directory.

## Required Tests

The current repository must keep executable evidence for:

- required receipt field validation;
- placeholder and comment rejection;
- exact field-heading detection;
- deleted and typechanged path coverage;
- staged receipt validation from the Git index;
- non-merge branch commit trailer enforcement;
- merge-base aware branch commit range selection;
- external `GIT_INDEX_FILE` sanitization.
