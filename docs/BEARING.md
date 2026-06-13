# Bunny — BEARING

This signpost summarizes short-term priorities, recent ships, and technical debt. It does not replace the backlog, design docs, or roadmap milestones.

## Where are we going?

**Current Priority**: v0.1.1 / Goalpost 3 — Headless WebAssembly Verification (`bunny-infra`).
* **Active Slice**: Slice 3.1 — Configure GitHub Actions to execute the full workspace unit test suite inside a headless Node.js/V8 WASM runner via `wasm-pack test`.
* **Next Branch**: `goalpost/v0.1.1-gp3`.

## What just shipped?

* **Broadphase Sweep-and-Prune Solver** (Completed Goalpost v0.3.0-GP2, 2026-06-13):
  Implemented a zero-allocation, multi-axis Sweep-and-Prune broadphase overlap query solver with stable lexicographical index sorting. Decomposed broadphase crate into modularized submodules (`bvh`, `sweep_and_prune`, `traversal`, `utils`) to strictly comply with the 300-line file limit.
* **Stable BVH Tree** (Completed Goalpost v0.3.0-GP1, 2026-06-13):
  Implemented a flat array-backed bounding volume hierarchy (BVH) with Surface Area Heuristic (SAH) construction and stack-based traversal solvers.
* **Geometry Intersection and Closest Point Queries** (Completed Release v0.2.0, 2026-06-12):
  Shipped `FixedRay3`, `FixedAabb3`, `FixedSphere3` shapes, ray-sphere/ray-AABB/ray-triangle intersection solvers, and point-to-triangle/segment-to-segment/AABB-to-sphere closest point solvers.
* **Compiler Directives and Numeric Safeguards** (Completed Release v0.1.1, 2026-06-12):
  Shipped directive-driven scalar mapping, Checked Division math guards, and vector saturation boundary verification suites.

## What feels wrong?

* **Missing Headless WebAssembly Verification**:
  CI check parses compile target portability but does not run headless tests via `wasm-pack test`.
* **Missing Matrix and Quaternion Math**:
  `bunny-linalg` lacks matrix and quaternion profiles (`FixedMat3`, `FixedMat4`, `FixedQuat`), which will be needed for transformation queries.
