# Performance

This chapter states how Bunny measures speed, which rule every optimization must obey,
and what each optimization that has landed cost and bought. It is the current reference;
the [test plan](test-plan.md) says how the measurement tooling itself is verified.

## The Fidelity Rule

Bunny's first rule is bit-level determinism (see `CODE_STANDARDS.md`, Rule 0). Speed never
outranks it, so every optimization falls into exactly one of two kinds:

| Kind | What may change | What must ship with it |
| --- | --- | --- |
| **Exact replacement** | Only speed. Every input produces the same output bits as before, on every supported target. | An equivalence test that keeps the previous implementation as a reference oracle and compares raw outputs over boundaries and seeded inputs. |
| **New approximate API** | Nothing existing. A new function may approximate a mathematical ideal. | A stated error bound in raw units, an oracle test that checks the bound, and golden vectors that lock the exact output bits. |

"Close enough" never replaces an existing result. An approximation that would change an
existing function's bits is a contract change: it needs its own issue, a version bump and
regenerated golden vectors, not a performance pull request.

Two kinds of trick are ruled out in core crates regardless of speed:

- **Floating point in canonical math.** A float-seeded square root can return exact
  integer results, but `CODE_STANDARDS.md` Rule 2 bans float methods in canonical math, so
  the proof burden is not worth taking on.
- **`unsafe`, SIMD intrinsics and platform assembly** (Rule 1). They belong in a quarantined
  backend crate with a safe reference implementation, if ever.

Allowed tricks include lookup tables (integers are the same everywhere), better integer
algorithms, precomputation at compile time, and algebraic rewrites that are proven to give
identical bits.

## Measuring

`crates/bunny-perf` times each kernel on a fixed batch of 1 024 seeded inputs. It runs 10
untimed batches, then 101 timed batches, and reports the 50th, 95th and 99th percentile of
nanoseconds per call. Cases use public APIs only.

`cargo xtask perf` compares two revisions:

```sh
cargo run --locked -p xtask -- perf                      # working tree against origin/main
cargo run --locked -p xtask -- perf --base v0.6.0 --rounds 7
cargo run --locked -p xtask -- perf --filter num/ --max-regression 20
```

| Option | Default | Meaning |
| --- | --- | --- |
| `--base <rev>` | `origin/main` | The "before" revision. Fetch it first. |
| `--rounds <n>` | `5` | Interleaved runs per side; the report takes the median of each percentile. |
| `--max-regression <percent>` | off | Exit non-zero when any case's median p50 slows by more than this. |
| `--summary <file>` | none | Append the Markdown table (CI passes `$GITHUB_STEP_SUMMARY`). |
| `--filter <text>`, `--samples <n>` | all cases, 101 | Passed through to the harness. |

How the comparison stays fair:

- **Same source, same build.** The working tree's harness source is copied twice under
  `target/perf/`: once pointing at the working tree's crates, once at a temporary
  `git worktree` of the base revision. Both are built as standalone crates with the
  repository's pinned toolchain and the release profile, so neither side gets different
  codegen from a workspace build.
- **Same machine, interleaved.** Rounds alternate base and candidate, and alternate which
  runs first, so warm-up and thermal drift favour neither side.
- **Medians, not one run.** Each percentile is the median across rounds.
- **New APIs are not "regressions".** The harness is split into feature groups (`num`,
  `linalg`, `query`, …). When the base revision cannot compile a group (an API it does not
  have yet), that group is left out of the base build and its cases are reported as `new`.

The temporary worktree is removed after the build. Build outputs stay under `target/perf/`.

### Reading the numbers

Results are nanoseconds per call on one machine: compare ratios, not absolute values
across machines. Running identical code on both sides on an Apple M5 Pro laptop under
normal desktop load showed changes within ±3 % for most cases and up to +8 % for the
fastest ones (a few nanoseconds per call), so treat differences under about 10 % as noise
unless they repeat.

### Continuous integration

`.github/workflows/perf.yml` runs the comparison on pull requests that touch crates, the
perf tooling, the lockfile or the toolchain, against the pull request's base branch, with
`--rounds 7 --max-regression 20`. It writes the table to the job summary and fails when a
case's median p50 slows by more than 20 %. Shared runners are noisier than a quiet laptop;
the threshold is set to catch real regressions, not to referee single-digit changes. The
workflow can also be started by hand (`workflow_dispatch`) or called from another workflow
in this repository (`workflow_call`) with its own `base`, `max-regression` and `rounds`.

### Adding a case

Add a closure to `crates/bunny-perf/src/cases.rs` under the group for the crate it
exercises. A case for a new API goes in a new feature group (declared in the harness
`Cargo.toml` and added to `default`), so comparisons against older revisions still build.
Use seeded inputs only, and return a checksum of the results so the work is not optimized
away.

## Optimization Ledger

Each landed optimization is recorded here with its technique, fidelity, trade-offs and
measured effect. Measurements name the machine and the command that produced them.

No optimizations have landed yet.
