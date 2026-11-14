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
| `--rounds <n>` | `5` | Interleaved runs (fresh processes) per side; the report takes the median and the best across them. |
| `--max-regression <percent>` | off | Exit non-zero when any case's best p50 slows by more than this. |
| `--summary <file>` | none | Append the Markdown table (CI passes `$GITHUB_STEP_SUMMARY`). |
| `--align-code` | off | Build both sides with every function and branch target on a 64-byte boundary (see [code-layout noise](#code-layout-noise)). |
| `--filter <text>`, `--samples <n>` | all cases, 101 | Passed through to the harness. |
| `--target <triple>` | the host | Build the harness for another Rust target (see [other targets](#measuring-other-targets)). |
| `--runner <command>` | none | Run the harness through a command, such as `wasmtime`; needed when the target cannot run natively. |

How the comparison stays fair:

- **Same source, same build.** The working tree's harness source is copied twice under
  `target/perf/`: once pointing at the working tree's crates, once at a temporary
  `git worktree` of the base revision. Both are built as standalone crates with the
  repository's pinned toolchain and the release profile, so neither side gets different
  codegen from a workspace build.
- **Same machine, interleaved.** Rounds alternate base and candidate, and alternate which
  runs first, so warm-up and thermal drift favour neither side.
- **Median and best, not one run.** Each round is a fresh process. The table shows the
  median of each percentile across rounds and the best (lowest) p50; the speedup column and
  the regression gate use the best p50 (see [bimodal cases](#bimodal-cases)).
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

### Code-layout noise

A change in one function can move an unrelated hot loop to a different address in the
binary. On modern cores that alone can change the loop's speed by 15 % or more, in either
direction, with no change to the code that runs. Bunny has already met this: when the
square root sped up, `query/ray_aabb` (which takes no square root and reads bit-identical
inputs) measured 15–19 % slower over 7 and 11 rounds. With `--align-code` one run measured
51.19 ns before and 51.15 ns after; alignment removes this layout effect but not the
placement effect described next.

`--align-code` passes `-C llvm-args=-align-all-functions=6` and
`-C llvm-args=-align-all-nofallthru-blocks=6` to both builds, so every function and branch
target starts on a 64-byte boundary and layout luck cannot favour either side. The
trade-off: absolute times under `--align-code` are not what production code sees (the
padding can cost a few nanoseconds per call), so use it for comparisons and treat unaligned
numbers as a second opinion. CI always uses it. Inherited `RUSTFLAGS` are kept and the
alignment flags are appended.

### Bimodal cases

Some kernels change speed from one process to the next with nothing rebuilt. Running the
same `bunny-perf` binary eight times gave `query/ray_aabb` p50s of 39.6, 39.8, 52.4, 40.3,
47.2, 52.0, 52.1 and 51.5 ns, while `query/ray_sphere_miss` stayed between 19.5 and 20.8 ns.
The likely cause is where memory, stack and code land in each process (address
randomization and cache or branch-predictor aliasing), which no compiler flag controls.

A median over a handful of rounds can land in either mode, so the same code can appear
20 % faster or slower. The best p50 across rounds reduces that problem without removing it:
noise only adds time, so more rounds make it likelier that both sides reach the fast mode,
but a side can still miss it in every round. A CI failure on a case whose median and best
disagree sharply, on code the change did not touch, calls for a rerun with more rounds
before it is treated as a regression. That is why speedups and
the CI gate use the best p50, while the median columns stay in the table to show spread.
When a case's median change and best change disagree sharply, suspect placement before
code.

### Measuring other targets

An optimization can win on one CPU and lose on another, because instruction costs differ:
Apple's cores divide 64-bit integers quickly, x86-64 has a native 128-by-64 divide, and
WebAssembly has neither, so 128-bit division is a software routine there. Every target
gets the same bits from Bunny, so only speed differs; the harness can measure any target
it can run.

```sh
rustup target add wasm32-wasip1
brew install wasmtime                     # or the installer at https://wasmtime.dev
cargo run --locked -p xtask -- perf --target wasm32-wasip1 --runner wasmtime --rounds 5
```

The report heading names the target it measured. Under wasmtime, kernels run about three
to four times slower than natively on the same Mac, so compare before and after on one
target, never across targets. `--align-code` is accepted for WebAssembly but means little
there, because wasmtime compiles the module to machine code itself.

Hardware you do not have is measured in CI (below). Running an x86-64 binary under
Rosetta 2 on Apple Silicon works but measures translated code, so it is not used.

### Continuous integration

`.github/workflows/perf.yml` runs the comparison on pull requests that touch crates, the
perf tooling, the lockfile or the toolchain, against the pull request's base branch, with
`--rounds 7 --align-code --max-regression 20`. It writes the table to the job summary and fails when a
case's best p50 slows by more than 20 %. Shared runners are noisier than a quiet laptop;
the threshold is set to catch real regressions, not to referee single-digit changes. The
workflow can also be started by hand (`workflow_dispatch`) or called from another workflow
in this repository (`workflow_call`) with its own `base`, `max-regression` and `rounds`.

A second, report-only job runs the same comparison on GitHub's hosted runners: x86-64
Linux, ARM Linux, x86-64 Windows, Apple Silicon macOS, and WebAssembly under wasmtime on
x86-64 Linux. Each writes its own table to the job summary and none can fail the run on
speed. It runs only when started by hand, to keep runner minutes bounded:

```sh
gh workflow run perf.yml --ref <branch> -f platforms=true -f base=origin/main
```

Hosted runners are shared virtual machines, so treat their tables as coarse: they
reliably show tens of percent, not a few. The WebAssembly job installs wasmtime with the
official `bytecodealliance/actions/wasmtime/setup` action rather than a downloaded script.

### Target-specific implementations

When a measurement shows an exact optimization winning on some targets and losing on
others, Bunny may ship it per target. The rules:

1. **Same bits everywhere.** Only implementations that return identical results may be
   chosen per target. Determinism across targets is the reason Bunny exists.
2. **Choose by target at compile time**, with `cfg(target_arch = "…")` (or `target_os`),
   never with a Cargo feature. Features are additive across a whole dependency graph, so
   one crate enabling a fast path would change it for every other crate; a target choice
   cannot be switched by accident.
3. **Compile and test every implementation on every target.** Only the dispatch is
   conditional. Each implementation stays an ordinary function that the equivalence tests
   call directly, so the path one target does not ship still runs in that target's tests
   and cannot rot.
4. **Measure each claim on its target** with the platform job or `--target`, and record
   the per-target numbers in the ledger entry. No target gets a special path on a guess.

No target-specific implementation exists yet.



### Adding a case

Add a closure to `crates/bunny-perf/src/cases.rs` under the group for the crate it
exercises. A case for a new API goes in a new feature group (declared in the harness
`Cargo.toml` and added to `default`), so comparisons against older revisions still build.
Use seeded inputs only, and return a checksum of the results so the work is not optimized
away.

## Optimization Ledger

Each landed optimization is recorded here with its technique, fidelity, trade-offs and
measured effect. Measurements name the machine and the command that produced them.

### 1. Table-seeded integer square root

**Where:** `FixedQ32_32::sqrt_u128` in `bunny-num`, and so `FixedQ32_32::sqrt`,
`FixedVec3::length`, `FixedVec3::normalize`, `FixedUnitVec3::new`, `FixedRay3::try_new`
and `ray_intersects_sphere` (which takes two square roots per hit: the half chord and the
normal).

**Technique.** The previous implementation found the square root one bit pair at a time:
up to 64 rounds of 128-bit compare, subtract and shift, with a data-dependent branch in
each. It now calls `u128::isqrt` from Rust's core library, which looks up the square root
of the leading bits in a table and then refines it with Karatsuba square-root steps on
halves of the number. Every step is integer arithmetic.

**Fidelity: exact replacement.** Both implementations return the floor of the square root,
the largest `r` with `r * r <= value`, for every `u128`. The old loop is kept in
`crates/bunny-num/tests/sqrt_equivalence_tests.rs` as the reference oracle and compared
against the new code on every power-of-two boundary and its neighbours, perfect squares
and their neighbours, 20 000 seeded inputs spread across all magnitudes, and every raw
Q32.32 value from 0 to 65 536. A one-off local check of 2 000 000 seeded inputs also
matched. No golden vector changed.

**Trade-offs.**

- *Trust moves to the core library.* The square root is no longer Bunny's own code. Its
  result is specified by Rust's documentation (floor of the square root) and has been
  stable and `const` since Rust 1.84, below Bunny's 1.96 MSRV. The retained reference
  tests would catch a future toolchain that changed results; a toolchain could still change
  the speed.
- *No new dependency.* `isqrt` lives in `core`, so it adds nothing for `no_std` or WASM
  builds. Its lookup table is part of `core`, not Bunny.
- *Readability.* One line replaces a 20-line loop. Readers who want the algorithm read the
  Rust source rather than Bunny's.
- *Unmeasured targets.* Results are identical everywhere because the operations are
  integer-only; the speedup has been measured on an Apple M5 Pro only, not on x86-64 or
  WASM.

**Rejected alternatives.**

- *Float-seeded square root with integer correction.* Exact and fast, but it uses
  `f64::sqrt` inside canonical math, which Rule 2 bans.
- *Table-seeded Newton iteration written in Bunny.* Measured at 87 ns against the old
  loop's 60 ns: each Newton step needs a 128-bit division.
- *Fast inverse square root and other approximations.* They change output bits, so they
  are contract changes, not optimizations.

**Measured.** `cargo run --locked -p xtask -- perf --base origin/main --rounds 11
--align-code` on an Apple M5 Pro (macOS, Rust 1.96.0, desktop load), base `9bf43600d08f`
(v0.6.0):

| Case | Before p50 | After p50 | Change | Before best | After best | Best change | Speedup | Before p95 | After p95 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `linalg/length` | 55.34 | 16.64 | -69.9% | 51.27 | 15.30 | -70.2% | 3.35× | 56.36 | 17.37 |
| `linalg/normalize` | 87.85 | 42.24 | -51.9% | 81.91 | 39.31 | -52.0% | 2.08× | 93.42 | 46.55 |
| `num/div` | 6.75 | 6.88 | +1.9% | 6.39 | 6.47 | +1.3% | 0.99× | 7.00 | 7.00 |
| `num/mul` | 1.46 | 1.42 | -2.7% | 1.30 | 1.30 | +0.0% | 1.00× | 1.51 | 1.51 |
| `num/sqrt` | 55.91 | 14.08 | -74.8% | 52.37 | 12.90 | -75.4% | 4.06× | 57.09 | 14.28 |
| `query/ray_aabb` | 51.88 | 40.85 | -21.3% | 42.56 | 37.52 | -11.8% | 1.13× | 55.95 | 42.40 |
| `query/ray_build` | 128.13 | 53.10 | -58.6% | 118.41 | 50.58 | -57.3% | 2.34× | 132.57 | 54.93 |
| `query/ray_sphere_hit` | 116.09 | 67.14 | -42.2% | 111.45 | 65.35 | -41.4% | 1.71× | 119.22 | 72.14 |
| `query/ray_sphere_miss` | 49.11 | 20.92 | -57.4% | 45.90 | 19.57 | -57.4% | 2.35× | 50.17 | 21.12 |
| `query/ray_triangle` | 52.82 | 51.92 | -1.7% | 51.68 | 51.27 | -0.8% | 1.01× | 54.40 | 53.92 |

The square-root kernels speed up 4.1× (`num/sqrt`), 3.4× (`linalg/length`), 2.1×
(`linalg/normalize`), 2.3× (`query/ray_build`, `query/ray_sphere_miss`) and 1.7×
(`query/ray_sphere_hit`). `num/div`, `num/mul`, `query/ray_triangle` and `query/ray_aabb`
take no square root and serve as controls. The first three stay within 1.3 % of their
previous best. `query/ray_aabb` reads 11.8 % faster on unchanged code. It is the
[bimodal case](#bimodal-cases) described above, so that number is not claimed as a speedup.
