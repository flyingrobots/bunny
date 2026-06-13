# Bunny — BEARING

This signpost summarizes short-term priorities, recent ships, and technical debt. It does not replace the backlog, design docs, or roadmap milestones.

## Where are we going?

**Current Priority**: v0.2.0 / Goalpost 1 — Core Bounding Shapes (`FixedRay3`, `FixedAabb3`, `FixedSphere3`).
* **Active Slice**: Slice 1.1 — Implement `FixedRay3`, `FixedAabb3`, and `FixedSphere3` using `FixedVec3` coordinates.
* **Next Branch**: `feature/geom-fixed-shapes`.

## What just shipped?

* **Crate Documentation Sync** (Merged via docs branch, 2026-06-12):
  Added independent, self-contained `README.md` files for all 5 workspace crates (`bunny-num`, `bunny-linalg`, `bunny-geom`, `bunny-contract`, `bunny-wesley`) to prepare them forcrates.io publishing.
* **Fixed-Point Linear Algebra** (Branch `feature/linalg-fixed-vectors` pushed, 2026-06-12):
  Implemented `FixedVec2` and `FixedVec3` vector operations (dot, cross, normalize, arithmetic traits, conversions) and deterministic binary `FixedQ32_32::sqrt()`.
* **Type-Safe Fixed-Point Math** (Branch `feature/q32-32-operators` pushed, 2026-06-12):
  Created type-safe `FixedQ32_32` numeric wrapper and saturating Bankers' rounding math.
* **Workspace CI Gates** (Branch `feature/q32-32-operators` pushed, 2026-06-12):
  Set up GitHub Actions validation suite running on Linux, macOS (ARM64), and Windows, plus WebAssembly checks.
* **Code Quality Rules**:
  Established `CODE_STANDARDS.md` enforcing strict clippy denies and 300-line limits.

## What feels wrong?

* **Ignored Schema Directives**:
  `bunny-wesley` ignores the `@bunnyScalarProfile` directive arguments on schemas, relying on hardcoded name string comparisons instead.
* **Missing Matrix Math**:
  `bunny-linalg` lacks matrix and quaternion profiles (`FixedMat3`, `FixedMat4`, `FixedQuat`), which are required for transformation queries.
* **Empty Geometry Implementation**:
  `bunny-geom` contains only float definitions; query algorithms are entirely unimplemented.
