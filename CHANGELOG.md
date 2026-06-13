# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.4.0] - Unreleased

### Added

* 16-bit integer coordinate quantization mapping for 3D vertices relative to an AABB boundary (`bunny-mesh`).
* Memory-stable, zero-allocation index buffer layouts with validation rules for 16-bit and 32-bit triangle indices (`bunny-mesh`).
* Cryptographic SHA-256 content-addressable hashing for mesh vertex and index buffers (`bunny-mesh`).

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
