---
title: "Rust Code Standards — Editor's Edition™"
date: 2026-06-17
lastmod: 2026-06-17
profile: "Strict Deterministic Math & Geometry"
repository-family: "Bunny"
---

## Rust Code Standards — Editor's Edition™

This is the engineering doctrine for deterministic Rust math and geometry crates.
It is written under one assumption: this codebase must be read, refactored, tested,
and modified by humans and coding agents without losing bit-level determinism.

The enemy is not just undefined behavior. The enemy is **plausible variation**:
platform-specific floating-point behavior, hidden ambient state, collection-order drift,
unchecked panic paths, nondeterministic reductions, and clever code that cannot be audited
under pressure.

## Rule 0: Bit-Level Determinism Wins

When the engine runs, only one question matters:

> For the same canonical input, do all supported targets compute the exact same output bits?

Required deterministic target family:

- `x86_64` desktop/server targets.
- `aarch64` / ARM64 targets.
- `wasm32-unknown-unknown` browser or sandbox targets.

Any cross-target variation in canonical math, geometry predicates, serialization, ordering,
rounding, tie-breaking, or error behavior is a defect.

**Sensei's Wisdom™**: "Close enough" is graphics talk. Canonical geometry speaks bits.

## Rule 1: Unsafe Code Is Banned in Core Crates

Core crates must declare or inherit:

```rust
#![forbid(unsafe_code)]
```

No `unsafe` blocks. No unsafe functions. No `transmute`. No raw-pointer games. No `no_mangle`,
`export_name`, or `link_section` tricks in core crates.

If future acceleration requires FFI, SIMD backends, GPU interop, or platform-specific unsafe code,
it must live in a quarantined backend crate with:

1. A safe deterministic reference implementation.
2. Equivalence tests against the reference implementation.
3. Documented `SAFETY:` comments for every unsafe block.
4. A public API that cannot leak unsafety into the core domain.
5. Explicit approval in review.

Core remains clean. Speed does not get to tunnel under the monastery wall.

## Rule 2: Canonical Math Is Fixed-Point, Not Native Float

Native `f32` and `f64` values are permitted only as boundary convenience formats:
file ingress, DTOs, debug views, renderer adapters, external API compatibility, or lossy display.
They are not canonical geometry.

Inside deterministic core computations:

- Canonical coordinates use project-defined fixed-point values.
- Arithmetic uses explicitly documented overflow and rounding policy.
- Float arithmetic operators are banned unless the operation is proven non-variant and documented.
- Float methods such as `sin`, `cos`, `tan`, `sqrt`, `hypot`, `powf`, `exp`, `ln`, and `log` are banned in canonical math.
- Branching on `NaN` behavior is banned in core logic.

The default fixed-point type shape is a private newtype, not a public raw integer costume:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Q32x32(i64);
```

Expose `from_raw` and `raw` deliberately. Do not expose the field as `pub`.

See `docs/NUMERIC_CONSTITUTION.md` for the arithmetic law.

**Sensei's Wisdom™**: A public `pub i64` inside a fixed-point type is not encapsulation. It is a bead curtain.

## Rule 3: Panic Paths Are API Bugs

Library code must not contain uncontrolled panic paths.

Banned in core crates:

- `unwrap()`
- `expect()`
- `panic!()`
- `todo!()`
- `unimplemented!()`
- unchecked indexing where the local proof is not obvious
- arithmetic that can overflow without a documented policy

Prefer:

- `Result<T, DomainError>` for recoverable invalid input.
- `Option<T>` for legitimate absence.
- `checked_*` operations when overflow is domain-invalid.
- explicit `Saturating` operations only when saturation is part of the geometry contract.
- local proof comments for rare indexing that is deliberately safe.

Permitted narrow exception:

```rust
// dojo: allow indexing -- loop bounds prove i < points.len(); covered by degeneracy tests
let point = points[i];
```

The waiver must be local, specific, and boring. A waiver that says "safe" is not a waiver; it is a shrug.

## Rule 4: Side Effects Stay Outside the Core

Core crates must never access ambient host state:

- system time
- wall-clock timestamps
- thread-local RNGs
- filesystem
- network
- environment variables
- process arguments
- current working directory
- locale-dependent formatting

If the capability is necessary, inject an explicit boundary-owned value whose behavior is deterministic,
replayable, and test-controlled.

Examples:

- `ClockPort` or `TickSource` instead of `SystemTime::now()`.
- seeded deterministic generator instead of `thread_rng()`.
- in-memory adapter instead of filesystem access.
- passed configuration struct instead of `std::env::var()`.

**Sensei's Wisdom™**: Ambient state is global mutable state wearing a fake mustache.

## Rule 5: Deterministic Collections and Ordering

Core crates must not allow hash randomization, insertion order, scheduler order, or map traversal order
to affect canonical output.

Preferred deterministic structures:

- sorted `Vec`
- `BTreeMap`
- `BTreeSet`
- project-defined deterministic maps where output order does not depend on hash table iteration

Banned in canonical output paths unless documented and proven irrelevant:

- `HashMap`
- `HashSet`
- iteration over unordered maps/sets
- serialization of unordered collections without sorting

Tie-breaking must be explicit. If two geometric events compare equal by primary key, the secondary key
must be defined in the API or algorithm notes.

## Rule 6: Parallelism Must Not Change Results

Parallelism is allowed only when output is independent of scheduling.

Rules:

- Reductions must use a fixed tree order or deterministic partitioning.
- Work-stealing order must not affect canonical output.
- Error selection must be deterministic when multiple partitions can fail.
- Saturating arithmetic and checked arithmetic must not produce order-dependent results.
- Parallel code requires single-thread equivalence tests.

**Sensei's Wisdom™**: "It only flakes on CI" means CI is the only honest machine in the room.

## Rule 7: Geometry Degeneracy Is Not an Edge Case

Every geometric predicate and transformation must document its degeneracy behavior.

Required documented cases where applicable:

- zero-length vectors
- zero-area triangles
- zero-volume boxes
- duplicate vertices
- collinear points
- coplanar faces
- touching boundaries
- parallel rays/segments/planes
- empty meshes
- inverted bounds
- overflow-domain inputs

Boundary contact semantics must be named: inclusive, exclusive, half-open, or explicitly invalid.

A test suite that does not include degenerates is a demo, not a defense.

## Rule 8: Serialization Is Canonical and Versioned

Canonical serialization must be:

- endian-explicit
- schema-versioned
- independent of map iteration order
- independent of platform pointer width
- explicit about signedness and fixed-point raw-bit representation

Fixed-point values serialize as raw signed integer bits in the project-specified byte order.
DTO float formats are lossy boundary formats and must not be used as canonical snapshots.

## Rule 9: Crate Categories Define Strictness

### Core Runtime Crates

Applies to:

- `bunny-num`
- `bunny-linalg`
- `bunny-geom`
- `bunny-query`
- `bunny-broadphase`
- `bunny-mesh`

Targets:

- file length ≤ 300 source lines
- line length ≤ 100 characters, except URLs and long paths
- function length ≤ 25 source lines
- statements per function ≤ 15
- nesting depth ≤ 3
- parameters ≤ 4
- approximate cyclomatic complexity ≤ 6
- no unchecked panic paths
- no ambient state
- no native float arithmetic in canonical math

### Generator Crates

Applies to:

- `bunny-wesley`

Targets:

- file length ≤ 500 source lines
- function length ≤ 50 source lines
- nesting depth ≤ 4
- panics allowed only for unrecoverable build-time schema validation errors
- generated code must be deterministic from the same inputs

### Build Tooling Crates

Applies to:

- `xtask`

Targets:

- file length exempt
- scripting panics allowed when failure is unrecoverable and local
- must compile clean under workspace lint policy
- must not hide failures behind silent shell behavior

## Rule 10: Clippy Is a Guardrail, Not a Random Opinion Generator

Core crates use strict compiler and Clippy posture, but do not blanket-deny unstable lint groups.

Recommended stance:

```rust
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(warnings)]
#![deny(clippy::all)]
#![warn(clippy::pedantic)]
```

`clippy::nursery` must be cherry-picked. Blanket-denying nursery is banned because nursery lints are not mature enough to be a repository constitution.

Clippy suggestions may be rejected when they reduce determinism, local reasoning, or API clarity.
Rejected suggestions require a local allow comment with a reason.

### Audited Workspace Allowance

| Lint | Status | Rationale |
| --- | --- | --- |
| `clippy::multiple_crate_versions` | Permanent until the `wesley-core` graph converges. | The current generator dependency graph brings duplicate `hashbrown`, `thiserror`, `thiserror-impl`, and `tower` versions through `wesley-core` and its parser stack. `cargo deny check` keeps these duplicates visible as warnings, but denying this Clippy lint would block the workspace for dependencies Bunny does not directly own. |

The former workspace allowances for `clippy::module_name_repetitions` and
`clippy::must_use_candidate` were removed after an audit showed the current
workspace passes with those lints re-enabled. Narrow local
`module_name_repetitions` allowances remain where module names are part of a
public API boundary.

To ratchet the remaining allowance, first run Clippy with
`clippy::multiple_crate_versions` re-enabled and record the dependency surface.
Remove the allowance once the transitive generator stack converges to a single
version for each duplicated crate.

## Rule 11: Tests Prove Behavior, Not Choreography

Core tests must prefer:

- deterministic golden vectors
- property-style tests with fixed seeds
- cross-target snapshot checks
- in-memory fakes for ports
- degenerate geometry cases
- serialization round trips
- equivalence tests between optimized and reference algorithms

Avoid tests that only prove implementation choreography.

For every core numeric crate, maintain golden vectors for:

- addition
- subtraction
- multiplication
- division
- rounding ties
- negative rounding ties
- min/max raw values
- overflow boundaries
- canonical serialization

For every geometry crate, maintain deterministic tests for degenerates and boundary contact semantics.

## Rule 12: Commits Tell a Deterministic Story

A commit should explain one causal change. Do not mix unrelated migrations, formatting churn, behavior changes,
and generated updates into one blob.

Commit subject format:

```text
<scope>: <imperative summary>
```

Examples:

```text
bunny-num: define checked Q32x32 multiplication
bunny-geom: make ray-box boundary contact inclusive
bunny-mesh: add golden vectors for duplicate vertex collapse
```

Every non-merge commit must include a receipt trailer:

```text
Repo-Respect-Receipt: .repo-respect/receipts/<id>.md
```

Every pull request must add or update a receipt under `.repo-respect/receipts/`.
Receipts are required for all contributors and all contribution methods. The
receipt must identify the bounded context used, files read, files edited, topic
documentation impact, generated artifact impact, checks run, known risks, and
human reviewer.
Bare receipt IDs are invalid; the trailer must use the full receipt path.

Create a receipt template with:

```bash
cargo run --locked -p xtask -- repo-respect receipt <short-topic>
```

**Sensei's Wisdom™**: `--no-verify` is a confession, not a strategy.
