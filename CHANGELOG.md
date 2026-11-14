# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added

* Added two `bunny-perf` cases in the `trace` group, `trace/triangle_hit` and
  `trace/triangle_beyond_limit`, which time `TraceRay::hit_triangle` alone on the
  benchmark scene's nearest hits with no limit and with the limit one raw unit
  short. Added a frozen reference copy of the triangle test's arithmetic
  (`crates/bunny-query/tests/support/trace_reference.rs`) with equivalence
  tests over seeded, constructed and domain-corner corpora at limits around the
  true distance, and scene tests that compare every triangle test of
  `trace/closest_hit` with it and count how many reach the division. No library
  function changes. A pre-check that rejects hits beyond the limit before the
  division (flyingrobots/bunny#214) was measured and not shipped: only 24 of 714
  crossings on the scene are beyond the limit, and the result is within noise;
  see "Tried And Rejected" in the performance chapter.

## [0.7.0] - 2026-11-13

This is an early cut of the 0.7 line. It exists so downstream repositories can
use the view, projection, ray-tracing and clipping APIs now; the rest of the
`v0.7.0 Geometry Law and Primitive Coverage` milestone (quaternions,
interpolation and curves, degeneracy policy, robust predicates, shapes,
topology, mass properties) is not in this release and continues as additive
`0.7.x` releases. `bunny-optics` is a new published crate. `bunny-perf` is a
workspace tool and is not published.

### Changed (behaviour)

* `bunny_query::ray_intersects_aabb`, and so
  `bunny_broadphase::intersect_ray`, no longer report a miss when a slab
  distance or a coordinate difference overflows. The inputs that change are
  exactly those for which the old function stopped on an overflowing
  `bound - origin` subtraction or `(bound - origin) / direction` division:
  a direction component of a few raw units (for a slab of half-width `k`
  centred on the origin, any component of at most `2k` raw units), or a bound
  2^31 units or more from the origin. For them the function now returns
  `Some((t_enter, t_exit))` when the ray reaches the box ahead of its origin
  and `None` when it truly misses, decided exactly with `i128` rationals; a
  slab distance that does not fit Q32.32 saturates to the minimum or maximum
  by sign, and coordinate differences are widened to `i128` instead of
  saturated. Every hit the function reported before is reported with the same
  bits, and every miss not caused by overflow is still a miss; a differential
  test against the retained old implementation and an exact rational oracle
  check both. Per the Fidelity Rule this contract change needs a version that signals it,
  which is why this release is `0.7.0` and not a patch release (flyingrobots/bunny#211,
  PR #236).

### Added

* Added the `bunny-optics` crate with a deterministic pinhole camera.
  `PinholeCamera::new` takes a position, a look direction, an up hint, the
  tangent of half the vertical field of view (no trigonometry) and an integer
  image size, derives a right-handed basis (`right = forward x up`) and refuses
  degenerate input with a typed `CameraError`. `PinholeCamera::ray` returns a
  `bunny_query::TraceRay` for a `PixelSample` (pixel plus sub-pixel offset in
  `[0, 1)`; pixel `(0, 0)` is top-left and rows grow downward, as in the
  viewport conversion) with no square
  root or division per ray: directions are not normalized but scaled by
  `2^-scale_shift`, chosen at construction from the image corners so every
  component is at most 1.0. Each component is within one raw unit plus
  `(width + height) / 2^33` of the exact image-plane point. `point_at` gives the
  point at a hit distance. Images up to 32768 pixels per side and tangents up
  to 256.0 are accepted. See the new `docs/topics/optics/` chapter and test plan.
* Added `bunny-perf --stats` and the `trace/kernel_*` cases: per-ray counters
  for `closest_hit` and `any_hit` (nodes, box tests, triangle-test stages,
  divisions, tree shape, shared nodes between neighbouring rays) from a harness
  replica of the search loop that a test checks against the shipped functions on
  every benchmark ray, and timed replays of the box and triangle tests the
  search makes. The performance chapter's new profile section says where the
  time of `trace/closest_hit` goes and ranks the stage 2 options. No library
  function changed and the core crates carry no counters.
* Added `FixedDivisor`, a Q32.32 divisor prepared for repeated exact division
  (Möller–Granlund division by an invariant integer): `divisor.checked_div(a)`
  returns exactly `a.checked_div(divisor)`, `None` included. Best p50 on an
  Apple M5 Pro: 3.5 to 4.0 ns against 6.4 to 6.5 ns for
  `FixedQ32_32::checked_div` (two runs, ledger entry 5).
* Added `PreparedRay3` and `ray_intersects_aabb_prepared`: the same
  `(enter, exit)` bits as `ray_intersects_aabb` on every input, including the
  overflow inputs that `ray_intersects_aabb` now decides exactly
  (flyingrobots/bunny#211), with no division per box. Best p50: 21.5 to 21.7 ns
  against 51.5 to 52.3 ns in the same runs, after a 26 ns preparation. For ray
  traversal that does not need exact distances, `TraceRay::box_entry` (6.0 ns
  after a 1.8 ns preparation) is faster, and on an x86-64 CI runner the
  prepared box test is only 1.11 times faster; see the ledger entry.
* Added `Viewport` and `ViewportError` to `bunny-linalg`: checked Q32.32
  conversion between NDC and viewport coordinates (`try_ndc_to_viewport`,
  `try_viewport_to_ndc`). The origin is the top-left corner, pixel `y` runs
  down, coordinates are continuous with pixel corners on integers, each
  coordinate is the floor of the exact raw value, and depth passes through. The
  round trip is within one raw unit per axis. The coordinate law's new
  Viewport Conversion section records the choices; golden vectors run natively
  and under WebAssembly. Existing functions are unchanged.
* Added `FixedMat4::try_look_at` and `LookAtError` to `bunny-linalg`: a checked
  Q32.32 right-handed look-at `view_from_world` matrix (camera looks down `-Z`).
  The basis is orthonormal within 2 raw units in length and pairwise dot
  product; eye equal to target, a zero or parallel `up`, and overflow return
  typed errors. Golden vectors run natively and under WebAssembly.
* Added the `scene` and `trace` feature groups to `bunny-perf`: a whole-scene
  ray benchmark (27 650 triangles, 1 024 camera rays) timing closest hit through
  the visitor API and through `closest_hit`, with a check that both choose the
  same primitive on every ray, and added `bunny-broadphase` to the `xtask perf`
  base crates. Ledger entry 4 in `docs/topics/performance/` records the
  measurement.
* Added `FixedPlane3::clip_polygon`, `PolygonClip` and `PolygonClipError` to
  `bunny-geom`: Sutherland-Hodgman clipping of a convex polygon to the closed
  front half-space, written into a caller-provided buffer (`n + 1` slots always
  suffice; a smaller one returns `OutputTooSmall`). Winding is preserved and
  output starts at the first kept input vertex; a vertex on the plane is kept
  without a duplicate cut; a result with fewer than three vertices is
  `Outside`. Cut points are computed in a canonical edge direction (ascending
  raw `(x, y, z)`) so polygons sharing an edge get identical points, because
  `clip_segment` can differ by direction at a rounding tie. Existing functions
  are unchanged. See `docs/topics/plane-clipping/`.
* Added `FixedMat4::try_orthographic` and `FixedMat4::try_perspective` to
  `bunny-linalg`: checked Q32.32 `clip_from_view` projection matrices that take
  a tangent (no trigonometry) and return the new `ProjectionError`. The
  coordinate law now defines the canonical NDC depth range `[0, 1]` (near to
  `0`, far to `1`) and a `-Z` camera, with golden vectors that run natively and
  under WebAssembly.
* Added closest-hit and any-hit BVH ray traversal to `bunny-broadphase`:
  `closest_hit`, `any_hit`, `ClosestHit` and `BvhView` (the borrowed node and
  primitive-index buffers). The search is front to back over existing
  `BvhNode` trees, uses `TraceRay::box_entry` for nodes and a caller closure
  for primitives, skips subtrees whose entry bound is beyond the best hit, and
  gives ties to the lowest primitive index. Its answer equals a test of every
  primitive whatever the tree's shape, under the caller conditions stated in
  the `docs/topics/ray-tracing/` chapter, which now documents the traversal
  and its correctness argument. `intersect_ray` and the other existing
  functions are unchanged.
* Added ray-tracing kernels to `bunny-query`: `TraceRay`, `TraceTriangle` and
  `TriangleHit`. `TraceRay::box_entry` is a division-free conservative ray-box
  test that never rejects a box the ray reaches and returns an entry bound
  within `2^11 + 2` raw units plus `distance / 2^30`; `TraceRay::hit_triangle`
  is an exact integer ray-triangle test (two-sided, inclusive edges, floored
  distance, barycentrics on demand). Inputs are limited to a trace domain of
  coordinates below 256.0 and direction components at most 1.0. Existing
  `bunny-query` functions are unchanged. See the new
  `docs/topics/ray-tracing/` chapter and test plan.
* Added `FixedPlane3`, `PlaneSide`, `FixedSegment3` and `SegmentClip` to
  `bunny-geom`: checked signed distance, three-way point classification, and
  segment clipping to the closed front half-space (a point exactly on the plane
  is kept). The clipped endpoint uses one checked division and rounds
  ties-to-even. See `docs/topics/plane-clipping/`.
* Added `crates/bunny-perf`, a timing harness for Bunny's math and query
  kernels (p50/p95/p99 nanoseconds per call on seeded inputs), and
  `cargo run --locked -p xtask -- perf --base <rev>`, which builds the same
  harness against a base revision and the working tree, runs them interleaved
  and prints a before/after table (median and best p50 across rounds;
  `--align-code` removes code-layout luck).
* `cargo xtask perf --target <triple> --runner <command>` measures other
  targets, such as WebAssembly under wasmtime, and every report names the
  target it measured.
* The `Perf` workflow has a hand-started, report-only platform matrix (x86-64
  and ARM Linux, Windows, Apple Silicon macOS, WebAssembly), and the
  performance chapter states the rules for target-specific implementations.
* Added the `Perf` GitHub workflow: pull requests get a before/after table in
  the job summary and fail when a case's best p50 slows by more than 20 %.
* Added the `docs/topics/performance/` chapter: the fidelity rule every
  optimization must satisfy, how measurements are made and read, and the
  optimization ledger.
* Added `FixedAngle`, Bunny's canonical angle (#110): a binary angle with
  `2^64` units per turn, wrapping arithmetic, and exact or symmetric-rounding
  conversions from turns, degrees and radians.
* Added deterministic `sin`, `cos` and `sin_cos` on `FixedAngle`: a
  quarter-wave table built at compile time by an integer Taylor series, cubic
  Hermite interpolation, and quadrant symmetry. Results are within one raw
  Q32.32 unit of the exact value, exact at quarter turns and
  bit-exact under negation, with golden vectors and the
  `docs/topics/angle-trigonometry/` chapter.

### Changed

* Added `isqrt_u128_two_divides` (crate-private), an exact integer square root that
  finishes with two hardware `u64` divisions, with unit tests against core's
  `u128::isqrt`. Re-measured on the corrected harness it makes `sqrt` 1.33 times and
  `length` 1.28 times faster but `normalize` about 4 % slower on aarch64, so every
  target keeps core's implementation; `sqrt_u128` output and speed are unchanged. An early
  version that marked `sqrt_u128` `#[inline]` read 71 % slower on `num/sqrt` on x86-64 CI
  and was fixed (see the performance ledger, entry 3).
* `FixedQ32_32::sqrt_u128` (and so `sqrt`, `FixedVec3::length`, `normalize`
  and the ray queries) now uses the table-seeded integer square root from the
  core library: the same exact bits as before, 4.9 times faster for `sqrt`,
  2.1 times for `normalize` and 1.6 times for a sphere hit (best p50, Apple
  M5 Pro in the rust-dev container). Equivalence tests keep the old bit-by-bit loop as the reference;
  the performance chapter's ledger records the method, trade-offs and full
  measurements.
* The lockfile takes `ninelives` 0.2.1 and `tokio` 1.54.1 (transitive
  dependencies of the host-side `wesley-core` tooling) to clear RUSTSEC-2026-0386.
  No published crate's dependency requirements changed.
* Bumped every publishable Bunny crate to `0.7.0`, including the inter-crate
  dependency requirements; regenerated contract witnesses now identify the
  generator as `bunny-wesley/0.7.0`. `bunny-optics` is added to the publish
  order after `bunny-query`.

### Fixed

* Fixed `bunny-perf` so every feature group builds on its own: the `case` helper
  compiled only with the `scene` or `trace` group, which made `cargo xtask perf`
  report the `num`, `linalg`, `query` and `angle` cases as new instead of
  comparing them against a base revision.
* Two `bunny-codec` test helpers use `as_chunks`, so clippy from newer
  toolchains no longer fails the gate.

## [0.6.0] - 2026-06-26

### Added

* Added the `docs/topics/coordinate-law/` living topic chapter and test plan for
  Bunny's right-handed coordinate frame, unit policy, transform naming
  convention, NDC reservations, and documentation source-of-truth rules.
* Added `docs/README.md` as the documentation spine for living topic chapters,
  test plans, historical design records, and PR documentation gates.
* Added `CONTRIBUTING.md` to explain Bunny's current-truth documentation
  technique and contract-graph workflow for new contributors.
* Added `cargo run --locked -p xtask -- topic-docs` and wired it into Code
  Dojo so topic test plans validate stable requirement IDs, case IDs, explicit
  oracles, evidence status, and implemented Rust test names.
* Added universal repo-respect receipt enforcement to Code Dojo and local Git
  hooks, plus `cargo run --locked -p xtask -- repo-respect receipt
  <short-topic>` for creating receipt templates before commit or PR handoff.
* Added the `docs/topics/deterministic-contract-profile/` living topic chapter
  and test plan for Bunny-owned deterministic scalar profiles, generated
  witnesses, and future codec boundaries.
* Extended `bunny-wesley` scalar profiles with deterministic wire metadata for
  integer, fixed-byte, bounded-byte, UTF-8, fixed-point, and boundary-float
  profiles, plus generated Rust, TypeScript, and manifest witnesses.
* Added checked Q32.32 addition, subtraction, negation, and multiplication APIs
  so geometry code can reject overflow instead of consuming saturated
  intermediates.
* Added deterministic `FixedMat2`, `FixedMat3`, and `FixedMat4` matrix
  primitives with row-major layout, checked multiplication, `determinant()`, and
  `try_inverse()` APIs.
* Added deterministic `FixedAffine2` and `FixedAffine3` affine transform
  primitives with `checked_transform_point()`, `checked_transform_vector()`,
  `checked_mul_affine()`, and `try_inverse()` APIs.
* Added deterministic seeded property-test corpora for Q32.32 raw/order
  invariants, vector algebra identities, query symmetry and bounds, and mesh
  quantization round trips.

### Changed

* Bumped the publishable Bunny workspace crates to `0.6.0`; regenerated contract
  witnesses now identify the released generator as `bunny-wesley/0.6.0`.
* Reworked `docs/MATH_GEOMETRY_CAPABILITY_MAP.md` into a prose-led planning
  reference with compact tables for navigation and issue anchors.
* Added explicit release cut policy and cut gates to `ROADMAP.md`, including
  must-ship, may-slip, and not-included boundaries for the planned release
  train, and refreshed `docs/BEARING.md` through the `v0.6.0` release cut and
  next `v0.7.0` sequence.
* Ray intersection queries now return `None` when checked intermediate
  arithmetic overflows, rather than accepting saturated Q32.32 values as valid
  geometric hits.
* `scripts/publish-crates.sh publish` now refuses dirty worktrees
  unconditionally; `ALLOW_DIRTY=1` is limited to local `verify` and `dry-run`
  diagnostics.
* Removed global `clippy::module_name_repetitions` and
  `clippy::must_use_candidate` allowances after auditing that the workspace
  passes without them.

### Fixed

* `bunny-wesley` now rejects schema type names reserved for generated helper
  types instead of emitting Rust duplicate type items or TypeScript interface
  merges.
* `bunny-wesley` now rejects field-level `@bunnyScalarProfile` directives until
  field override semantics are implemented, instead of silently omitting them
  from generated witnesses.
* Repo-respect fixture repositories now disable commit signing during tests so
  the gate does not depend on the contributor's local GPG configuration.
* Repo-respect receipt coverage now includes deleted and typechanged paths,
  validates staged receipt contents from the Git index, rejects placeholder-only
  receipt sections, and enforces receipt trailers across non-merge branch
  commits in the full gate; the repo-respect topic records the evidence as
  `RR-TP-003` through `RR-TP-009`.
* Repo-respect branch commit validation now checks only commits on the PR side
  of the merge base, uses shared sanitized Git subprocess helpers, and rejects
  receipt sections whose only apparent content is prose field names or HTML
  comments; the repo-respect topic records the evidence as `RR-TP-004`,
  `RR-TP-005`, `RR-TP-010`, and `RR-TP-011`.
* `FixedMat2::try_inverse` now divides off-diagonal entries before negating
  them, so minimum raw values can still invert when the divided inverse entry is
  representable.
* `FixedMat3::try_inverse` and `FixedMat4::try_inverse` now divide negative
  cofactors before negating them, so minimum raw cofactors can still invert when
  the divided inverse entry is representable.
* Affine inverse computation now applies the inverse linear transform before
  negating translation, so minimum raw translations can still invert when the
  scaled inverse translation is representable.
* Historical goalpost evidence now distinguishes retired `ci.yml` anchors from
  the current Code Dojo workflow.

## [0.5.0] - 2026-06-19

### Added

* Rust Code Standards Editor's Edition docs, Numeric Constitution, Sensei's
  Wisdom, Code Dojo docs, repo-local Git hooks, and repo-respect receipt
  scaffolding.
* Code Dojo local and CI enforcement scripts for source-shape policy,
  deterministic test receipts, formatting, Clippy, tests, and WASM checks.
* Deterministic receipt integration tests and crate README/package metadata
  needed by the new standards gate.
* Validating Q32.32 float ingress through `FixedQ32_32::try_from_f32`,
  `TryFrom<f32>`, and the raw `fixed_q32_32::try_from_f32` helper.
* Math and geometry capability map covering Bunny's owned surface, non-goals,
  missing deterministic math layers, SIMD policy, and recommended build order.
* Planned post-v0.4.0 roadmap release train with GitHub milestones, goalpost
  trackers, and slice issue references.

### Changed

* `FixedQ32_32` now keeps its raw field private, derives `Hash`, exposes
  `raw()` as the canonical raw accessor, and keeps `to_raw()` as a compatibility
  alias.
* Float-to-fixed geometry ingress now rejects finite values outside the Q32.32
  range instead of allowing them to saturate silently.
* Replaced the old quality CI workflow with `.github/workflows/code-dojo.yml`.
* Root and crate manifests now inherit the workspace lint baseline from the new
  standards pack.
* Rust policy checks now run through the `xtask` AST-backed Code Dojo checker
  plus strict package-scoped Clippy enforcement instead of regex source scans.
* Code Dojo now scans tracked and untracked nonignored Rust files in full-gate
  mode and no longer exposes rollout skip knobs for Cargo, deterministic
  receipts, or WASM checks.
* `ROADMAP.md` now reserves `v0.5.0` for this standards and numeric-law release,
  and shifts the planned math and geometry feature train to `v0.6.0` onward.
* `docs/BEARING.md` now avoids branch and pull-request tracking, leaving live
  work state to GitHub while keeping durable release posture in the repo.
* Front-door signposts now describe the current workspace crate surface,
  generator command, release posture, and future math train without claiming
  shipped crates or volatile PR state as planned work.
* Dependency policy is now part of Code Dojo through `cargo deny check`.
  Duplicate transitive-version findings from the current `wesley-core` graph
  remain visible cargo-deny warnings rather than hidden skip exemptions.
* Refactored broadphase, codec, numeric, query, mesh, and generated contract
  code until the full Code Dojo and headless WASM gates pass.

### Fixed

* crates.io publication recovery now tolerates registry propagation delays and
  rate-limit responses while publishing workspace crates in dependency order.
* Generated contract witnesses now report the `bunny-wesley/0.5.0` generator
  identifier used by this release.

### Removed

* Removed the local issue generator in favor of GitHub as the backlog source of
  truth.

## [0.4.0] - 2026-06-16

### Added

* 16-bit integer coordinate quantization mapping for 3D vertices relative to an AABB boundary (`bunny-mesh`).
* Memory-stable, zero-allocation index buffer layouts with validation rules for 16-bit and 32-bit triangle indices (`bunny-mesh`).
* Cryptographic SHA-256 content-addressable hashing for mesh vertex and index buffers (`bunny-mesh`).
* Zero-copy binary PLY and OBJ text parser contracts in `bunny-codec`, with
  borrowed mesh views and Stanford Bunny fixture regressions.
* Zero-allocation OBJ vertex and triangle iterators in `bunny-codec` for
  forward full-mesh traversal without quadratic indexed-access scans.
* Captured GP2 witness table with repo-truth anchors for each completed file
  format adapter claim.
* GP3 Bunny-native compressed mesh decoder in `bunny-codec`, including
  `decode_compressed_mesh`, a zero-allocation borrowed `CompressedMesh` view,
  `CompressedIndexWidth`, typed compressed triangle variants, and explicit
  malformed-input errors.
* Canonical compressed triangle fixture bytes plus width-16, width-32,
  malformed-input, native allocation, and WASM-compatible decoder regressions.
* crates.io release automation that verifies publishable package archives and
  publishes all public Bunny crates in dependency order when a GitHub Release is
  published.

### Changed

* Raised the workspace Rust toolchain contract and CI pin to Rust 1.96.0.
* `Aabb3` to `FixedAabb3` conversion now rejects non-finite coordinates and
  inverted float bounds before fixed-point quantization.
* Mesh hashes now include quantization bounds so identical quantized buffers under different AABBs produce distinct verification IDs.
* `Ray3` to `FixedRay3` conversion now rejects non-finite origin and direction
  coordinates before fixed-point canonicalization.
* `Sphere3` to `FixedSphere3` conversion now rejects non-finite centers plus
  non-finite and negative radii before fixed-point quantization.
* BVH traversal now returns explicit errors for malformed node and primitive
  index buffers instead of panicking.
* `bunny-codec` now rejects non-finite OBJ/PLY vertex coordinates and
  out-of-bounds binary PLY face indices before returning borrowed mesh views.
* `bunny-codec` now rejects duplicate or late PLY `format` directives and
  classifies non-triangle PLY polygon payloads before generic trailing-data
  checks.
* `bunny-codec` now handles extreme OBJ float exponents without panicking and
  preserves finite OBJ coordinates with very large decimal mantissas.
* The GP3 compressed profile records its allocation contract explicitly: accepted
  decoding borrows raw payload byte sections and exposes typed checked accessors,
  rather than unsafely reinterpreting arbitrary bytes as typed slices.
* The GP3 compressed decoder now classifies oversized declared payload lengths
  against the canonical profile length before host pointer-width-dependent slice
  checks.
* The workspace crate version now matches the `v0.4.0` release tag, and
  internal crate dependencies carry crates.io version requirements for publish.

## [0.3.0] - 2026-06-13

### Added

* Memory-stable, zero-allocation flat array-backed bounding volume hierarchy (BVH) tree builder.
* SAH (Surface Area Heuristic) split selection solver for optimal tree building.
* Stack-based, deterministic tree traversal solvers for AABB overlap and ray intersection queries.
* Multi-axis Sweep-and-Prune broadphase collision solver with dynamic optimal axis selection.
* Zero-allocation Sweep-and-Prune active overlap pair generator with stable lexicographical index sorting.

## [0.2.0] - 2026-06-12

### Added

* `FixedRay3`, `FixedAabb3`, and `FixedSphere3` bounding volumes defined using fixed-point coordinate structures inside `bunny-geom`.
* Compile-time normalized vector wrappers `FixedUnitVec2` and `FixedUnitVec3` to enforce normalization invariants.
* Ray-Sphere, Ray-AABB, and Ray-Triangle (Möller-Trumbore) deterministic query intersection solvers in `bunny-query`.
* Point-to-Triangle, Segment-to-Segment, and AABB-to-Sphere closest point and minimum distance query solvers in `bunny-query`.

## [0.1.1] - 2026-06-12

### Added

* AST parser mappings to dynamically resolve custom `@bunnyScalarProfile` schema directives in `bunny-wesley` instead of hardcoded strings.
* Saturation-checked mathematical division `FixedQ32_32::checked_div` returning `Option<FixedQ32_32>` for mathematical division guards.
* Q32.32 vector boundary-condition and coordinate saturation verification suites.
* Headless WebAssembly unit testing suite executing inside Node.js using `wasm-pack test`.

## [0.1.0] - 2026-06-12

### Added

* Type-safe `FixedQ32_32` newtype wrapper for deterministic fixed-point math profiles.
* Addition, subtraction, multiplication, division, negation, and assignment operators for `FixedQ32_32` with Ties-to-Even rounding and saturation.
* Deterministic integer square root `FixedQ32_32::sqrt` via a software-defined digit-by-digit binary algorithm.
* Type-safe deterministic fixed-point vectors `FixedVec2` and `FixedVec3` inside `bunny-linalg` with arithmetic operators and dot/cross product utilities.
* Double-width boundary mapping `From`/`Into` conversions between DTO float vectors (`Vec2`, `Vec3`) and fixed-point vectors.
* Strict Rust development rules via `CODE_STANDARDS.md` and crate linter denials.
* Cross-platform GitHub Actions CI suite verifying Ubuntu, macOS, and Windows determinism, plus WebAssembly checks.
* Independent `README.md` layouts for all 5 workspace crates.
* Long-term developmental `ROADMAP.md` mapping versions, goalposts, and slices.
* Repository signposts `VISION.md`, `BEARING.md`, `PROCESS.md`, and `TESTING.md` defining standard workflows and testing rules.
