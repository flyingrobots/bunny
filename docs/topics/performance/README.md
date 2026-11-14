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
  `linalg`, `query`, `scene`, `trace`, `sample`, `build`, `sah`, …). When the base revision cannot compile a group (an API it does not
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

One target-specific choice exists: the integer square root (ledger entry 3). It is a
choice of **core's implementation on every target**, because the alternative measured
slower on aarch64 for `normalize` and is unmeasured elsewhere. Until a target is measured
as a net win, an unmeasured target gets the implementation already shipped.



### Adding a case

Add a closure to `crates/bunny-perf/src/cases.rs` under the group for the crate it
exercises. A case for a new API goes in a new feature group (declared in the harness
`Cargo.toml` and added to `default`), so comparisons against older revisions still build.
Use seeded inputs only, pass every result through `black_box`, and return a checksum of
the results so the work is not optimized away.

The whole-scene cases live in `crates/bunny-perf/src/scene.rs`: a bumpy sphere of 27 648
triangles plus two ground triangles in a BVH from `build_bvh`, seen through 1 024 camera
rays (a 32 by 32 grid). Where a case measures an old API against a new one, the harness
also checks that both give the same answer on every input: `scene/closest_visitor` and
`trace/closest_hit` must choose the same primitive on every ray, and `trace/closest_hit`
and `trace/any_hit` must give the same answers through the `build_bvh` tree as
`trace/closest_hit_sah` and `trace/any_hit_sah` through the `build_bvh_sah` tree. The
harness runs those checks before timing and exits with the differing ray indices if one
fails. Both trees' traversal cases call the same functions, so they run the same machine
code and differ only in the tree. The `build` and `sah` groups also time one whole build
of the scene's tree per call (`build/bvh`, `build/bvh_sah`, into buffers allocated once).

The `sample` group (`crates/bunny-perf/src/sample.rs`) times the `bunny-optics` sample hash
and pixel sampler over 1 024 keys of a 32 by 32 tile, 16 samples and 4 dimensions:
`sample/stream_bits` and `sample/sampler_point` draw from a pixel's held `PixelStream` or
`PixelSampler`, and `sample/sampler_new_point` builds the sampler first. A crate that the
base revision lacks (such as `bunny-optics` before it existed) is left out of the base
manifest, and the groups that need it are reported as `new`.

`bunny-perf --stats` prints, instead of timing, the BVH shape and the per-ray traversal counters of
`closest_hit` and `any_hit` on this scene, split by what the ray hits. They come from a replica of
the search loop in `crates/bunny-perf/src/profile.rs`, built on public APIs only, so the core
crates carry no counters; a test checks that it tests the same primitives in the same order as
the shipped functions on every ray. The same module provides the `trace/kernel_*` cases, which
replay the box and triangle tests the search makes and time them in isolation. See the
[profile](#profile-of-closest_hit-on-the-benchmark-scene).

`trace/triangle_hit` and `trace/triangle_beyond_limit` time `TraceRay::hit_triangle` alone
on the scene: each camera ray that hits something against the triangle it hits first,
repeated to fill a batch, once with no upper limit and once with `t_max` one raw unit below
the hit (the case the traversal meets behind its best hit). The scene's unit tests also
compare every triangle test of `trace/closest_hit` with the frozen reference copy of the
kernel and count how many reach the division (entry 6).

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

### 3. Bunny's own square root, without 128-bit division (retained, not selected)

**Where:** `isqrt_u128_two_divides` in `crates/bunny-num/src/lib.rs`. `FixedQ32_32::sqrt_u128`
calls `isqrt_u128`, which returns core's `u128::isqrt` on every target (entry 1).

**Technique.** Rust 1.96's `u128::isqrt` (`core/src/num/imp/int_sqrt.rs`) runs Karatsuba
square-root stages from a byte lookup table up to 64 bits, then a last stage that divides
`u128 / u128`. aarch64 and wasm32 have no 128-bit divide instruction, so that one division
goes to compiler-rt's software routine, although its divisor is below `2^33`.
`isqrt_u128_two_divides`:

1. hands inputs up to `u64::MAX` to `u64::isqrt`, whose stages divide in hardware;
2. otherwise normalizes by an even shift, takes `u64::isqrt` of the top 64 bits and its
   remainder, and does the same last Karatsuba step as core;
3. performs that step's division as long division in two 31-bit digits, so each step is
   one hardware `u64` divide and no intermediate reaches `2^64`;
4. applies core's correction (`candidate^2 > n` means subtract one).

**Fidelity: exact replacement.** The result is the floor of the square root, identical to
`u128::isqrt`. Unit tests in `crates/bunny-num/src/lib.rs` call it directly on every
target and compare it with core's `isqrt` on 200 000 seeded inputs plus the 64-bit split
and the squares around the quotient-correction points; a third test compares the
dispatched function with it. Core's `isqrt` is in turn compared with the retained
bit-by-bit loop in `crates/bunny-num/tests/sqrt_equivalence_tests.rs`. Replacing the
correction step's `candidate - 1` with `candidate` makes all three unit tests fail.

**Decision: not selected on any target.** `normalize` is slower with this implementation
on aarch64 in every run, beyond the controls' spread, so it is not the default there. No
other target is measured in this PR (x86-64 and wasm32 included), so each keeps core's
implementation, which is what `main` ships. The function stays as an ordinary, tested
function so a later per-target measurement can select it with one `cfg(target_arch)` arm
(rules: [Target-specific implementations](#target-specific-implementations)).

**Measured.** All numbers below are new; they supersede earlier host measurements, none
of which is retained. Machine: Apple M5 Pro, inside the `flyingrobots/rust-dev:1.96.0`
container (aarch64 Linux), 2026-11-13. Base: `origin/main` at `91ed71fc7a5e` (core's
`isqrt`, entry 1). Command, run through `devrun`:

```text
cargo run --locked -p xtask -- perf --base origin/main --rounds 11 --align-code
```

Run 1 (11 rounds):

| Case | Before p50 | After p50 | Change | Before best | After best | Best change | Speedup | Before p95 | After p95 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `linalg/length` | 16.52 | 12.94 | -21.7% | 15.87 | 12.37 | -22.1% | 1.28× | 16.97 | 13.18 |
| `linalg/normalize` | 42.64 | 44.27 | +3.8% | 41.42 | 43.29 | +4.5% | 0.96× | 45.00 | 47.57 |
| `num/div` | 6.79 | 6.80 | +0.1% | 6.63 | 6.51 | -1.8% | 1.02× | 7.08 | 7.04 |
| `num/mul` | 1.46 | 1.42 | -2.7% | 1.42 | 1.42 | +0.0% | 1.00× | 1.46 | 1.46 |
| `num/sqrt` | 14.04 | 10.54 | -24.9% | 13.55 | 10.21 | -24.6% | 1.33× | 14.44 | 10.62 |
| `query/ray_aabb` | 45.25 | 52.78 | +16.6% | 41.30 | 43.54 | +5.4% | 0.95× | 58.80 | 56.03 |
| `query/ray_build` | 54.00 | 56.52 | +4.7% | 51.27 | 54.73 | +6.7% | 0.94× | 56.32 | 60.18 |
| `query/ray_sphere_hit` | 71.17 | 74.02 | +4.0% | 66.85 | 66.94 | +0.1% | 1.00× | 73.85 | 77.03 |
| `query/ray_sphere_miss` | 21.20 | 21.20 | +0.0% | 21.00 | 21.00 | +0.0% | 1.00× | 21.36 | 21.44 |
| `query/ray_triangle` | 52.17 | 52.69 | +1.0% | 51.80 | 52.00 | +0.4% | 1.00× | 55.05 | 54.97 |

Run 2 (11 rounds):

| Case | Before p50 | After p50 | Change | Before best | After best | Best change | Speedup | Before p95 | After p95 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `linalg/length` | 16.56 | 12.90 | -22.1% | 15.91 | 12.37 | -22.3% | 1.29× | 16.93 | 13.26 |
| `linalg/normalize` | 41.83 | 44.31 | +5.9% | 41.10 | 42.52 | +3.5% | 0.97× | 44.15 | 46.18 |
| `num/div` | 6.84 | 6.71 | -1.9% | 6.31 | 6.51 | +3.2% | 0.97× | 6.96 | 6.96 |
| `num/mul` | 1.42 | 1.42 | +0.0% | 1.42 | 1.42 | +0.0% | 1.00× | 1.46 | 1.46 |
| `num/sqrt` | 14.20 | 10.50 | -26.1% | 13.51 | 10.09 | -25.3% | 1.34× | 14.45 | 10.54 |
| `query/ray_aabb` | 53.43 | 52.90 | -1.0% | 39.47 | 43.58 | +10.4% | 0.91× | 59.57 | 56.64 |
| `query/ray_build` | 53.96 | 56.15 | +4.1% | 53.02 | 54.61 | +3.0% | 0.97× | 56.23 | 59.20 |
| `query/ray_sphere_hit` | 71.33 | 72.02 | +1.0% | 66.00 | 68.20 | +3.3% | 0.97× | 73.85 | 76.17 |
| `query/ray_sphere_miss` | 21.12 | 21.08 | -0.2% | 21.08 | 21.04 | -0.2% | 1.00× | 21.24 | 21.16 |
| `query/ray_triangle` | 52.45 | 52.49 | +0.1% | 51.64 | 51.88 | +0.5% | 1.00× | 54.53 | 54.77 |

Run 3 (21 rounds, `--filter linalg/`):

| Case | Before p50 | After p50 | Change | Before best | After best | Best change | Speedup | Before p95 | After p95 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `linalg/length` | 16.60 | 13.02 | -21.6% | 16.24 | 12.86 | -20.8% | 1.26× | 17.13 | 13.71 |
| `linalg/normalize` | 42.48 | 44.35 | +4.4% | 40.77 | 43.01 | +5.5% | 0.95× | 45.12 | 47.16 |

**Reading.** The kernel gain is repeatable: `num/sqrt` 1.33x and 1.34x, `linalg/length`
1.28x, 1.29x and 1.26x (best p50). `linalg/normalize` was slower in all three runs: best
p50 +4.5 %, +3.5 % and +5.5 %, median +3.8 %, +5.9 % and +4.4 %. The controls that take no
square root (`num/mul`, `num/div`, `query/ray_triangle`) moved by at most 3.2 % at best
p50 and 2.7 % at the median, so a 4-5 % loss that repeats in the same direction is
treated as real, though small. `query/ray_build` was also slower in all three runs
(best p50 +6.7 % and +3.0 %; not in run 3's filter). `query/ray_sphere_hit` flipped sign
(+0.1 %, +3.3 %) and `query/ray_aabb`, the [bimodal case](#bimodal-cases), moved +5.4 % and
+10.4 % on unchanged code, so neither is claimed. The cause of the `normalize` loss is
still not established and no experiment here isolated it; a candidate explanation, that
`normalize`'s three software 128-bit divides share code the old square root used to keep
warm, is untested. A table-wide effect (code layout) is not excluded; `--align-code`
reduces it but does not remove it.

**The dispatched build, and an x86-64 regression it first caused.** The first version of
the dispatch (`FixedQ32_32::sqrt_u128` calling `isqrt_u128`, which called `u128::isqrt`)
was claimed to equal `main` but was not measured. The x86-64 Linux CI runner then read
`num/sqrt` **71 % slower** than `main` (best p50 26.13 ns to 44.71 ns, 7 rounds; the other
cases within 3.2 %). The cause is a code-shape difference, not the algorithm: `main`'s
`sqrt_u128` is a plain function that calls `u128::isqrt`, while the branch had marked
`sqrt_u128` itself `#[inline]`, which lets the caller in the benchmark crate inline core's
`isqrt` body and optimize it differently. I did not disassemble the x86-64 output, so the
exact codegen difference is unconfirmed; the evidence is the CI before and after. The fix
removes `#[inline]` from `sqrt_u128` (as on `main`) and leaves a plain `#[inline]` wrapper,
folded into `sqrt_u128` inside the crate.

CI-runner readings (shared GitHub x86-64 runners, 7 rounds, coarse; absolute times differ
between runs because the runners differ), best p50:

| Build | `num/sqrt` | `linalg/length` | `linalg/normalize` |
| --- | ---: | ---: | ---: |
| Before the fix | 26.13 to 44.71 ns (+71.1 %) | +3.2 % | +0.6 % |
| After the fix | 17.74 to 17.75 ns (+0.1 %) | +0.0 % | -0.0 % |

Local confirmation of the final build, `perf --base origin/main --rounds 11 --align-code`
(Apple M5 Pro, container, base `origin/main`): best p50 `num/sqrt` -0.4 %,
`linalg/length` -0.5 %, `linalg/normalize` -0.5 %, `query/ray_build` +0.5 %, controls
within 3.3 %: neutral. The aarch64 build never showed the x86 effect, so it was only visible
on the CI runner; lesson: measure the final dispatched build on each target before claiming
it matches `main`.

**Not measured:** wasm32, and local x86-64.

### 4. Closest-hit traversal with pruning (new API)

**Where:** `closest_hit`, `any_hit` and `BvhView` in `bunny-broadphase`, over the
`TraceRay` and `TraceTriangle` kernels in `bunny-query`. The full contract and
correctness argument are in the [ray-tracing chapter](../ray-tracing/README.md#closest-hit-traversal).

**Technique.** The older way to find the nearest triangle (`scene/closest_visitor`) visits
every primitive whose box the ray's line touches, in tree order, tests each with
`ray_intersects_triangle` and keeps the nearest. The new path walks the tree front to back
and skips any subtree that starts beyond the best hit found so far. Its box test is
division-free and conservative, with one reciprocal per ray, and its triangle test is an
exact integer test that divides only on a hit.

**Fidelity: new APIs.** No existing function changed. What is exact and what is a stated
bound is defined in the ray-tracing chapter, and is not restated here. The harness
confirms that on this scene the two paths choose the same primitive on all 1 024 rays.

**Trade-offs.**

- *Domain limit.* The trace kernels accept coordinates below 256 units only.
- *Two triangle tests.* `ray_intersects_triangle` and `TraceRay::hit_triangle` now both
  exist and round differently, so a caller cannot assume they agree on every ray; the harness checks only this scene's rays.
- *Distance units.* Distances are multiples of the ray's direction, which is not a unit
  vector.

**Measured.** Apple M5 Pro, inside the `flyingrobots/rust-dev:1.96.0` container
(`aarch64-linux`), 2026-11-13, two complete runs of:

```sh
cargo run --locked -p xtask -- perf --base origin/main --rounds 11 --align-code
```

Base revision `e4e4ee643cc5` (`origin/main`), which already contains `closest_hit` and
`intersect_ray`, so both sides built every feature group and ran the same source. The
comparison below is therefore **old API against new API within one run, on the same
binary and the same rays**. It is not a before-and-after of one function. The
`origin/main` against working tree columns serve as the noise reference (the `ray_intersects_aabb`
overflow change of #211 is in both revisions). Best p50, nanoseconds per ray:

| Case | Run 1 base | Run 1 head | Run 2 base | Run 2 head |
| --- | ---: | ---: | ---: | ---: |
| `scene/closest_visitor` (old APIs) | 3183.93 | 3181.00 | 3174.70 | 3182.75 |
| `trace/closest_hit` (new API) | 221.52 | 223.11 | 220.26 | 223.11 |
| `trace/any_hit` | 206.10 | 205.49 | 205.93 | 207.68 |
| `trace/ray_prepare` (per ray) | 1.99 | 1.99 | 1.99 | 1.99 |

Within each side of each run, `scene/closest_visitor` costs 14.3 to 14.4 times as much as
`trace/closest_hit` (3183.93 / 221.52 = 14.4, 3181.00 / 223.11 = 14.3, 3174.70 / 220.26 =
14.4, 3182.75 / 223.11 = 14.3). The median p50 ratios are 14.3 and 14.2 on the head
side. The same cases moved by -0.3 % to +1.3 % between base and head, and the existing
cases by at most about 1 percent except `query/ray_aabb`, the [bimodal case](#bimodal-cases)
(-11.1 % in run 1, +6.0 % in run 2, with no code change).

Full table of run 1 (p50 and best p50, nanoseconds per call; run 2 agreed within the same
noise, and its scene and trace rows are in the table above):

| Case | Before p50 | After p50 | Before best | After best |
| --- | ---: | ---: | ---: | ---: |
| `angle/sin_cos` | 11.11 | 11.11 | 11.07 | 11.11 |
| `linalg/length` | 16.15 | 16.56 | 15.75 | 15.87 |
| `linalg/normalize` | 42.20 | 41.99 | 41.26 | 41.22 |
| `num/div` | 6.59 | 6.51 | 6.43 | 6.47 |
| `num/mul` | 1.42 | 1.42 | 1.42 | 1.42 |
| `num/sqrt` | 14.08 | 14.04 | 13.43 | 13.39 |
| `query/ray_aabb` | 53.96 | 54.97 | 53.22 | 47.32 |
| `query/ray_build` | 52.33 | 52.04 | 50.66 | 50.42 |
| `query/ray_sphere_hit` | 70.64 | 67.38 | 64.98 | 65.67 |
| `query/ray_sphere_miss` | 21.20 | 21.20 | 21.08 | 21.12 |
| `query/ray_triangle` | 52.73 | 53.34 | 52.41 | 52.41 |
| `scene/closest_visitor` | 3229.14 | 3226.21 | 3183.93 | 3181.00 |
| `trace/any_hit` | 208.78 | 210.37 | 206.10 | 205.49 |
| `trace/closest_hit` | 225.75 | 225.51 | 221.52 | 223.11 |
| `trace/ray_prepare` | 1.99 | 1.99 | 1.99 | 1.99 |

**One scene is one scene.** The ratio holds for a 27 650-triangle bumpy sphere with a
ground plane, seen from one camera position through 1 024 rays, more than 600 of which hit the
mesh. Another scene, ray distribution or tree shape can give a different ratio and do not
follow from this one.

**Unmeasured.** x86-64 and wasm32 (the CI platform job covers them only when started by
hand), other scenes, other ray batches, thread scaling, and `closest_hit` against
`intersect_ray` with a pruning triangle test (the old path here never prunes).

### 5. Prepared divisors and prepared rays (new exact APIs)

**Where:** new `bunny_num::FixedDivisor`, `bunny_query::PreparedRay3` and
`bunny_query::ray_intersects_aabb_prepared`. Existing functions are unchanged.

**Technique.** Division by an invariant integer (Möller and Granlund, "Improved Division
by Invariant Integers", IEEE Transactions on Computers 60(2), 2011, Algorithm 4).
`FixedDivisor::new` normalizes the divisor's magnitude so its top bit is set and computes
one 64-bit reciprocal with a single 128-bit division. Each later division of a Q32.32
dividend becomes one or two two-word steps, each one 64×64→128 multiplication, a few
additions and at most two corrections; quotients small enough for one word take one step.
The remainder comes out exact, so ties-to-even rounding and the overflow decision are the
same as `FixedQ32_32::checked_div`.

`PreparedRay3` holds one `FixedDivisor` per nonzero direction component, so the slab test
in `ray_intersects_aabb_prepared` multiplies where `ray_intersects_aabb` divides. When a
slab distance or coordinate difference does not fit Q32.32 it calls the same exact wide
path as `ray_intersects_aabb` (`aabb_wide::exact_slab_hit`), so the two agree there too.

**Fidelity: exact, as new APIs.** `divisor.checked_div(a)` returns exactly
`a.checked_div(divisor)`, `None` included, and `ray_intersects_aabb_prepared` returns exactly
what `ray_intersects_aabb` returns for the same ray and box, the overflow inputs of
flyingrobots/bunny#211 included. Evidence (PERF-TP-034 to PERF-TP-041): boundary values,
every power-of-two divisor with both signs, 100 000 seeded pairs across all magnitudes,
10 000 constructed exact ties, 160 000 seeded ray and box pairs including axis-parallel
rays, up to 240 000 pairs from an overflow-heavy corpus (coordinates near the `i64` extremes,
direction components of a few raw units; the test asserts more than 100 000 pairs, more than
5 000 hits, more than 5 000 misses and more than 1 000 hits with a saturated bound) and one
constructed case answered by the wide path. Deliberate mutation: discarding the wide path's
hit on overflow (`.filter(|_| false)`) fails both overflow tests.

**Measured.** Apple M5 Pro, inside the `flyingrobots/rust-dev:1.96.0` container
(`aarch64-linux`), through the guarded container runner with the branch's commits as
history and the machine to itself (`--exclusive`), on 2026-11-13, base revision
`cc0521e15084` (`origin/main`). Two complete runs:

```sh
cargo run --locked -p xtask -- perf --base origin/main --rounds 11 --align-code
```

Run 1:

| Case | Before p50 | After p50 | Change | Before best | After best | Best change | Speedup | Before p95 | After p95 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `angle/f64_sin_cos_reference` | 4.19 | 4.23 | +1.0% | 4.11 | 4.19 | +1.9% | 0.98× | 4.80 | 5.09 |
| `angle/from_radians` | 0.94 | 0.94 | +0.0% | 0.89 | 0.94 | +5.6% | 0.95× | 0.94 | 0.94 |
| `angle/sin_cos` | 11.07 | 11.15 | +0.7% | 10.42 | 10.46 | +0.4% | 1.00× | 11.11 | 11.19 |
| `linalg/length` | 16.11 | 16.24 | +0.8% | 15.46 | 15.62 | +1.0% | 0.99× | 16.60 | 16.76 |
| `linalg/normalize` | 41.71 | 41.91 | +0.5% | 38.98 | 39.14 | +0.4% | 1.00× | 45.12 | 43.82 |
| `num/div` | 6.59 | 6.63 | +0.6% | 6.27 | 6.43 | +2.6% | 0.98× | 6.84 | 6.80 |
| `num/mul` | 1.38 | 1.42 | +2.9% | 1.34 | 1.34 | +0.0% | 1.00× | 1.46 | 1.46 |
| `num/sqrt` | 13.59 | 13.88 | +2.1% | 13.31 | 13.27 | -0.3% | 1.00× | 14.16 | 14.12 |
| `prepared/div` | — | 4.27 | new | — | 3.99 | new | — | — | 4.44 |
| `prepared/ray_aabb` | — | 36.30 | new | — | 21.73 | new | — | — | 39.10 |
| `prepared/ray_prepare` | — | 26.45 | new | — | 26.00 | new | — | — | 26.57 |
| `query/ray_aabb` | 55.09 | 54.24 | -1.5% | 52.29 | 52.25 | -0.1% | 1.00× | 61.28 | 56.88 |
| `query/ray_build` | 51.96 | 52.21 | +0.5% | 47.93 | 50.25 | +4.8% | 0.95× | 54.36 | 55.09 |
| `query/ray_sphere_hit` | 65.63 | 68.60 | +4.5% | 62.34 | 61.00 | -2.1% | 1.02× | 71.82 | 71.90 |
| `query/ray_sphere_miss` | 21.16 | 21.20 | +0.2% | 19.86 | 19.78 | -0.4% | 1.00× | 21.28 | 21.24 |
| `query/ray_triangle` | 52.61 | 52.41 | -0.4% | 52.49 | 51.80 | -1.3% | 1.01× | 55.26 | 54.97 |
| `scene/closest_visitor` | 3167.16 | 3189.79 | +0.7% | 3125.86 | 3149.30 | +0.7% | 0.99× | 3229.30 | 3326.10 |
| `trace/any_hit` | 206.50 | 208.90 | +1.2% | 205.12 | 204.75 | -0.2% | 1.00× | 222.66 | 223.06 |
| `trace/box_entry` | 6.02 | 6.02 | +0.0% | 5.62 | 5.98 | +6.4% | 0.94× | 6.06 | 6.06 |
| `trace/box_ray_prepare` | 1.91 | 1.95 | +2.1% | 1.75 | 1.83 | +4.6% | 0.96× | 1.95 | 1.99 |
| `trace/closest_hit` | 224.65 | 224.57 | -0.0% | 220.87 | 221.64 | +0.3% | 1.00× | 246.83 | 236.70 |
| `trace/ray_prepare` | 1.99 | 2.03 | +2.0% | 1.87 | 1.87 | +0.0% | 1.00× | 2.04 | 2.04 |

Run 2:

| Case | Before p50 | After p50 | Change | Before best | After best | Best change | Speedup | Before p95 | After p95 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `angle/f64_sin_cos_reference` | 4.19 | 4.19 | +0.0% | 4.11 | 4.19 | +1.9% | 0.98× | 4.84 | 5.13 |
| `angle/from_radians` | 0.94 | 0.94 | +0.0% | 0.94 | 0.94 | +0.0% | 1.00× | 0.94 | 0.94 |
| `angle/sin_cos` | 11.11 | 11.11 | +0.0% | 11.07 | 11.07 | +0.0% | 1.00× | 11.15 | 11.19 |
| `linalg/length` | 16.36 | 16.36 | +0.0% | 15.75 | 15.18 | -3.6% | 1.04× | 16.68 | 16.68 |
| `linalg/normalize` | 42.56 | 42.40 | -0.4% | 40.69 | 41.02 | +0.8% | 0.99× | 44.68 | 43.66 |
| `num/div` | 6.63 | 6.71 | +1.2% | 6.47 | 6.47 | +0.0% | 1.00× | 6.84 | 6.92 |
| `num/mul` | 1.42 | 1.42 | +0.0% | 1.42 | 1.34 | -5.6% | 1.06× | 1.46 | 1.46 |
| `num/sqrt` | 14.08 | 14.16 | +0.6% | 12.70 | 13.67 | +7.6% | 0.93× | 14.16 | 14.28 |
| `prepared/div` | — | 4.03 | new | — | 3.50 | new | — | — | 4.23 |
| `prepared/ray_aabb` | — | 23.36 | new | — | 21.53 | new | — | — | 23.72 |
| `prepared/ray_prepare` | — | 26.33 | new | — | 26.04 | new | — | — | 27.14 |
| `query/ray_aabb` | 54.28 | 53.30 | -1.8% | 51.64 | 51.47 | -0.3% | 1.00× | 60.71 | 59.86 |
| `query/ray_build` | 51.76 | 51.39 | -0.7% | 50.33 | 50.66 | +0.7% | 0.99× | 54.40 | 54.04 |
| `query/ray_sphere_hit` | 66.90 | 66.32 | -0.9% | 64.86 | 64.86 | +0.0% | 1.00× | 71.66 | 70.56 |
| `query/ray_sphere_miss` | 21.20 | 21.16 | -0.2% | 21.04 | 21.04 | +0.0% | 1.00× | 21.32 | 21.24 |
| `query/ray_triangle` | 52.86 | 52.61 | -0.5% | 52.45 | 52.29 | -0.3% | 1.00× | 55.42 | 55.01 |
| `scene/closest_visitor` | 3177.54 | 3200.65 | +0.7% | 3146.78 | 3159.27 | +0.4% | 1.00× | 3260.38 | 3257.05 |
| `trace/any_hit` | 210.08 | 210.74 | +0.3% | 207.11 | 206.95 | -0.1% | 1.00× | 227.34 | 232.54 |
| `trace/box_entry` | 5.98 | 6.02 | +0.7% | 5.98 | 5.98 | +0.0% | 1.00× | 6.02 | 6.02 |
| `trace/box_ray_prepare` | 1.87 | 1.91 | +2.1% | 1.83 | 1.83 | +0.0% | 1.00× | 1.91 | 1.95 |
| `trace/closest_hit` | 226.97 | 224.73 | -1.0% | 222.37 | 221.92 | -0.2% | 1.00× | 251.71 | 251.59 |
| `trace/ray_prepare` | 1.99 | 1.99 | +0.0% | 1.99 | 1.99 | +0.0% | 1.00× | 2.04 | 2.04 |

`prepared/*` report as new because the base has no such API. Best p50 in nanoseconds, run 1
and run 2, for the cases that time the same rays and box (`query/ray_aabb`,
`prepared/ray_aabb`, `trace/box_entry`):

| Operation | Run 1 | Run 2 |
| --- | ---: | ---: |
| `num/div` (`checked_div`) | 6.43 (base 6.27) | 6.47 (base 6.47) |
| `prepared/div` | 3.99 | 3.50 |
| `query/ray_aabb` (exact distances) | 52.25 (base 52.29) | 51.47 (base 51.64) |
| `prepared/ray_aabb` (the same exact bits) | 21.73 | 21.53 |
| `prepared/ray_prepare` (once per ray) | 26.00 | 26.04 |
| `trace/box_entry` (conservative, no distances) | 5.98 (base 5.62) | 5.98 (base 5.98) |
| `trace/box_ray_prepare` (once per ray) | 1.83 (base 1.75) | 1.83 (base 1.83) |

A prepared division is 1.6 to 1.8 times faster than `checked_div` (best p50); a prepared box
test is 2.4 times faster than `ray_intersects_aabb` in the same run, and with its 26 ns
preparation it is level at one box (26 + 22 = 48 ns against 52 ns) and ahead from the second.
`trace/box_entry` takes 6.0 ns after a 1.8 ns preparation: 3.6 times faster than the prepared
exact test per box and 14 times cheaper to prepare. Two things limit these readings.
`prepared/ray_aabb` and `prepared/div` medians are bimodal (36.30 and 23.36 ns median
against 21.73 and 21.53 best for the box test), the placement sensitivity described under
[bimodal cases](#bimodal-cases), so best p50 is used. And `query/ray_aabb`, whose code has
not changed, measured about 40 ns best p50 in two earlier runs against base `b2fe36638429`
and measures about 52 ns here against base `cc0521e15084`, in both the base and the
branch build: its absolute time depends on the build's code layout, so ratios are taken
within one run only, and no absolute `query/ray_aabb` figure is claimed. Every existing case
is within 8 % at best p50 in both runs and shows no consistent direction, and
`FixedQ32_32::sqrt_u128` is byte-identical to `main`'s, so the square root dispatch from
entry 3 is not re-selected. This run's `origin/main` could not build most groups alone
(the `case` helper compiled only with the `scene` or `trace` group); the commit
"bunny-perf: compile the case helper for every feature group" repairs that, and an earlier
attempt without it reported every `num`, `linalg`, `query` and `angle` case as new.

Two earlier complete runs of this branch against base `b2fe36638429`, before main added
closest-hit traversal, gave the same picture: best p50 `prepared/div` 3.46 and 3.42 ns,
`prepared/ray_aabb` 21.81 and 21.00 ns, `prepared/ray_prepare` 26.12 and 26.29 ns,
`trace/box_entry` 5.98 ns, with `num/div` 6.75 and 6.71 ns and `query/ray_aabb` 39.92 and
40.97 ns in the same runs.

The numbers of the earlier version of this entry (host measurement, older harness, stacked
on the two-divide square root) are superseded and not carried over.

**x86-64 (the pull request's `Perf / before and after` job, GitHub `ubuntu-24.04` runner,
shared and coarse, 7 rounds, base `origin/main`).** The gate passed: no existing case moved
by more than 2.5 % at best p50 (`num/sqrt` -2.5 %, `linalg/length` +1.4 %, the controls
within 1.1 %). The new cases are much less favourable to the exact prepared APIs than on the
M5. Best p50 in nanoseconds:

| Operation | x86-64 CI | M5 Pro (two runs) |
| --- | ---: | ---: |
| `num/div` | 9.92 | 6.43, 6.47 |
| `prepared/div` | 7.97 | 3.99, 3.50 |
| `query/ray_aabb` | 68.76 | 52.25, 51.47 |
| `prepared/ray_aabb` | 62.06 | 21.73, 21.53 |
| `prepared/ray_prepare` | 18.75 | 26.00, 26.04 |
| `trace/box_entry` | 15.52 | 5.98, 5.98 |
| `trace/box_ray_prepare` | 8.05 | 1.83, 1.83 |

On x86-64 a prepared division is 1.25 times faster than `checked_div` and a prepared box
test only 1.11 times faster than `ray_intersects_aabb`, so with its preparation it pays off
only from a ray's third box (18.75 + 2 x 62.06 = 142.9 against 137.5 ns at two boxes, 205.0
against 206.3 ns at three). `trace/box_entry` is 4.4 times faster than the exact box test
there. The speed of the prepared exact API is therefore target-dependent and modest where
the hardware divides quickly; the
[target-specific rules](#target-specific-implementations) do not apply because the prepared
path is a separate API, not a replacement.

**Which API suits which use.**

- *Exact `(enter, exit)` distances, bit for bit as `ray_intersects_aabb`, for many boxes
  per ray* (for example a caller of `bunny_broadphase::intersect_ray` that wants the
  existing results faster): `PreparedRay3` with `ray_intersects_aabb_prepared`.
- *Division by one divisor many times, exactly as `checked_div`:* `FixedDivisor`.
- *Ray tracing and closest-hit traversal* (deciding whether a node can contain a hit and
  ordering nodes by a lower bound on the entry distance): `TraceRay::box_entry`. It is
  faster than the prepared exact test per box and per ray. It does not return the exit
  distance, may report a box a ray narrowly misses, and limits inputs to the trace domain.
  `PreparedRay3` does not speed up a ray tracer that can use it, and this entry does not
  claim it does.
- *A single division or a single box per ray:* `checked_div` and `ray_intersects_aabb`.

**Does the exact prepared API still earn its place?** `FixedDivisor` does: nothing else in
Bunny offers exact repeated division, and it is 1.6 to 1.8 times faster. `PreparedRay3` is
narrower than when it was written. Its use is an exact box test that is faster than
`ray_intersects_aabb`, and nothing in the workspace calls it yet; the traversals added since
use `box_entry`. It stays as a new API with an oracle test. If the maintainer prefers not
to keep an exact box API without a caller, the cheapest course is to keep `FixedDivisor`,
drop `PreparedRay3` and `ray_intersects_aabb_prepared`, and move this experiment's box part
to "Tried And Rejected"; the pull request states the same recommendation. Switching
`intersect_ray` to the prepared test would keep its output bits and is a separate change
that needs its own measurement.

**Trade-offs.**

- *Preparation costs a division per axis.* `PreparedRay3::new` takes about 26 ns; a single
  division should still use `checked_div`.
- *More state per ray.* `PreparedRay3` carries three divisors (25 bytes each, with padding)
  next to the ray.
- *Subtle arithmetic.* Algorithm 4's correction steps are easy to get wrong; the
  equivalence tests against `checked_div` are the guard.
- *Not used inside `normalize`.* Preparing a divisor costs about as much as the three
  divisions it would replace there.

### 8. Binned SAH BVH builder (new API)

**Where:** `build_bvh_sah` in `bunny-broadphase` (flyingrobots/bunny#213). The contract is in
the [BVH construction chapter](../bvh-construction/README.md#the-binned-sah-builder).

**Technique.** `build_bvh` tries three planes per axis at the quartiles of the centroid
span, with a full pass over the node's primitives for each. `build_bvh_sah` counts
centroids into 16 equal-width bins per axis in one pass, sweeps the bins from both ends to
cost all 15 planes per axis, and compares the cheapest with the leaf cost, using the
surface area heuristic with unit traversal and intersection weights. Nodes of up to four
primitives become leaves when splitting does not pay. Costs are `u128` integers.

**Fidelity: new API.** `build_bvh` keeps every output bit: its source changed only in the
visibility of four helpers, its earlier tests pass unmodified, and a new golden fingerprint
locks its bytes. The SAH tree is a deterministic function of the input with stated tie
rules (lower axis, then lower plane; an equal split and leaf cost makes a leaf; a stable
partition), locked by its own golden fingerprint on every CI platform. `closest_hit` and
`any_hit` return the same answers on both trees, by the
[tree-independence argument](../ray-tracing/README.md#correctness-the-answer-does-not-depend-on-the-tree);
the harness checks it on all 1 024 benchmark rays before timing, and the broadphase tests
against a test of every primitive on seeded scenes.

**Choosing the parameters.** Exploratory sweeps, each variant a fresh build of the harness
run three times inside one `--exclusive` container run (Apple M5 Pro, `aarch64-linux`,
alignment flags on, not through `xtask perf`). Median of the three p50s, nanoseconds per
ray; the quartile-tree case ran in the same process as the reference:

| Leaf size | Bins | `closest_hit` quartile | `closest_hit` SAH | ratio | `any_hit` quartile | `any_hit` SAH | ratio | SAH build |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 16 | 227.50 | 230.51 | 1.013 | 211.47 | 216.23 | 1.023 | 18.1 ms |
| 2 | 16 | 226.93 | 210.32 | 0.927 | 209.59 | 195.47 | 0.933 | 17.8 ms |
| 4 | 16 | 225.02 | 208.45 | 0.926 | 207.97 | 194.74 | 0.936 | 17.5 ms |
| 8 | 16 | 225.79 | 209.92 | 0.930 | 207.84 | 197.14 | 0.949 | 17.6 ms |
| 4 | 8 | 227.95 | 212.57 | 0.933 | 208.70 | 198.45 | 0.951 | 12.7 ms |
| 4 | 16 | 222.25 | 209.35 | 0.942 | 209.55 | 196.66 | 0.938 | 17.5 ms |
| 4 | 32 | 225.59 | 206.46 | 0.915 | 208.46 | 193.12 | 0.926 | 26.1 ms |

Weights other than 1 : 1 were no better: traversal 1 and intersection 2 gave ratios 0.955
and 0.967, traversal 2 and intersection 1 gave 0.983 and 0.922 (leaf size 4, 16 bins).
Leaf size 1 (always split) loses the gain; sizes 2, 4 and 8 are within the noise of each
other, and 4 was best or tied in both runs. 32 bins traced about 1 to 2 % faster than 16,
inside the noise, at 1.5 times the build time; 8 bins built faster and traced no better.
Chosen: leaf size 4, 16 bins, weights 1 and 1.

**Measured.** Apple M5 Pro, inside the `flyingrobots/rust-dev:1.96.0` container
(`aarch64-linux`), 2026-11-13, two complete runs with `--exclusive` (no other container run
overlapped) of:

```sh
cargo run --locked -p xtask -- perf --base origin/main --rounds 11 --align-code
```

Run 1 used base `8cdbb1cb38f5` and the branch before its last merge of `origin/main`; run 2
used base `fd69f19` (`origin/main`, the 0.7.0 release preparation, which changed versions
and documentation only) and the final branch. The base cannot build `build_bvh_sah`, so the
SAH cases are reported as `new`; the comparison that matters is quartile tree against SAH
tree within one run, on the same binary, the same rays and the same traversal functions.
After run 2, `origin/main` gained the `sample` group and the `trace/triangle_*` cases
(flyingrobots/bunny#214 and the optics sampler), which change no function these cases
call; the branch was merged with it and not re-timed locally.
Best p50 (nanoseconds per ray; per build for `build/`):

| Case | Run 1 base | Run 1 head | Run 2 base | Run 2 head |
| --- | ---: | ---: | ---: | ---: |
| `trace/closest_hit` (quartile tree) | 221.15 | 220.26 | 221.07 | 219.24 |
| `trace/closest_hit_sah` (SAH tree) | new | 205.20 | new | 204.14 |
| `trace/any_hit` (quartile tree) | 206.70 | 205.12 | 205.97 | 204.79 |
| `trace/any_hit_sah` (SAH tree) | new | 188.68 | new | 188.44 |
| `build/bvh` | 167.29 ms | 167.53 ms | 166.28 ms | 166.01 ms |
| `build/bvh_sah` | new | 17.44 ms | new | 17.39 ms |
| `scene/closest_visitor` (quartile tree) | 3175.99 | 3179.41 | 3162.15 | 3183.06 |

Within the head side of each run, `closest_hit` costs 205.20 / 220.26 = 0.932 and
204.14 / 219.24 = 0.931 on the SAH tree, 15.1 and 15.1 ns less per ray (6.9 %); `any_hit`
costs 188.68 / 205.12 = 0.920 and 188.44 / 204.79 = 0.920, 16.4 and 16.4 ns less (8.0 %).
The median p50 ratios agree (closest 210.65 / 225.18 = 0.935 and 207.97 / 223.88 = 0.929;
any 193.31 / 207.85 = 0.930 and 192.42 / 209.02 = 0.921). Base against head on the
unchanged cases is the noise reference: `trace/closest_hit`, `trace/any_hit`, `build/bvh`
and `scene/closest_visitor` moved by -0.8 % to +0.7 % in best p50. The kernel cases of the
other groups moved by at most 6.7 % in best p50 (`num/div`, run 2; its median moved
-1.2 %), except `prepared/div`, +17.4 % in best p50 in run 1 with its median unchanged
(4.23 ns on both sides) and -3.4 % in run 2: a lucky base round, not a change. The exception is the three `trace/kernel_tri_*` replays, which were 7 to 14 %
slower on the head side in both runs (best p50 +13.2 %, +7.0 %, +9.2 % and +13.7 %, +8.9 %,
+8.5 %): see the note below.

**The `trace/kernel_tri_*` shift.** Those cases replay the quartile tree's recorded triangle
tests through `TraceRay::hit_triangle` in `bunny-query`, which this branch does not touch,
and the harness source is the same on both sides. The difference between the two binaries
is that the head harness also compiles the `build` and `sah` groups, which the base cannot
build. To test whether that alone moves them, the head harness was built
in the workspace twice, with and without the `build` and `sah` groups (alignment flags on),
and the two binaries were run alternately three times with `--filter trace/kernel_tri`
inside one `--exclusive` container run: p50s were 18.84 to 19.45, 29.01 to 32.31 and 45.49
to 50.13 ns, with no consistent difference between the binaries. So the shift was not
reproduced outside `xtask perf`'s standalone builds, and its cause is not established.
Binary layout or placement (see [code-layout noise](#code-layout-noise) and
[bimodal cases](#bimodal-cases)) is the suspect, because no code those cases run changed;
that is a suspicion, not a finding. The cases replay the quartile tree's calls, so they do
not bear on the SAH comparison above.

**x86-64 CI reading.** The pull request's `Perf / before and after` job (GitHub's shared
x86-64 Linux runner, 7 rounds, `--align-code`, base `cda15ecc020a`) read, best p50:
`trace/closest_hit` 411.54 ns against `trace/closest_hit_sah` 386.45 ns (0.939),
`trace/any_hit` 371.78 against `trace/any_hit_sah` 347.00 (0.933), and `build/bvh`
248.5 ms against `build/bvh_sah` 30.2 ms (8.2 times faster). Base against head moved the
unchanged trace, build and scene cases by -0.2 % to +0.5 %, and the `trace/kernel_tri_*`
replays by -0.2 % to +1.4 %: the shift seen on aarch64 did not appear there. Shared runners
are coarse; this is one reading.

**Why the gain is small, from the traversal counters.** `bunny-perf --stats` now prints the
profile tables for both trees (per ray, averaged over the 1 024 `closest_hit` searches):

| | quartile tree | SAH tree |
| --- | ---: | ---: |
| nodes | 55 277 | 29 279 |
| leaf sizes (primitives: leaves) | 1: 27 628, 2: 11 | 1: 2 473, 2: 11 388, 3: 715, 4: 64 |
| maximum / mean leaf depth | 19 / 15.44 | 18 / 14.93 |
| interior area / root area (SAH traversal term) | 3.13 | 2.17 |
| leaf area times count / root area (intersection term) | 1.86 | 1.87 |
| nodes popped | 11.37 | 10.20 |
| `box_entry` calls | 19.76 | 17.89 |
| interior nodes visited by sphere rays | 24.14 | 21.49 |
| triangle tests | 1.63 | 1.91 |
| depth of the two ground triangles' leaves | 3 | 2 |

The SAH tree has 9.5 % fewer box tests and 10 % fewer pops, and 17 % more triangle tests,
because leaves hold two primitives on average. At the profile's costs (about 6.5 ns per box
test, about 4 ns per pop, 16 to 32 ns per triangle test) that is roughly 12 ns saved, 6 ns
of pops saved and 5 to 9 ns of triangle tests added, close to the 14 to 16 ns measured.
The SAH cost fell by 19 % (4.99 to 4.04 root areas), almost all of it in the traversal term,
but the ray cost fell by 7 %: the root box is dominated by the 40-unit ground triangles, so
the area weights describe rays spread uniformly over that box, not camera rays aimed at a
unit sphere. On the near-uniform tessellated sphere the quartile split is already close to
the median split SAH would choose, so the sphere rays still make 21.5 interior visits where
one descent takes about 15. The profile's ceiling for a builder on this scene was 30 to
35 ns; this one gets about half of it.

**Build time.** `build_bvh_sah` builds the scene's tree in about 17.5 ms against 167 to
169 ms for `build_bvh`, 9.6 times faster: one binning pass per node instead of nine full
passes, and half as many nodes. The stable partition sorts both sides of every split in
place, which is included.

**Quality independent of timing.** SAH cost (the builder's own weights, exact areas, per
1 000 root areas): benchmark scene 4 984 (quartile) and 4 038 (SAH); seeded test scenes
from `bvh_tp_015`: 600 scattered boxes 16 672 and 16 175, clustered boxes 3 233 and 3 228,
a grid on two huge ground boxes 2 935 and 2 935 (lower by less than one part in a
thousand). The cost is the SAH builder's own objective, so a lower value is expected, not
independent evidence that rays get faster; the timings above are the evidence for this
scene.

**Trade-offs.**

- *Different trees.* `intersect_ray` and `intersect_aabb` report primitives in a different
  order on SAH trees, and leaves hold up to four primitives, so a caller that relied on the
  quartile tree's order or on one primitive per leaf must not switch silently.
- *Memory.* 29 279 nodes of 56 bytes (1.6 MB) instead of 55 277 (3.1 MB); the buffers the
  caller provides are the same size (2n - 1 nodes).
- *More triangle tests per ray*, which a scene with a costlier primitive test would feel.
- *Depth cap.* A node at depth 60 is a leaf of any size, a guarantee for the 64-entry
  traversal stacks that ordinary inputs do not reach.

**One scene is one scene.** These numbers are for the 27 650-triangle bumpy sphere on two
ground triangles, one camera and 1 024 primary rays. Scenes with uneven primitive density,
long thin primitives or secondary rays may gain more or less.

**Unmeasured.** x86-64 beyond the one CI reading above, wasm32, other
scenes, the parameter sweep on any other machine, and tuning the weights per scene.

## Profile of `closest_hit` on the benchmark scene

This section is an investigation (flyingrobots/bunny#212). Nothing got faster, so it is not
a ledger entry. It says where the time of `trace/closest_hit` goes, so that the stage 2
experiments can be chosen by evidence. No library function changed.

### Method

- **Counters live in the harness.** `crates/bunny-perf/src/profile.rs` replicates the search
  loop of `closest_hit` and `any_hit` using only public items (`TraceRay::box_entry`,
  `BvhNode`, `TraceRay::hit_triangle`) and counts events. The core crates carry no counters.
  `bunny-perf --stats` prints the tables below.
- **The replica is checked against the shipped functions.** On every one of the 1 024 rays
  the replica calls the primitive test with the same primitives, the same limits and in the
  same order as `closest_hit` (the same primitives in the same order as `any_hit`), and
  returns the same hit. Swapping the order of the two pushes in the replica makes both tests
  fail.
- **Triangle stages** are redone from the public inputs in `i128` and compared with
  `hit_triangle` on every test: the stage counter never disagreed. A test that passes the
  edge checks with a negative distance numerator returns before dividing, so it is counted
  as `behind_origin`, not as divided (none occur on this scene).
- **Ray classes** come from the hit `closest_hit` finds: sphere (primitive below 27 648),
  ground only, or nothing.
- **Time split.** The replica records every box test (node, limit) and triangle test
  (primitive, limit) of all 1 024 `closest_hit` searches. The `trace/kernel_*` cases replay
  exactly those calls on the real node boxes and triangles, in the order the search made
  them, and report nanoseconds per ray (a batch is the calls of 1 024 rays). The `_chain`
  cases make each call's limit depend on the previous result, which measures latency instead
  of overlapped throughput.

### The tree

| | |
| --- | ---: |
| nodes | 55 277 |
| leaves | 27 639 |
| interior nodes | 27 638 |
| maximum leaf depth (root is 0) | 19 |
| mean leaf depth | 15.44 |
| leaf sizes (primitives: leaves) | 1: 27 628, 2: 11 |
| interior surface area / root surface area, summed | 3.13 |
| primitive-weighted leaf area / root area, summed | 1.86 |

The last two rows are the traversal and intersection terms of the surface-area heuristic
with unit costs (total 4.99 in those units). The root box is dominated by the 40-unit ground
triangles, so they serve as a baseline for comparing builders, not as a prediction of ray
cost. The builder's leaves of up to four primitives are almost never used: 99.96 % of
leaves hold one triangle. The tree is close to the smallest binary tree over these
primitives, and its mean leaf depth (15.4) is near the 14.8 of a perfectly balanced tree
with 27 650 leaves.

### Counts per ray: `closest_hit`

| counter per ray | all rays | hit the sphere | hit only the ground | hit nothing |
| --- | ---: | ---: | ---: | ---: |
| rays | 1 024 | 236 | 448 | 340 |
| nodes popped | 11.37 | 30.78 | 7.58 | 2.89 |
| popped and skipped (entry bound beyond best) | 0.79 | 3.44 | 0.00 | 0.00 |
| interior nodes visited | 9.38 | 24.14 | 6.54 | 2.89 |
| leaves visited | 1.20 | 3.21 | 1.04 | 0.00 |
| `box_entry` calls | 19.76 | 49.27 | 14.08 | 6.78 |
| `box_entry` returned a bound | 11.37 | 30.78 | 7.58 | 2.89 |
| `box_entry` rejected | 8.39 | 18.49 | 6.50 | 3.89 |
| interior visits pushing both children | 2.05 | 7.07 | 0.80 | 0.21 |
| interior visits pushing one child | 6.27 | 15.65 | 4.98 | 1.46 |
| interior visits pushing no child | 1.06 | 1.42 | 0.76 | 1.21 |
| peak stack depth | 2.49 | 6.15 | 1.56 | 1.16 |
| triangle tests | 1.63 | 3.21 | 2.04 | 0.00 |
| failed at the determinant | 0.00 | 0.00 | 0.00 | 0.00 |
| failed the first barycentric check | 0.49 | 1.13 | 0.53 | 0.00 |
| failed the second barycentric check | 0.44 | 0.95 | 0.51 | 0.00 |
| passed the edges, behind the origin (no division) | 0.00 | 0.00 | 0.00 | 0.00 |
| reached the division | 0.70 | 1.13 | 1.00 | 0.00 |
| divided, then beyond the distance limit | 0.02 | 0.10 | 0.00 | 0.00 |
| divided, in range but not better | 0.00 | 0.00 | 0.00 | 0.00 |
| divided, accepted as the new best | 0.67 | 1.03 | 1.00 | 0.00 |

Each class column averages over that class's rays; "all rays" averages over 1 024. Sphere
rays are 23 % of the rays and 57 % of the box tests (236 x 49.27 of 20 238).

### Counts per ray: `any_hit`

Rays are classed by what `closest_hit` finds, so the columns match.

| counter per ray | all rays | hit the sphere | hit only the ground | hit nothing |
| --- | ---: | ---: | ---: | ---: |
| nodes popped | 9.74 | 24.29 | 7.27 | 2.89 |
| interior nodes visited | 8.69 | 21.74 | 6.23 | 2.89 |
| leaves visited | 1.04 | 2.55 | 1.04 | 0.00 |
| `box_entry` calls | 18.39 | 44.48 | 13.45 | 6.78 |
| `box_entry` returned a bound | 10.77 | 28.64 | 7.34 | 2.89 |
| triangle tests | 1.26 | 2.55 | 1.55 | 0.00 |
| reached the division | 0.67 | 1.00 | 1.00 | 0.00 |

`any_hit` costs 91 to 93 % of `closest_hit` because stopping early saves little here. Over
all rays it makes 93 % of the box tests, 86 % of the pops and 77 % of the triangle tests
of `closest_hit`. Rays that hit nothing (33 %) do identical work in both. Ground rays
save 4 % of their box tests and sphere rays 10 % of theirs (21 % of their pops). Read from
the counts, not tested: the nearer-first descent reaches a first hit early, so stopping
there only avoids the 3.4 skipped pops and the nearby subtrees the closest search still
checks, and the descent to the first leaf, which both pay for, dominates.

### Where the time goes

Apple M5 Pro, inside the `flyingrobots/rust-dev:1.96.0` container (`aarch64-linux`),
2026-11-13, two complete runs (run with `--exclusive`, so no other container run
overlapped) of:

```sh
cargo run --locked -p xtask -- perf --base origin/main --rounds 11 --align-code --filter trace/
```

Base revision `e395090e8cd5` (`origin/main`). The harness source is the same on both
sides, so the base and head columns of a run are a noise reference: no case moved by more
than about 2 % in best p50 (the `ray_prepare` p50 moved 6 % on 0.1 ns). Head side, best
p50, nanoseconds per ray:

| Case | Run 1 | Run 2 | What it times |
| --- | ---: | ---: | --- |
| `trace/closest_hit` | 216.23 | 213.50 | the whole search |
| `trace/any_hit` | 197.35 | 198.32 | the whole search |
| `trace/kernel_box_entry` | 128.58 | 126.59 | the 19.76 box tests of a ray, independent calls |
| `trace/kernel_box_entry_chain` | 129.96 | 131.88 | the same, each limit depending on the last result |
| `trace/kernel_tri_rejected` | 15.34 | 15.30 | the 0.93 triangle tests of a ray that do not divide |
| `trace/kernel_tri_divided` | 22.50 | 22.09 | the 0.70 triangle tests of a ray that divide |
| `trace/kernel_tri_all_chain` | 37.11 | 38.05 | all 1.63 triangle tests, chained |

The split of one ray (about 214 to 216 ns), from the two replay styles:

| Part | ns per ray | Share | Per call |
| --- | ---: | ---: | --- |
| box tests (19.76 calls) | 127 to 132 | 59 to 61 % | 6.4 to 6.7 ns |
| triangle tests that fail before dividing (0.93) | 15 | 7 % | about 16 ns |
| triangle tests that divide (0.70) | 22 | 10 % | about 32 ns |
| everything else | 44 to 50 | 20 to 23 % | about 4 ns per pop |

"Everything else" is the stack, the pop and skip decisions, branch mispredictions, the
closure call, bounds checks and any cache misses the replays do not repeat.

A divided test costs about 15 ns more than a rejected one. That difference includes the
products for `qvec` and `t` as well as `shifted_quotient`, so the division is at most about
15 x 0.70 = 10 ns per ray (under 5 %), and less than that.

**How "everything else" was estimated, and how uncertain it is.** It is the whole-search
time minus the replayed kernel times, so it is a residual, not a measurement. Two
effects move it. A replay calls the kernels from a tight loop over inputs known in advance,
so branch prediction and out-of-order overlap are easier than in the real search, which
picks the next node from the previous result; any extra cost of that sits in the residual.
The chained replays bound the overlap effect from the other side: they cost 1 to 5 ns per
ray more than the independent ones, so hidden latency is small for these kernels. The
replays do read the real nodes in the search's order, so they include the cache behaviour
of the 3 MB node array (55 277 nodes of 56 bytes, not checked against this chip's cache
sizes). Treat the "everything else" row as good to plus or minus 15 ns; the box-test row
is the better pinned one. The x86-64 CI runner reads about 1.85 times as long for
`trace/closest_hit` (412 ns, ledger entry 4); this split was not measured there.

### Neighbouring rays

For square blocks of neighbouring camera rays, the box tests the rays make in total
against the distinct nodes they test (`bunny-perf --stats`):

| block | box tests, summed | distinct nodes | shared |
| --- | ---: | ---: | ---: |
| 1 x 1 rays | 20 238 | 20 238 | 0 % |
| 2 x 2 rays | 20 238 | 10 316 | 49 % |
| 4 x 4 rays | 20 238 | 7 346 | 64 % |
| 8 x 8 rays | 20 238 | 6 518 | 68 % |

### Ranked opportunities

"Bound" means the ray's time if that part were free, from the numbers above (about 215 ns
today). A real change gets a fraction of it. Nothing below was tried.

1. **Fewer box tests per ray: #213 (builder), #216 (4-wide), #217 (packets).** Box tests are
   127 to 132 ns (60 %); free, the ray would cost about 87 ns. 42 % of them (8.39 of 19.76)
   reject. Sphere rays make 57 % of all box tests and 24.1 interior visits each where one
   descent to a leaf takes about 15, so about 9 visits (18 box tests, about 115 ns) are
   excess for them; averaged over all rays that is about 26 ns of box tests plus about
   8 ns of pops. **#213 can therefore pay at most about 30 to 35 ns (15 %)** on this scene,
   and only if the builder makes sphere rays descend almost one path. It cannot shrink the
   tree: it already has one triangle per leaf and near-minimal depth. It can only reduce
   box overlap.
2. **A cheaper box test: #215 (compact node layout).** Bound: the box row, 127 to 132 ns.
   A box test costs about 6.5 ns, and the chained replay is no slower than 5 ns per ray
   above the independent one, so the test is limited by its own arithmetic (`i128`
   products) more than by waiting for loads. A compact node cuts bytes per test, which
   helps only the load share of that time; the replays do not separate load time from
   arithmetic time, so that share is unmeasured. With integer-only arithmetic and no SIMD
   in the core crates, expect a layout change alone to pay a small part of the box row.
3. **A 4-wide BVH: #216.** Bound: the 44 to 50 ns residual (20 to 23 %), not the box row.
   A 4-wide node tests four boxes per visit and halves the depth, so the box tests along a
   path stay about the same (4 per wide level against 2 per binary level, over half the
   levels) and the 20 238 box tests per 1 024 rays do not fall. What falls is pops, pushes
   and steps between nodes, by up to about half of the residual, minus the cost of
   ordering four children. Without SIMD a wide node does not make one box test cheaper.
   Expect at most 15 to 25 ns.
4. **Ray packets: #217.** Neighbouring rays do test the same nodes: 49 % of box tests in a
   2 x 2 block, 64 % in 4 x 4, 68 % in 8 x 8 are repeats. If one test per node served a
   whole 4 x 4 block, the box row could fall by up to 64 %, to about 47 ns, and the ray
   would cost about 134 ns. That is a ceiling: each ray still needs its own entry bound
   to order its children, so only a conservative whole-packet test saves anything, and it
   saves less as rays diverge. These are primary camera rays on one smooth surface; secondary
   rays share far less. #217 has the largest ceiling of the five and the least certain
   payoff. **Tried (entry 7, "Tried And Rejected"): the ceiling was not
   reachable.** A whole-packet test decided only 19 % of its tests at 4 x 4 and cost about
   1.6 box tests each, so packets with culling were 14 to 43 % slower on this Mac, and the
   shared descent alone was within 4 % of single rays (faster only in 8 x 8 tiles).
5. **Rejecting far triangle hits before dividing: #214 cannot pay off on this scene.** Only
   0.02 of the 0.70 divisions per ray are rejected by the distance limit afterwards, worth
   about 0.3 ns. Accepted divisions are 1.03 per sphere ray and 1.00 per ground ray, one
   per hit plus 0.03, so there is no division left to avoid. All triangle tests together are
   37 to 38 ns (17 %): free, the ray would cost about 178 ns, and the division is at most
   10 ns of that. A cheaper rejecting test (15 ns per ray) is a different idea from #214.

No single item reaches the stage 2 target of under 100 ns per ray: free box tests give about
87 ns, and nothing else comes close. The target needs both fewer box tests (builder,
packets) and a lower cost per box test or per visit (layout, 4-wide).

### Unmeasured

x86-64 and wasm32; other scenes and ray batches (secondary rays share less between
neighbours); the split of a box test into arithmetic and loads; whether a packet or a
4-wide node would reach its ceiling; the effect of another builder on the sphere rays'
excess visits (the 30 to 35 ns is an arithmetic ceiling, not a trial).

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

## Tried And Rejected

Experiments that kept exact results but did not pay off. They are recorded so the same idea
is not retried without new evidence (a different target, or a changed design).

### Division-free `normalize`

**Idea.** Replace `normalize`'s three divisions by the length with multiplications: a
reciprocal of the length from a 256-entry seed table and three Newton steps
(`y <- y(2 - xy)`, within 0–2 units of `2^127 / x` on 1 000 000 samples), a multiplied
quotient estimate, and an exact remainder correction. No component is longer than the
length, so the quotient is at most `2^32` and one correction step suffices.

**Fidelity.** Exact: equivalence tests against `checked_div` and against the previous
`normalize` (100 000 `FixedVec3` and `FixedVec2` cases each) passed.

**Result on an Apple M5 Pro** (`--base origin/perf/fixed-divisor --rounds 11
--align-code --filter linalg/`; measured with the harness before it passed every result
through `black_box`, and not re-measured since, so read the sizes as indicative):

| Variant | Best p50 before → after | Median before → after |
| --- | --- | --- |
| Branching correction | 42.72 → 41.46 ns (−2.9 %) | 45.49 → 55.66 ns (+22.4 %) |
| Branch-free correction (masks) | 42.60 → 58.39 ns (+37.1 %) | 44.47 → 61.85 ns (+39.1 %) |

**Why it lost.** The M5's 64-bit divide instruction is fast, so compiler-rt's 128-bit
division takes few cycles for these operand sizes. The replacement's dependent chain
(widening multiply, variable 128-bit shift, second multiply, subtract, correct, round) is
about as long, and the branch-free form only lengthens it. Targets without a fast 64-bit
divide (wasm32 in particular) were not measured and may behave differently.

**Where it is.** Branch `experiment/divide-free-normalize` (`UnitDivisor` and its tests),
not proposed for merging.

### 6. Reject triangle hits beyond the limit before dividing

**Where tried:** `TraceRay::hit_triangle` in `bunny-query` (flyingrobots/bunny#214). The
kernel on `main` is unchanged; the branch added the check, measured it and withdrew it.

**Idea.** After the exact crossing test, `hit_triangle` divides once to get
`distance = floor(t * 2^32 / det)` (with `det > 0`) and only then compares it with the limits.
`closest_hit` passes the best distance so far as `t_max`, so a triangle crossed behind the
best hit pays for a division whose result is thrown away. The check tried was, before the
division:

```text
reject when  t >= (t_max + 1) * ceil(det / 2^32)        (checked i128 multiply)
```

**Why it is exact.** With `c = ceil(det / 2^32)`, `det <= c * 2^32`. For `t_max >= 0`,
`t >= (t_max + 1) * c` gives `t * 2^32 >= (t_max + 1) * c * 2^32 >= (t_max + 1) * det`, so
the floored distance is at least `t_max + 1`: the old code returns `None` too (or the quotient
does not fit `i64`, which is also `None`). For `t_max < 0`, every crossing is a miss either way:
a negative `t` is behind the origin, and a non-negative `t` gives a distance of at least zero.
A product that overflows `i128` (`t_max + 1` up to `2^63` times `c` up to about `2^85`)
exceeds every `t`, so it proves nothing and the old division runs. The check can only say
"certainly beyond"; otherwise the code is the old code. It is conservative only where
`det` is not a multiple of `2^32` and the distance is within `(t_max + 1) / c` of the limit,
which for the scene's triangles (`det` around `2^86`) means essentially never. A symmetric
check for `t_min` was not tried: the traversal always passes `t_min = 0`, and a negative `t`
is already rejected by a sign test before any division (`u128::try_from` in
`shifted_quotient`), so neither an early sign test nor a lower-limit check removes work here.

**Fidelity: it was an exact replacement.** `crates/bunny-query/tests/trace_equivalence_tests.rs`
compares `hit_triangle` with a frozen copy of its arithmetic
(`crates/bunny-query/tests/support/trace_reference.rs`): distance, facing and both
barycentric weights, at upper and lower limits equal to the true distance, one and two raw
units below it, one above, zero, -1, 1, `i64::MIN`, `i64::MAX` and `i64::MAX - 1`, over
30 000 seeded rays and triangles (each at those limits as an upper limit, a lower limit and an
upper limit with `t_min = i64::MIN`: up to 900 000 comparisons), constructed
hits whose `det` is a multiple of `2^32`, `2^32 + 1` (`641 * 6 700 417`) or 1, and the
corners of the trace domain. `crates/bunny-perf/src/scene.rs` compares every triangle test
the scene's `closest_hit` makes, at the limit it passed and around the true distance. The
existing traversal tests passed unmodified. Four deliberate mutations of the check were each
caught:

| Mutation | Caught by |
| --- | --- |
| `t_max` instead of `t_max + 1` (one raw unit too eager) | TRACE-TP-003, TRACE-TP-201 to TRACE-TP-205, TRACE-TP-102, TRACE-TP-103, TRACE-TP-106, TRACE-TP-108 |
| `floor(det / 2^32)` instead of the ceiling | TRACE-TP-201, TRACE-TP-204 |
| `t + 1 >= bound` instead of `t >= bound` | TRACE-TP-204 |
| `ceil` computed with `2^32 - 2` instead of `2^32 - 1` | TRACE-TP-204 |

The last two survived the first corpus; `TRACE-TP-204` (raw-unit triangles whose `det` is
exactly `2^32`, `2^32 + 1` or 1) was added to catch them.

**How often it could fire.** `count_the_divisions_the_scene_traversal_makes` in
`crates/bunny-perf/src/scene.rs` replays `trace/closest_hit` on its 1 024 camera rays and
classifies every triangle test with the reference arithmetic:

| Triangle tests | Reach the division (`t >= 0` crossing) | Divided but beyond the limit | Rejected by the check |
| ---: | ---: | ---: | ---: |
| 1 673 (1.63 per ray) | 714 (42.7 %) | 24 (3.4 % of divisions, 1.4 % of tests) | 24 (all of them) |

The traversal already prunes almost everything behind the best hit at the box level, so
only 24 divisions in 1 024 rays are wasted. This agrees with the
[profile](#profile-of-closest_hit-on-the-benchmark-scene) (0.02 of 0.70 divisions per ray
beyond the limit), which predicted that #214 could not pay on this scene. Even a check that cost nothing would save about
24 × 12.5 ns over 1 024 rays, roughly 0.3 ns per ray, or 0.13 % of `trace/closest_hit`.

**Measured.** Apple M5 Pro, inside the `flyingrobots/rust-dev:1.96.0` container
(`aarch64-linux`), 2026-11-13, two complete runs through the guarded container runner with
`--exclusive` (no other container run active) and the branch's commits as history:

```sh
cargo run --locked -p xtask -- perf --base origin/main --rounds 11 --align-code
```

Base revision `42279ed2825f` (`origin/main`); the working tree was the branch merged with
that base, with the check in place. The two `trace/triangle_*` cases are new in the
harness and run the same source on both sides: `trace/triangle_hit` tests each camera ray
against the triangle it hits first with no upper limit (the hit divides), and
`trace/triangle_beyond_limit` tests the same pairs with `t_max` one raw unit below the hit.
Nanoseconds per call:

| Case | Run 1 p50 before | Run 1 p50 after | Run 1 best change | Run 2 p50 before | Run 2 p50 after | Run 2 best change |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `trace/closest_hit` | 225.02 | 222.94 | -1.5% | 223.75 | 223.02 | -2.0% |
| `trace/any_hit` | 208.42 | 209.72 | +0.5% | 209.47 | 209.96 | +2.1% |
| `trace/triangle_beyond_limit` | 30.64 | 18.07 | -39.8% | 30.56 | 17.99 | -39.9% |
| `trace/triangle_hit` | 30.15 | 32.71 | +7.1% | 29.95 | 32.59 | +7.2% |
| `trace/ray_prepare` | 2.03 | 1.99 | +0.0% | 1.99 | 1.99 | +0.0% |
| `scene/closest_visitor` | 3179.13 | 3217.74 | +0.3% | 3226.89 | 3203.49 | -0.4% |
| `query/ray_triangle` | 52.61 | 52.73 | +0.2% | 52.86 | 52.69 | -0.5% |
| `query/ray_aabb` | 54.12 | 54.57 | +2.6% | 54.97 | 54.32 | -0.8% |
| `query/ray_build` | 51.55 | 51.84 | -0.2% | 51.35 | 51.72 | -0.5% |
| `query/ray_sphere_hit` | 67.26 | 70.56 | -1.6% | 67.59 | 70.68 | +0.6% |
| `query/ray_sphere_miss` | 21.20 | 21.20 | -0.2% | 21.12 | 21.16 | -0.2% |
| `linalg/length` | 16.32 | 16.03 | -0.8% | 16.24 | 16.36 | +0.3% |
| `linalg/normalize` | 41.83 | 42.48 | -1.3% | 41.95 | 42.11 | +0.3% |
| `num/div` | 6.71 | 6.71 | +0.0% | 6.67 | 6.67 | -1.2% |
| `num/mul` | 1.42 | 1.42 | +0.0% | 1.42 | 1.42 | +0.0% |
| `num/sqrt` | 13.59 | 13.88 | +3.2% | 13.92 | 14.04 | -0.3% |
| `angle/sin_cos` | 11.11 | 11.11 | +0.4% | 11.11 | 11.11 | +0.0% |
| `angle/from_radians` | 0.94 | 0.94 | +0.0% | 0.94 | 0.94 | +0.0% |
| `angle/f64_sin_cos_reference` | 4.19 | 4.19 | +0.0% | 4.19 | 4.15 | +0.0% |

**Reading.** In isolation the check does what it should: a rejected hit costs about 12.6 ns
less (-40 %, both runs), and every hit that still divides costs about 2.1 ns more (+7 %, both
runs), the price of the 128-bit checked multiply and compare. On the scene, 690 crossings
pay that price for every 24 that skip the division, so the per-ray cost model predicts a
loss of about 1.2 ns (+0.5 %), and the check breaks even only when about one crossing in
seven is behind the limit. The measured `trace/closest_hit` moved by -1.5 % and -2.0 % in best
p50 and -0.9 % and -0.3 % in median p50: inside the noise band, smaller than the controls'
spread (`num/sqrt` +3.2 %, `query/ray_aabb` +2.6 %, `trace/any_hit` +2.1 % on unchanged or
non-firing paths), and the opposite of what the counts predict, so it is not claimed as a
gain. `trace/any_hit` passes an unbounded limit, so the check never fires there and only
adds cost.

**Decision: not shipped.** It adds a kernel branch whose benefit cannot be seen on the scene
this project measures. The equivalence tests, the scene comparison, the counting test and
the two micro cases stay: they are the oracle and the measurement any later attempt needs.

**What would change it:** a scene where many crossings are behind the nearest hit at the
leaf level (dense overlapping geometry, large leaves, or a tree whose leaf boxes overlap a
lot; flyingrobots/bunny#213 may change the tree), or a cheaper certainty test. The counting
test gives the fraction directly; above about 15 % of crossings it is worth re-measuring.
x86-64 (with a hardware 128-by-64 divide the saving per skipped division is smaller) and
wasm32 (where both the division and the 128-bit multiply are software routines) were not
measured; the CI job measured x86-64 only on the withdrawn state.

### 7. Ray packets for coherent camera rays

**Where tried:** a new `closest_hits` beside `closest_hit` in `bunny-broadphase`
(flyingrobots/bunny#217), with harness cases and counters in `bunny-perf`. The code is in
commit `7c65c5e` of branch `perf/ray-packets`; it was measured there and removed before the
branch's last commit, so `main` gains no packet API.

**What was built.** `closest_hits(bvh, &RayPacket::new(&rays, t_max, tests), hit)` traced a
fixed-size array of up to 64 rays together and returned one `Option<ClosestHit>` per ray;
the primitive closure took the ray's index. One stack holds `(node, mask of rays that may
enter it, smallest entry bound among them)`. At a pop, the rays whose limit is below that
bound are dropped from the mask. At an interior node each child is tested for the rays still
in the mask; children are pushed nearer smallest bound first, ties to the left, one order for
the whole packet. At a leaf every ray in the mask tests every primitive with its own limit
and keeps hits by `closest_hit`'s rule (smaller distance, then lower index). Three variants:

| Variant | Test at each child |
| --- | --- |
| `per_ray` | `box_entry` for every ray in the mask (a shared descent, no new geometry) |
| `cull` | one whole-packet test first; a node it rejects is not tested ray by ray |
| `accept` | as `cull`, and a node that every ray certainly enters is not tested ray by ray either; its bound for every ray is the packet's smallest entry |

**The whole-packet test, and why it is exact.** It applies only when every ray has the same
origin (the camera's rays do). On one axis, `box_entry` computes the slab lower bound
`scaled(near, r)` and upper bound `scaled(far, r + 1) + 1`, where `near` and `far` depend only
on the origin, the box and the sign of the direction component, `r = floor(2^63 / |d|)` is
the ray's reciprocal and `scaled(v, r) = min(floor(v * r / 2^31), u64::MAX)`. For a fixed
`v >= 0`, `scaled` is non-decreasing in `r`, so if every ray's component on the axis has one
sign, the packet's smallest and largest reciprocal `r_lo`, `r_hi` bound every ray's slab
bounds on that axis: `scaled(near, r_lo) <= lower_i <= scaled(near, r_hi)` and
`scaled(far, r_lo + 1) + 1 <= upper_i <= scaled(far, r_hi + 1) + 1` (saturation is monotone
too). An axis whose components are all zero gives every ray the same answer (inside the slab
or not). An axis whose components change sign contributes only the trivial bounds
`0 <= lower_i` and `upper_i <= u64::MAX`. With `E_lo`, `E_hi` the largest over axes of the low
and high lower bounds and `X_lo`, `X_hi` the smallest over axes of the low and high upper
bounds:

- **Reject** when a common-sign axis has `far < 0`, an all-zero axis has the origin outside
  the slab, `E_lo > X_hi`, or `E_lo` exceeds the largest limit in the mask (or that limit is
  negative). Then for every ray `enter_i >= E_lo > X_hi >= exit_i` or
  `enter_i >= E_lo > limit_max >= limit_i`, so `box_entry` returns `None` for every ray: the
  test never culls a node that `box_entry` would accept for some ray at its own limit.
- **Accept** when no axis changes sign, `E_hi <= X_lo` and `E_hi` is at most the smallest
  limit in the mask. Then for every ray `enter_i <= E_hi <= X_lo <= exit_i` and
  `enter_i <= limit_i`, so `box_entry` returns `Some(enter_i)` with `enter_i >= E_lo`, and
  `E_lo` is a valid lower bound to push for every ray.

This argument is about `box_entry`'s computed bounds, floors and per-axis reciprocal
rounding included, not about exact slab distances; it needs `r` to be exactly the reciprocal
`TraceRay::new` stores, which the packet recomputed from `TraceRay::direction`, so a change
to that formula would have to change both. A seeded test (20 000 packets of 1 to 64 rays from
one origin, spreads from zero to half a unit, zero, tiny and sign-changing components, seeded
boxes and per-ray limits including negative ones) checked every verdict against `box_entry`
ray by ray: 2 000 or more rejects, 1 000 or more accepts and 1 000 or more undecided cases,
no contradiction.

**Why the answers equal `closest_hit`'s.** The correctness argument of the
[ray-tracing chapter](../ray-tracing/README.md#correctness-the-answer-does-not-depend-on-the-tree)
does not depend on the visiting order. Per ray: a node is removed from a ray's mask only
when that ray's own `box_entry` rejects it (the per-ray test, or a whole-packet reject that
implies it), or at a pop when the pushed bound, at most that ray's own bound, exceeds the
ray's own limit. So every ancestor of the ray's winner stays in its mask (steps 1 to 4 of the
argument, with the ray's own limit), the winner is tested, and the tie rule keeps it whatever
order the hits arrive in. Visiting order only changes how soon each ray's limit shrinks, that
is, the amount of work.

**Fidelity: a new API that returned exactly `closest_hit`'s hit for every ray.** Evidence at
commit `7c65c5e`: packets of every size from 1 to 64 over about 440 seeded triangles (a
tenth duplicated, so ties occur), fanned from one origin with three spreads and scattered
with their own origins and zero or tiny components, matched `closest_hit` (primitive and
distance bits) for every ray and every variant, over 5 000 hits; `closest_hit` in turn
matched a test of every triangle on the packet rays; nine copies of one ray, copies limited
one raw unit short of their hit, packets that all miss, limits 0, -1 and `i64::MIN`, two
origins in one packet, and an empty tree all agreed; and the harness checked every packing of
the 1 024 benchmark rays against `closest_hit` before timing. A replica of the search in the
harness made the same primitive calls with the same limits in the same order as
`closest_hits`. Deliberate mutations, each run against the tests:

| Mutation of the whole-packet test | Result |
| --- | --- |
| Smallest entry from the largest reciprocal (`scaled(near, r_hi)` as `E_lo`) | caught (seeded verdict test) |
| Largest exit from the smallest reciprocal (`upper(r_lo)` as `X_hi`) | caught (seeded verdict test) |
| Reject against the smallest limit in the mask instead of the largest | caught (seeded verdict test) |
| A sign-changing axis treated as unbounded and exact | caught (seeded verdict test and the hand-built accept case) |
| Accept from the smallest entry instead of the largest (`E_lo <= X_lo`) | caught (seeded verdict test) |
| Accepted bound not a lower bound (`i64::MAX`) | caught (seeded verdict test and the hand-built accept case) |
| The "no sign-changing axis" condition dropped from accept | survives, by construction: such an axis already sets `E_hi = u64::MAX`, so the condition is redundant |
| Pop skip made strict (`bound < limit`) | survives: it changes an answer only for a tie exactly on a subtree's bound (as `TRACE-TP-108` shows for `closest_hit`); no packet case was constructed for it |
| Children always left first | survives, as expected: order changes work, not answers |

**Measured.** Apple M5 Pro, inside the `flyingrobots/rust-dev:1.96.0` container
(`aarch64-linux`), 2026-11-14, two complete runs with `--exclusive` (no other container run
overlapped) of:

```sh
cargo run --locked -p xtask -- perf --base origin/main --rounds 11 --align-code
```

Base revision `57bc035f69f2` (`origin/main`), which has no packet API, so the packet cases
are reported as new and compare with `trace/closest_hit` of the same run and binary. Every
case traces all 1 024 camera rays, so the times are per ray. Coherent packets are square
tiles of the 32 x 32 camera grid; shuffled packets are consecutive runs of a seeded
permutation of the rays, compared with `packet/single_shuffled` (`closest_hit` one ray at a
time in that shuffled order). Best p50, nanoseconds per ray, change against the single-ray
reference of the same run:

| Case | Run 1 | Change | Run 2 | Change |
| --- | ---: | ---: | ---: | ---: |
| `trace/closest_hit` (reference, camera order) | 222.53 | | 222.37 | |
| `packet/per_ray_2x2` | 245.57 | +10.4 % | 246.50 | +10.9 % |
| `packet/per_ray_4x4` | 225.10 | +1.2 % | 225.22 | +1.3 % |
| `packet/per_ray_8x8` | 213.42 | -4.1 % | 212.93 | -4.2 % |
| `packet/cull_2x2` | 316.94 | +42.4 % | 317.30 | +42.7 % |
| `packet/cull_4x4` | 276.45 | +24.2 % | 275.80 | +24.0 % |
| `packet/cull_8x8` | 262.04 | +17.8 % | 262.94 | +18.2 % |
| `packet/accept_2x2` | 277.47 | +24.7 % | 276.82 | +24.5 % |
| `packet/accept_4x4` | 257.45 | +15.7 % | 256.92 | +15.5 % |
| `packet/accept_8x8` | 253.13 | +13.8 % | 253.95 | +14.2 % |
| `packet/single_shuffled` (reference, shuffled order) | 227.38 | | 227.78 | |
| `packet/per_ray_shuffled_4` | 313.96 | +38.1 % | 312.09 | +37.0 % |
| `packet/per_ray_shuffled_16` | 319.78 | +40.6 % | 319.30 | +40.2 % |
| `packet/per_ray_shuffled_64` | 295.90 | +30.1 % | 296.22 | +30.0 % |
| `packet/cull_shuffled_4` | 421.75 | +85.5 % | 421.71 | +85.1 % |
| `packet/cull_shuffled_16` | 405.64 | +78.4 % | 406.37 | +78.4 % |
| `packet/cull_shuffled_64` | 367.88 | +61.8 % | 366.66 | +61.0 % |
| `packet/accept_shuffled_4` | 419.03 | +84.3 % | 420.29 | +84.5 % |
| `packet/accept_shuffled_16` | 404.13 | +77.7 % | 404.05 | +77.4 % |
| `packet/accept_shuffled_64` | 367.88 | +61.8 % | 366.17 | +60.8 % |

The existing cases moved between base and head by at most 1.4 % in best p50 except
`num/mul` (-5.6 % on 1.4 ns) and `trace/kernel_tri_divided` (+5.7 %, run 1), on unchanged
code. `trace/closest_hit` read 219.4 to 219.6 ns on the base side and 222.4 to 222.5 ns on the
head side, so the -4 % of `per_ray_8x8` is about the size of the between-binary spread.

The x86-64 CI runner (`Perf / before and after` job of the pull request, shared GitHub
runner, 7 rounds, `--align-code`; coarse) read, best p50 in nanoseconds per ray:

| Case | CI run 1 | Change | CI run 2 | Change |
| --- | ---: | ---: | ---: | ---: |
| `trace/closest_hit` (reference, camera order) | 633.74 | | 375.61 | |
| `packet/per_ray_2x2` | 604.48 | -4.6 % | 347.03 | -7.6 % |
| `packet/per_ray_4x4` | 576.18 | -9.1 % | 325.72 | -13.3 % |
| `packet/per_ray_8x8` | 545.16 | -14.0 % | 299.32 | -20.3 % |
| `packet/cull_2x2` | 829.25 | +30.9 % | 474.69 | +26.4 % |
| `packet/cull_4x4` | 737.42 | +16.4 % | 423.13 | +12.7 % |
| `packet/cull_8x8` | 712.43 | +12.4 % | 391.54 | +4.2 % |
| `packet/accept_2x2` | 733.73 | +15.8 % | 417.04 | +11.0 % |
| `packet/accept_4x4` | 687.46 | +8.5 % | 388.62 | +3.5 % |
| `packet/accept_8x8` | 687.60 | +8.5 % | 377.81 | +0.6 % |
| `packet/single_shuffled` (reference, shuffled order) | 653.20 | | 386.57 | |
| `packet/per_ray_shuffled_4` | 758.88 | +16.2 % | 440.80 | +14.0 % |
| `packet/per_ray_shuffled_16` | 822.67 | +25.9 % | 465.19 | +20.3 % |
| `packet/per_ray_shuffled_64` | 700.32 | +7.2 % | 418.33 | +8.2 % |
| `packet/cull_shuffled_4` | 1141.32 | +74.7 % | 624.69 | +61.6 % |
| `packet/cull_shuffled_16` | 1111.13 | +70.1 % | 602.76 | +55.9 % |
| `packet/cull_shuffled_64` | 966.89 | +48.0 % | 531.30 | +37.4 % |
| `packet/accept_shuffled_4` | 1140.05 | +74.5 % | 623.88 | +61.4 % |
| `packet/accept_shuffled_16` | 1106.86 | +69.5 % | 602.09 | +55.8 % |
| `packet/accept_shuffled_64` | 966.25 | +47.9 % | 529.75 | +37.0 % |

The two CI runs ran on different runner machines (absolute times differ by 1.7 times) and
compared against different `main` revisions (`57bc035f69f2` and `cda15ecc020a`, the pull
request merged into the newer `main`); only the ratios within one run mean anything.

**Counts.** From a replica of the packet search in the harness (as in the profile,
the library carried no counters), per ray, summed over all packets of a packing and divided
by 1 024. "Packet node visits" counts each node the packet visits once; "ray-node visits"
counts it once per ray in the mask. For comparison, `closest_hit` makes 19.76 `box_entry`
calls, 11.37 pops and 1.63 triangle tests per ray.

| Packing, variant | Per-ray box tests | Packet tests | Packet rejects | Packet accepts | Packet node visits | Ray-node visits | Pops | Triangle tests |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 2 x 2, per_ray | 19.91 | 0.00 | 0.00 | 0.00 | 5.80 | 10.65 | 6.10 | 1.63 |
| 4 x 4, per_ray | 20.12 | 0.00 | 0.00 | 0.00 | 4.41 | 10.78 | 4.59 | 1.66 |
| 8 x 8, per_ray | 20.24 | 0.00 | 0.00 | 0.00 | 4.03 | 10.85 | 4.17 | 1.67 |
| shuffled 4, per_ray | 21.66 | 0.00 | 0.00 | 0.00 | 9.36 | 11.72 | 9.94 | 1.92 |
| shuffled 16, per_ray | 23.59 | 0.00 | 0.00 | 0.00 | 8.43 | 12.88 | 8.88 | 2.21 |
| shuffled 64, per_ray | 23.64 | 0.00 | 0.00 | 0.00 | 7.00 | 12.92 | 7.31 | 2.22 |
| 2 x 2, cull | 13.66 | 10.10 | 3.09 | 0.00 | 5.80 | 10.65 | 6.10 | 1.63 |
| 4 x 4, cull | 15.94 | 7.28 | 1.23 | 0.00 | 4.41 | 10.78 | 4.59 | 1.66 |
| 8 x 8, cull | 17.89 | 6.49 | 0.60 | 0.00 | 4.03 | 10.85 | 4.17 | 1.67 |
| shuffled 4, cull | 21.00 | 16.82 | 0.58 | 0.00 | 9.36 | 11.72 | 9.94 | 1.92 |
| shuffled 16, cull | 23.32 | 14.87 | 0.27 | 0.00 | 8.43 | 12.88 | 8.88 | 2.21 |
| shuffled 64, cull | 23.42 | 12.05 | 0.22 | 0.00 | 7.00 | 12.92 | 7.31 | 2.22 |
| 2 x 2, accept | 8.32 | 10.10 | 3.09 | 1.39 | 5.80 | 10.65 | 6.10 | 1.63 |
| 4 x 4, accept | 13.12 | 7.28 | 1.23 | 0.18 | 4.41 | 10.78 | 4.59 | 1.66 |
| 8 x 8, accept | 16.57 | 6.49 | 0.60 | 0.02 | 4.03 | 10.85 | 4.17 | 1.67 |
| shuffled 4, accept | 20.97 | 16.82 | 0.58 | 0.01 | 9.36 | 11.72 | 9.94 | 1.92 |
| shuffled 16, accept | 23.32 | 14.87 | 0.27 | 0.00 | 8.43 | 12.88 | 8.88 | 2.21 |
| shuffled 64, accept | 23.42 | 12.05 | 0.22 | 0.00 | 7.00 | 12.92 | 7.31 | 2.22 |

**Why the profile's ceiling was not reachable.** The profile estimated about 134 ns per ray
"at best" for 4 x 4 packets, from the box tests a block makes (20 238) against the distinct
nodes it tests (7 346): if one test per distinct node served the block, at the cost of one
box test, the box row would fall by 64 %. Both assumptions fail:

- *A packet test does not replace the per-ray tests at most nodes.* The shared descent does
  visit few nodes (4.41 packet visits per ray at 4 x 4 against 11.37 pops for `closest_hit`),
  but every ray in the mask still needs its own answer at every child: whether it enters
  (which sets its mask below) and its own entry bound (which prunes it at the pop). A
  whole-packet test can skip those only where it decides for all rays at once. At 4 x 4 it
  decided 1.41 of its 7.28 tests per ray (19 %: 1.23 rejects, 0.18 accepts); at 2 x 2, 4.48
  of 10.10 (44 %). The interval bound over the packet's reciprocals widens with the packet,
  so larger packets decide less, and a packet whose rays have not all hit yet carries the
  largest limit (often unlimited), which stops limit-based rejects.
- *A packet test costs more than a box test.* It computes four scaled bounds per axis instead
  of two, and the smallest and largest limit over the mask. From the counts and the timings,
  each packet test costs about 10.6 ns (at 2 x 2: 11.6 per-ray box tests saved, about 75 ns,
  against 10.10 packet tests and +32 ns measured; at 4 x 4: 7.0 saved, about 45 ns, against
  7.28 tests and +32 ns), about 1.6 box tests. To pay, a packet test must remove more than
  1.6 per-ray tests on average; it removed 1.15 at 2 x 2 and 0.96 at 4 x 4.
- *The shared descent alone is roughly neutral on this Mac.* It removes pops and stack
  traffic (6.10, 4.59 and 4.17 packet pops per ray), but the per-ray mask loops at every
  pop and child cost about as much, and the per-ray box tests rise slightly (19.91 to
  20.24 against 19.76) because one visiting order for the packet is not every ray's own
  front-to-back order, so rays find their hits later and prune less. Net: +10 % at 2 x 2,
  +1 % at 4 x 4, -4 % at 8 x 8.
- *Incoherent packets are much worse*, 30 to 41 % slower with per-ray tests and 61 to 86 %
  with packet tests, because the rays share few nodes (ray-node visits rise only from about
  10.7 to 12.9, but packet visits per ray do not fall much) and the mask overheads stay.

**Decision: not kept.** No variant beats single-ray tracing on this Mac by a margin outside
noise: the best, the shared descent in 8 x 8 tiles, is 4 % faster in both runs, about the
spread between binaries, and the culling variants are 14 to 43 % slower on coherent rays.
The packet code is removed from the library and the harness (it needs the API); the record
and commit `7c65c5e` keep it reproducible.

**What would change the verdict.**

- *x86-64.* The CI runner's readings above favour the shared descent more than the Mac does.
  In both CI runs the shared descent was faster than `closest_hit`, by 5 and 8 % at 2 x 2,
  9 and 13 % at 4 x 4 and 14 and 20 % at 8 x 8, and the culling variants were slower or level.
  Two 7-round readings on shared runners are coarse, and this experiment's target is the
  Mac, so the shared descent is not shipped for x86-64 here. A dedicated measurement on
  x86-64 (the platform job with more rounds, or local x86 hardware) is what would justify a
  target-specific choice under [the rules above](#target-specific-implementations): one
  `closest_hits` whose x86-64 build runs the shared descent and whose other builds trace each
  ray with `closest_hit`, both tested on every target, both returning the same bits.
- *SIMD in a quarantined backend crate.* The per-ray mask loop is the loop that vector lanes
  do for free; with 4 or 8 lanes of 64-bit products the per-ray box tests of a packet would
  cost about one test's latency. Core crates cannot do this (no SIMD intrinsics).
- *A cheaper or tighter whole-packet test*: the limit range kept incrementally instead of
  recomputed per node, or a frustum of planes through the corner rays (needs its own proof
  for `box_entry`'s rounding).
- *A wider tree* (flyingrobots/bunny#216): fewer, larger nodes would give a packet test more
  rays to decide per node.
- *A different scene*: large coherent regions (walls, floors) where whole packets enter the
  same boxes would give more accepts. Secondary rays (reflections, shadows from area lights)
  are less coherent than these camera rays, so they would gain less.
