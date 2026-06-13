# Bunny — BEARING

This signpost summarizes short-term priorities, recent ships, and technical debt. It does not replace the backlog, design docs, or roadmap milestones.

## Where are we going?

**Current Priority**: v0.4.0 / Pre-GP2 — Completion Integrity Gate.

* **Active Slice**: Slice A — Fact-check completed roadmap claims and publish
  honest documents.
* **Next Branch**: `roadmap/pre-gp2-truth-gate`.
* **Blocked Work**: v0.4.0 / Goalpost 2 — File Format Adapters
  (`bunny-codec`) must wait until every outstanding completed-claim acceptance
  criterion is implemented, tested, and documented.

## What just shipped?

* **Compressed Mesh Layouts** (Completed Goalpost v0.4.0-GP1, 2026-06-13):
  Implemented 16-bit integer coordinate quantization mapping for 3D vertices, memory-stable `IndexBufferLayout::Width16` / `IndexBufferLayout::Width32` layouts backed by `Triangle16` / `Triangle32` faces with vertex index validation, and zero-allocation cryptographic SHA-256 content-addressable asset hashing for mesh verification inside `bunny-mesh`.
* **WASM Headless Test Runner** (Completed Goalpost v0.1.1-GP3, 2026-06-13):
  Configured Node.js headless WebAssembly unit testing via `wasm-pack test` for all core libraries (`bunny-num`, `bunny-linalg`, `bunny-geom`, `bunny-query`, `bunny-broadphase`) using `wasm-bindgen-test(unsupported = test)` fallback for native host compilation. Added automated WASM check and WASM test jobs in GitHub Actions CI suite.
* **Broadphase Sweep-and-Prune Solver** (Completed Goalpost v0.3.0-GP2, 2026-06-13):
  Implemented a zero-allocation, multi-axis Sweep-and-Prune broadphase overlap query solver with stable lexicographical index sorting. Decomposed broadphase crate into modularized submodules (`bvh`, `sweep_and_prune`, `traversal`, `utils`) to strictly comply with the 300-line file limit.
* **Stable BVH Tree** (Completed Goalpost v0.3.0-GP1, 2026-06-13):
  Implemented a flat array-backed bounding volume hierarchy (BVH) with Surface Area Heuristic (SAH) construction and stack-based traversal solvers.
* **Geometry Intersection and Closest Point Queries** (Completed Release v0.2.0, 2026-06-12):
  Shipped `FixedRay3`, `FixedAabb3`, `FixedSphere3` shapes, ray-sphere/ray-AABB/ray-triangle intersection solvers, and point-to-triangle/segment-to-segment/AABB-to-sphere closest point solvers.
* **Compiler Directives and Numeric Safeguards** (Completed Release v0.1.1, 2026-06-12):
  Shipped directive-driven scalar mapping, Checked Division math guards, and vector saturation boundary verification suites.

## What feels wrong?

* **Overstated Completion Claims**:
  Several completed roadmap entries have acceptance criteria that are stronger
  than the current source/test evidence. GP2 is blocked until these are
  fact-checked and finished off in implementation.
* **Missing Matrix and Quaternion Math**:
  `bunny-linalg` lacks matrix and quaternion profiles (`FixedMat3`, `FixedMat4`, `FixedQuat`), which will be needed for transformation queries.
