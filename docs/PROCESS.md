# Bunny - Process

Bunny uses goalpost-based delivery. A goalpost is the smallest unit that should
be reviewed and merged as product-quality work.

## Operating Loop

```text
Roadmap claim -> goalpost branch -> slice tests -> implementation -> evidence
-> self-review -> PR -> CI/review -> merge -> signpost sync
```

Every loop must preserve repo truth. If a claim is not backed by implementation,
tests, CI, or an explicit evidence artifact, it is not done.

## Work Units

| Unit | Meaning |
| --- | --- |
| Release | SemVer-scale roadmap section |
| Goalpost | PR-sized body of related work |
| Slice | Commit-sized acceptance step inside a goalpost |
| Evidence | File, test, command, CI result, or doc anchor proving a claim |

## Branches

| Branch Type | Pattern | Example |
| --- | --- | --- |
| Goalpost | `goalpost/version-gpN` | `goalpost/v0.4.0-gp2` |
| Documentation | `docs/slug` | `docs/rewrite-signposts` |
| Maintenance | `maint-slug` | `maint-update-lockfile` |
| Triage | `triage-slug` | `triage-backlog-cleanup` |

Do not use agent prefixes in branch names, commit messages, or PR titles. Do not
rebase, amend, force-push, or create draft pull requests.

## Slice Discipline

For each meaningful issue or slice:

1. State the claim or bug in one sentence.
2. Add the smallest deterministic failing test or evidence check.
3. Implement the smallest fix that satisfies the claim.
4. Run focused verification first.
5. Broaden verification when public APIs, shared contracts, or CI behavior
   changed.
6. Commit the slice as its own new commit.

Documentation-only slices still need verification: Markdown lint, diff checks,
and source anchors where claims are factual.

## Review Discipline

Self-review happens before PR and after substantial review feedback. It should
look for:

* False completion labels.
* Missing rejection paths.
* Non-deterministic behavior.
* Hidden allocation claims without witnesses.
* Panics in library code.
* Docs that describe future intent as current truth.
* CI claims that do not match workflow files.

Findings are fixed with new commits. They are not hidden by softening the
acceptance criteria after the fact.

## Pull Requests

PRs should be ready for review, not drafts. A PR body must explain:

* What changed.
* Why it changed.
* The user or downstream impact.
* Any root cause for fixes.
* The exact validation commands that passed.

Merge only when CI is green, review threads are resolved, and the active merge
policy is satisfied.

## Post-Merge Sync

After a PR lands on `main`:

1. Fetch and update local `main`.
2. Confirm the merge commit and PR state.
3. Update `docs/BEARING.md` for the new current position.
4. Update `CHANGELOG.md` if release-visible behavior changed.
5. Start the next goalpost branch from current `main`.
