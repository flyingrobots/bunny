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
These are contributor rules verified during review and handoff; they are not
presented as local hook guarantees.

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

## Documentation Source Of Truth

Current feature behavior belongs in a living reference document that is updated
in place as the code changes. Historical design documents, goalpost notes,
issues, and pull requests explain why decisions happened; they must not become
competing current references. A living topic chapter describes behavior that
exists in `HEAD`; it must not describe intended behavior before implementation
and evidence exist.

When a feature changes:

1. Write or update a proposal, RFC, or rationale note when genuine design
   discussion is needed.
2. Update the topic test plan before implementation with planned cases,
   explicit oracles, evidence type, status, and stable IDs.
3. Write failing executable evidence for the planned cases.
4. Implement the behavior.
5. Update the living topic chapter so it describes the now-landed behavior.
6. Mark planned cases as implemented evidence and record the actual test names,
   fixtures, or artifact anchors.
7. Leave historical design documents intact unless they are factually wrong.
8. Add a short superseded note to historical documents when readers could
   otherwise mistake them for current truth.
9. Keep release chronology in `CHANGELOG.md`.

Examples of living references include `docs/NUMERIC_CONSTITUTION.md`,
`docs/topics/coordinate-law/`, and `docs/MATH_GEOMETRY_CAPABILITY_MAP.md`.

Durable concepts that will be changed by more than one pull request should use
the topic-folder shape documented in `docs/README.md`:

```text
docs/topics/<topic>/
  README.md
  test-plan.md
  architecture.md  # optional
  rationale.md     # optional
```

`README.md` is current behavior. `test-plan.md` is current verification design.
Historical proposals belong in `docs/design/`, and delivery evidence belongs in
`docs/goalposts/`.

Topic test plans use fenced `toml` metadata blocks as the machine-readable
contract graph. Visual Markdown tables are for readers only. The local contract
checker validates stable requirement IDs, case IDs, oracles, evidence types,
planned versus implemented status, and implemented Rust test names:

```bash
cargo run --locked -p xtask -- topic-docs
```

## Code Dojo

The repository uses Code Dojo as its local and CI enforcement layer. Install the
repo-local hooks once per clone:

```bash
bash scripts/install-githooks.sh
```

Before handoff, run:

```bash
cargo run --locked -p xtask -- code-dojo --all
```

Every pull request must add or update a repo-respect receipt under
`.repo-respect/receipts/`, and every non-merge commit must reference that
receipt with a `Repo-Respect-Receipt:` trailer. Create the receipt template
with:

```bash
cargo run --locked -p xtask -- repo-respect receipt <short-topic>
```

Local hooks check staged Rust policy, commit subject shape, universal
repo-respect receipt trailers, branch receipt coverage, topic documentation
contract metadata, and the full pre-push quality gate. The full gate runs
workspace tests, doctests, topic-docs, and repo-respect checks. The hooks are
guardrails; CI remains the final merge gate.

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
3. Update `docs/BEARING.md` only for durable release posture and watchpoints;
   keep branch, PR, CI, and assignment state in GitHub.
4. Update `CHANGELOG.md` if release-visible behavior changed.
5. Start the next goalpost branch from current `main`.

## Release Publication

Release tags must match the publishable workspace crate version exactly:
`v<workspace.package.version>`. The GitHub Release `published` event triggers
`.github/workflows/release.yml`, which verifies package archives and then
publishes all public Bunny crates to crates.io in dependency order.

The release workflow requires the repository secret `CARGO_REGISTRY_TOKEN` with
crates.io publish permission for the Bunny crates. `xtask` is intentionally not
published.

Before publishing a GitHub Release, run the release archive gate locally:

```bash
scripts/publish-crates.sh verify
```

`ALLOW_DIRTY=1` may be used only for local `verify` or `dry-run` diagnostics
while preparing a PR. It passes Cargo's `--allow-dirty` flag to packaging
commands and prints a warning that the mode is local-only. `publish` mode
unconditionally refuses dirty worktrees, even when `ALLOW_DIRTY=1` is set.

For the first crates.io publication of a Bunny version, `verify` fully packages
the independent root crates and checks the package file lists for crates whose
internal Bunny dependencies are not visible in the crates.io index yet. The
`publish` mode still runs `cargo publish` verification for every crate after its
internal dependencies have landed.

Manual workflow dispatch may run `verify`, `dry-run`, or `publish` against an
explicit tag and checkout ref. Before a Bunny version exists in crates.io,
`dry-run` follows the same first-publication split as `verify`; after internal
dependencies exist in the registry, set `VERIFY_REGISTRY_DEPS=1` to dry-run
every crate against the registry. The `publish` mode is idempotent: it skips
crate versions that are already visible and retries dependency-resolution
failures caused by crates.io registry propagation. The normal release path is
still: merge to `main`, tag the verified release commit, publish the GitHub
Release, then let the workflow push the crates.
