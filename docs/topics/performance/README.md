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
  `linalg`, `query`, `scene`, `trace`, `sample`, …). When the base revision cannot compile a group (an API it does not
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
`trace/closest_hit` must choose the same primitive on every ray. The harness runs that
check before timing and exits with the differing ray indices if it fails.

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

`crates/bunny-perf/src/memory.rs` adds replays that separate the loads of a box test from its
arithmetic (`trace/kernel_box_load`, `trace/kernel_box_load_chain`,
`trace/kernel_box_entry_addr_chain`), a two-box replica of the box test
(`trace/kernel_box_single`, `trace/kernel_box_pair`) and `closest_hit` on the tree with its
sibling pairs shuffled (`trace/closest_hit_shuffled`); see
[Memory share of a box test](#memory-share-of-a-box-test). The `trace_compact` group
(`crates/bunny-perf/src/compact.rs`) times the compact layout on the same rays:
`trace/closest_hit_compact`, `trace/any_hit_compact`, one conversion of the tree per call
(`trace/compact_convert`), and the compact nodes in three other orders
(`_builder_order`, `_near_first`, `_shuffled`). Before timing, the harness checks that every
compact tree makes the same primitive calls and gives the same answers as the builder's tree on
every ray, and exits with the differing ray indices if not.

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

### 7. Compact node layout with both child boxes in the parent (new API)

**Where:** `compact_bvh`, `compact_closest_hit`, `compact_any_hit` and their types in
`bunny-broadphase` (`src/compact.rs`, `src/compact_hit.rs`), flyingrobots/bunny#215.
`BvhNode`, `build_bvh`, `closest_hit`, `any_hit`, `intersect_ray` and `intersect_aabb` are
unchanged. The layout and its contract are in the
[ray-tracing chapter](../ray-tracing/README.md#compact-layout).

**Technique.** A second, read-only representation of a built tree. Each interior node holds
both children's boxes and references (112 bytes); a leaf's primitive range lives in its
parent, so leaves have no node; nodes are in depth-first, left-first order. In the
builder's layout a search step pops an index, loads that node to learn where its children
are, then loads the children's boxes: two dependent loads. In the compact layout the stack
entry already holds the child's reference, so a step loads one node and tests both boxes,
and a leaf visit loads nothing. The [memory estimate](#memory-share-of-a-box-test) shows why
that matters more than the bytes: loads whose addresses are known early cost 0.6 ns per box
test, loads that wait on the previous one about 7 ns.

**Fidelity: new API with an exact contract.** The compact searches make the same box tests,
in the same order, with the same pruning and tie rules, as `closest_hit` and `any_hit` on
the source tree, so they call the primitive test with the same primitives and limits in the
same order and return the same `ClosestHit` bits. Evidence: TRACE-TP-301 to TRACE-TP-307
(seeded scenes against the source searches and a test of every primitive, ties, limits,
other tree shapes, empty and one-leaf trees, typed errors, golden conversion vectors) with
eight deliberate mutations caught, and PERF-TP-045 (all 1 024 benchmark rays, every primitive
call, on the compact tree and three reordered copies). The harness repeats that check before
every timing run.

**Measured.** Apple M5 Pro, inside the `flyingrobots/rust-dev:1.96.0` container
(`aarch64-linux`), each run with `--exclusive`:

```sh
cargo run --locked -p xtask -- perf --base origin/main --rounds 11 --align-code --filter trace/
```

The base revision lacks the API, so the compact cases are `new` and the comparison is
**old layout against new layout within one run, on the same binary and rays**; the base and
head columns of the unchanged cases are the noise reference (best p50 within 1.3 % in every
run below, apart from the few-nanosecond `box_entry` and `ray_prepare` cases and, in run B,
the base side of `trace/closest_hit_shuffled`, which landed in a slow mode in every round:
247.92 against 226.15 ns on identical code, a [bimodal case](#bimodal-cases)). Head side,
best p50, nanoseconds per ray:

| Run (base revision) | `closest_hit` | `closest_hit_compact` | Change | `any_hit` | `any_hit_compact` | Change |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 112-byte nodes (`57bc035f69f2`) | 222.29 | 197.18 | -11.3 % | 203.86 | 180.87 | -11.3 % |
| 128-byte aligned nodes (`57bc035f69f2`, `--filter _hit`) | 221.64 | 195.64 | -11.7 % | 207.28 | 177.86 | -14.2 % |
| Final code, run A (`cda15ecc020a`) | 218.63 | 196.37 | -10.2 % | 203.86 | 184.20 | -9.6 % |
| Final code, run B (`0e84fff31a3c`) | 220.58 | 198.69 | -9.9 % | 204.63 | 184.32 | -9.9 % |

Node order, same runs (best p50, ns per ray; the library writes the depth-first order):

| Order | 112-byte run | aligned run | Final run A | Final run B |
| --- | ---: | ---: | ---: | ---: |
| depth first, left subtree first (`closest_hit_compact`) | 197.18 | 195.64 | 196.37 | 198.69 |
| builder's order of the source nodes (`_builder_order`) | 198.28 | 192.99 | 196.57 | 197.99 |
| depth first, child nearer the camera first (`_near_first`) | 194.95 | 190.63 | 193.48 | 194.25 |
| seeded random order (`_shuffled`) | 203.53 | 197.75 | 201.78 | 201.45 |

Conversion of the scene's tree (`trace/compact_convert`, one call): 421.4 to 431.6 µs
best p50, about 15 ns per compact node.

**Reading.** The compact layout is 10 to 12 % faster for both searches in every run, about
22 to 26 ns per ray, far outside the noise reference. Aligning nodes to 128 bytes moved
`closest_hit_compact` by 0.8 % (inside the noise) for 14 % more memory, so the node stays at
112 bytes. Placement changes the result by a few percent at most: the camera-aware order is
1 to 2.5 % faster than the default and a random order 2.5 to 3 % slower, consistent with
the shuffled builder tree. The near-first order depends on the camera, so the library does
not use it; a renderer that wants it can reorder the public nodes.

**x86-64 (this pull request's `Perf / before and after` job,** a shared GitHub x86-64 Linux
runner, `--rounds 7 --align-code`, base `0e84fff31a3c`; coarse, and absolute times are not
comparable with the Mac's). Best p50, nanoseconds per ray:

| Case | Base side | Head side |
| --- | ---: | ---: |
| `trace/closest_hit` | 360.02 | 375.59 |
| `trace/closest_hit_compact` | — | 317.16 |
| `trace/closest_hit_compact_near_first` | — | 315.55 |
| `trace/closest_hit_compact_shuffled` | — | 321.89 |
| `trace/any_hit` | 331.20 | 346.15 |
| `trace/any_hit_compact` | — | 283.26 |
| `trace/compact_convert` (one call) | — | 308 838 |

The compact searches are 12 to 16 % (closest) and 14 to 18 % (any) faster than the old
layout on that runner, against the base and the head side respectively, so the layout is
not worse on x86-64; it gains at least as much as on the Mac. The unchanged
`closest_hit` and `any_hit` read 4.3 and 4.5 % slower on the head side than on the base
side, although neither function changed, while the other unchanged cases stayed within
about 2 %: placement noise in a different binary, but it means the head-side ratio
overstates the gain a little. On this runner the address-chained box replay costs only
24 ns per ray more than the limit-chained one (100 ns on the Mac): dependent loads are
relatively cheaper there, and its gain must come at least as much from the leaf visits and
node reads the layout removes.

**Trade-offs.**

- *Memory.* The same size as the source tree: 27 638 nodes of 112 bytes against 55 277 of
  56 bytes (3.10 MB each). The compact copy is in addition to the source tree, which
  `build_bvh` still needs, so a caller that keeps both holds twice the node memory.
- *Conversion.* About 0.43 ms for this tree, a pass that pays for itself after roughly
  20 000 rays (0.43 ms / 22 ns).
- *A second layout to maintain.* `compact_hit.rs` repeats the search of `closest_hit.rs`;
  any change to the order or pruning rules must be made in both, and the call-for-call
  tests (TRACE-TP-301, PERF-TP-045) are what keeps them in step.
- *Malformed input.* Hand-built compact trees can contain reference cycles; the search
  bounds its interior visits by the node count and reports `InvalidNodeIndex` instead of
  looping (found by a mutant that hung; see the ray-tracing test plan).

**Not tried.** Quantized child boxes relative to the parent (the usual compact-BVH trick).
On paper it is not promising here: the estimate above shows that bytes are not what a step
waits for, the shuffled tree shows that the 3 MB array's placement barely matters, and the
box test is limited by its own `u128` arithmetic, to which dequantization (six
multiply-adds per box, with conservative rounding) would add; looser boxes would also make
the search visit more nodes. It becomes worth a trial for trees that leave the cache, which
this scene's does not.

**Unmeasured.** wasm32, ARM Linux and Windows runners; local x86-64; other scenes and ray
batches; scenes whose trees do not fit the cache; whether the gain survives a different builder (flyingrobots/bunny#213) or packets
(flyingrobots/bunny#217).

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
   (Tried in flyingrobots/bunny#215: the layout paid 10 to 12 % of the ray, by removing one
   dependent load per step rather than by cutting bytes; see ledger entry 7.)
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
   payoff.
5. **Rejecting far triangle hits before dividing: #214 cannot pay off on this scene.** Only
   0.02 of the 0.70 divisions per ray are rejected by the distance limit afterwards, worth
   about 0.3 ns. Accepted divisions are 1.03 per sphere ray and 1.00 per ground ray, one
   per hit plus 0.03, so there is no division left to avoid. All triangle tests together are
   37 to 38 ns (17 %): free, the ray would cost about 178 ns, and the division is at most
   10 ns of that. A cheaper rejecting test (15 ns per ray) is a different idea from #214.

No single item reaches the stage 2 target of under 100 ns per ray: free box tests give about
87 ns, and nothing else comes close. The target needs both fewer box tests (builder,
packets) and a lower cost per box test or per visit (layout, 4-wide).

### Memory share of a box test

Added for flyingrobots/bunny#215, before building a compact node layout, to see how much any
layout could gain. `crates/bunny-perf/src/memory.rs` replays the same 20 238 box tests of
the 1 024 `closest_hit` searches as `trace/kernel_box_entry`, on the real nodes in the
search's order, in four ways, and runs `closest_hit` on a copy of the tree whose sibling
pairs sit in seeded random slots (the search makes the same calls in the same order there,
so only placement differs). A test checks each replica and the shuffled tree against the
shipped functions on every call or ray (PERF-TP-042 to PERF-TP-044).

Apple M5 Pro, `flyingrobots/rust-dev:1.96.0` container, two runs of
`perf --base origin/main --rounds 11 --align-code --filter trace/` with `--exclusive`
(base `8cdbb1cb38f5` and `57bc035f69f2`); head side, best p50, nanoseconds per ray:

| Case | Run 1 | Run 2 | What it replays |
| --- | ---: | ---: | --- |
| `trace/kernel_box_entry` | 131.06 | 132.00 | `box_entry`, node indices known in advance |
| `trace/kernel_box_entry_chain` | 133.91 | 133.95 | the same, each limit depending on the last result |
| `trace/kernel_box_entry_addr_chain` | 233.93 | 239.38 | the same, each node index depending on the last result |
| `trace/kernel_box_load` | 12.21 | 12.17 | only the loads of the six bounds, indices known |
| `trace/kernel_box_load_chain` | 136.47 | 144.33 | only the loads, each index depending on the last load |
| `trace/closest_hit` | 221.72 | 222.29 | the search on the builder's tree |
| `trace/closest_hit_shuffled` | 227.58 | 225.95 | the search with sibling pairs placed at random |

Reading:

- **Loads that can start early are nearly free.** Reading the bounds of the 19.76 nodes a
  ray tests costs 12.2 ns when the core knows the addresses ahead: 0.6 ns per box test, 9 %
  of the box row.
- **Loads that wait for the previous one are not.** Chained through their addresses, the
  same loads cost 136 to 144 ns per ray, about 7 ns each (this includes the `black_box`
  store and reload that carries the chain, roughly a nanosecond). Chaining `box_entry` by
  address instead of by limit adds 100 to 105 ns per ray, about 5 ns per test. So the
  memory component of a box test is not its bytes but the latency of a load whose address
  comes from the previous step.
- **The search sits between the two.** It costs 222 ns per ray, less than the
  address-chained box replay alone, because its next node depends on a comparison, not on
  loaded data, and the core predicts that branch and starts the next loads early. The
  profile's 44 to 50 ns residual is the upper bound on the latency it does pay.
- **Placement hardly matters.** Shuffling sibling pairs across the 3 MB array costs 0.7 to
  2.6 % (best p50), so which cache lines nodes share is not the problem on this scene; the
  number of dependent loads per step is.

That made a layout worth building, but for a different reason than bytes: the builder's
layout needs two dependent loads per step (the popped node, for its child index, and then
its children's boxes), the compact layout one. The result is
[ledger entry 7](#7-compact-node-layout-with-both-child-boxes-in-the-parent-new-api).

**A two-box replica of `box_entry`.** The same module holds a harness replica of
`box_entry`'s arithmetic from per-ray values prepared once (`floor(2^63 / |d|)` from the
public direction), for one box and for both children at once, axis by axis. It returns
`box_entry`'s exact result on every recorded call and on constructed axis-parallel,
inside-the-box and behind-the-origin rays. Hoisting the preparation changes nothing
(`trace/kernel_box_single` 130.00 and 129.03 ns against 131.06 and 132.00), and testing two
boxes together is slower (`trace/kernel_box_pair` 144.98 and 143.15 ns, +11 %): the
compiler neither shares work nor vectorizes the `u128` products. No two-box helper was added
to `bunny-query`, so nothing here touches `box_entry` (which flyingrobots/bunny#258 is
changing).

### Unmeasured

x86-64 and wasm32; other scenes and ray batches (secondary rays share less between
neighbours); the split of a box test into arithmetic and loads (since estimated: see
[Memory share of a box test](#memory-share-of-a-box-test)); whether a packet or a
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
