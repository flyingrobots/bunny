# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

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
