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
  integer-only; the speedup has been measured on an Apple M5 Pro (in a Linux container) only, not on
  x86-64 or WASM.

**Rejected alternatives.**

- *Float-seeded square root with integer correction.* Exact and fast, but it uses
  `f64::sqrt` inside canonical math, which Rule 2 bans.
- *Table-seeded Newton iteration written in Bunny.* Measured with the earlier
  harness (not re-measured) as slower than the old loop: each Newton step needs a 128-bit
  division.
- *Fast inverse square root and other approximations.* They change output bits, so they
  are contract changes, not optimizations.

**Measured.** Re-measured on 2026-11-13 with the corrected harness, which passes every
result through `black_box`. Environment: Apple M5 Pro, inside the
`flyingrobots/rust-dev:1.96.0` container (Linux VM, `aarch64-linux`), one run of 11
rounds. Command, run through the guarded container runner with the branch's merge of
`origin/main` as the working tree:

```sh
cargo run --locked -p xtask -- perf --base origin/main --rounds 11 --align-code
```

Base revision `be55424fef16` (`origin/main` after PR #209). This table supersedes the
earlier numbers, which were taken on the host with the older harness that let the
optimizer drop work in some cases; none of those numbers is retained.

| Case | Before p50 | After p50 | Change | Before best | After best | Best change | Speedup | Before p95 | After p95 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `linalg/length` | 70.15 | 15.58 | -77.8% | 63.92 | 14.89 | -76.7% | 4.29× | 79.51 | 17.25 |
| `linalg/normalize` | 86.79 | 39.92 | -54.0% | 80.93 | 38.66 | -52.2% | 2.09× | 110.35 | 42.03 |
| `num/div` | 6.47 | 6.63 | +2.5% | 6.14 | 6.18 | +0.7% | 0.99× | 6.67 | 7.24 |
| `num/mul` | 1.38 | 1.46 | +5.8% | 1.34 | 1.34 | +0.0% | 1.00× | 1.71 | 1.71 |
| `num/sqrt` | 72.10 | 13.35 | -81.5% | 62.54 | 12.82 | -79.5% | 4.88× | 78.65 | 15.79 |
| `query/ray_aabb` | 43.95 | 41.67 | -5.2% | 39.67 | 39.27 | -1.0% | 1.01× | 51.15 | 48.99 |
| `query/ray_build` | 125.33 | 50.21 | -59.9% | 118.49 | 49.60 | -58.1% | 2.39× | 149.17 | 54.40 |
| `query/ray_sphere_hit` | 103.92 | 67.06 | -35.5% | 100.34 | 63.40 | -36.8% | 1.58× | 126.83 | 70.23 |
| `query/ray_sphere_miss` | 51.68 | 19.90 | -61.5% | 45.70 | 19.73 | -56.8% | 2.32× | 57.90 | 19.98 |
| `query/ray_triangle` | 49.44 | 49.68 | +0.5% | 48.42 | 48.71 | +0.6% | 0.99× | 52.78 | 52.98 |

The square-root kernels speed up 4.9× (`num/sqrt`), 4.3× (`linalg/length`), 2.4×
(`query/ray_build`), 2.3× (`query/ray_sphere_miss`), 2.1× (`linalg/normalize`) and 1.6×
(`query/ray_sphere_hit`). `num/div`, `num/mul`, `query/ray_triangle` and `query/ray_aabb`
take no square root and serve as controls. Their best p50 moved by +0.7 %, 0.0 %, +0.6 %
and -1.0 %, and their median p50 by +2.5 %, +5.8 %, +0.5 % and -5.2 %, all inside the
roughly 10 % noise band. `query/ray_aabb`, the [bimodal case](#bimodal-cases), did not
show a layout or placement effect in this run. That does not mean it cannot. This is one
run on one machine under a Linux VM, not an x86-64 or WASM measurement.

### 2. Table-based sine and cosine (new approximate API)

**Where:** `FixedAngle::sin`, `cos` and `sin_cos` in `bunny-num`, new with #110. The full
contract is in the [angle and trigonometry chapter](../angle-trigonometry/README.md).

**Technique.** Angles are binary angles (`2^64` units per turn), so range reduction is a
bit mask. A 1 025-sample quarter-wave sine table with 62 fractional bits is computed at
compile time by an integer Taylor series. Values between samples are cubic Hermite
interpolated, using `cos` (the same table read backwards) as the exact slope, and quadrant
symmetry rebuilds the full wave. This reproduces the structure of Echo's `warp-math`
trigonometry (`f32` table, linear interpolation) with integers only.

**Fidelity: new approximate API.** There was no previous result to preserve. Error bound:
at most one raw Q32.32 unit; see the accuracy evidence below. Quarter turns are exact and
negation symmetry is bit-exact. Twenty golden vectors lock the output bits.

**Trade-offs.** 8.2 KB of read-only table; a little extra compile time for the const
evaluation; four more multiplications per call than linear interpolation, in exchange for
an error about 1 000 times smaller; radians and degrees round once on the way in.

**Measured.** Apple M5 Pro, inside the `flyingrobots/rust-dev:1.96.0` container (the
harness reports `aarch64-linux`), 2026-11-13, with the harness on `main` (every result goes
through `black_box`), base `origin/main` (`be55424fef16`), command:

```sh
cargo run --locked -p xtask -- perf --base origin/main --rounds 11 --align-code
```

The base has no trigonometry, so the angle cases report as `new`. Nanoseconds per call
from two complete runs of that command (p50 and best p50 of 11 rounds each):

| Case | Run 1 p50 | Run 1 best | Run 2 p50 | Run 2 best |
| --- | ---: | ---: | ---: | ---: |
| `angle/sin_cos` | 11.15 | 11.07 | 11.07 | 10.42 |
| `angle/from_radians` | 0.94 | 0.94 | 0.94 | 0.90 |
| `angle/f64_sin_cos_reference` | 4.27 | 4.15 | 4.19 | 4.15 |

`angle/f64_sin_cos_reference` times Rust's `f64::sin_cos` on the same angles in the same
harness and container, as a reference point only (float trigonometry is not canonical).
On these runs `sin_cos` costs about 2.6 times the float routine. Cases unchanged by this
branch (`num/*`, `linalg/*`, `query/*`) moved by up to about 10 percent between the two
sides in the same runs, which is the noise floor of this setup; read the angle figures to
that precision. The numbers recorded here before the harness passed every result through
`black_box` (10.21 ns and 2.81 ns) are superseded and not comparable.

**Accuracy evidence.** The error test (`at_tp_004_error_is_within_one_raw_unit`) compares
against `f64` trigonometry in test code only (Rule 2 bans float methods in canonical math,
not in test oracles; `f64` error is about `4e-7` raw units, far inside the bound) at 20 000
seeded angles, the 4 097 segment-aligned angles of the first quadrant, and five probes in
each of the 4 096 segments (where Hermite error peaks). A float-free check
(`at_tp_004b_pythagorean_identity_holds_within_the_bound`) bounds `sin^2 + cos^2`. A
deliberate mutation of the Hermite basis (`h11 = t3 + t2`) fails the golden, error and
identity tests. The earlier claim of a 0.5001 raw-unit maximum over 4 196 611 angles has no
test or tool in the repository and is withdrawn until it can be reproduced.

## Contract Change Cost Checks

A declared contract change is not an optimization and does not enter the ledger above, but
when it must leave the common path's cost alone, the measurement that shows it is recorded
here.

### `ray_intersects_aabb` overflow handling (flyingrobots/bunny#211)

**Change.** When a coordinate difference or slab distance overflows, `ray_intersects_aabb`
now falls through to an exact `i128` slab test (`crates/bunny-query/src/ray/aabb_wide.rs`,
marked `#[cold]`) instead of returning `None`. The rounded slab test, which answers every
input without overflow, still does the same checked subtractions, divisions and comparisons;
it now reports which of the two stops (zero-component miss or overflow) it hit. See the
[ray-tracing chapter](../ray-tracing/README.md#general-purpose-ray-aabb-solver).

**Measured.** Apple M5 Pro, inside the `flyingrobots/rust-dev:1.96.0` container
(`aarch64-linux`), two complete runs through the guarded container runner with the
branch's commits as history, base revision `2e92ce2748be` (`origin/main`):

```sh
cargo run --locked -p xtask -- perf --base origin/main --rounds 11 --align-code --filter query/
```

| Case | Run 1 p50 before | Run 1 p50 after | Run 1 best change | Run 2 p50 before | Run 2 p50 after | Run 2 best change |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `query/ray_aabb` | 53.47 | 52.90 | +0.2% | 51.15 | 52.49 | -2.7% |
| `query/ray_triangle` (control) | 51.96 | 52.53 | +0.4% | 51.51 | 51.92 | -0.2% |

`query/ray_aabb` moved by +0.2 % and -2.7 % in best p50 and by -1.1 % and +2.6 % in median
p50: inside the noise band, with no consistent direction. Its rays all reach the unit box
without overflow, so the cases time the common path only; the exact path's cost is not
measured and is paid only by inputs that used to return a false miss.
