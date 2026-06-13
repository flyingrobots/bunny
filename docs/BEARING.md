# Bunny — BEARING

This signpost summarizes short-term priorities, recent ships, and technical debt. It does not replace the backlog, design docs, or roadmap milestones.

## Where are we going?

**Current Priority**: v0.4.0 / Goalpost 2 — File Format Adapters.

* **Pre-GP2 Gate**: Completed. Every outstanding completed-claim acceptance
  criterion has implementation, test, and documentation evidence.
* **Next Branch**: `roadmap/pre-gp2-truth-gate`.
* **Next Work**: Start v0.4.0 / Goalpost 2 — File Format Adapters
  (`bunny-codec`) after this gate branch lands.

## What just shipped?

* **Compressed Mesh Layouts** (Completed Goalpost v0.4.0-GP1, 2026-06-13):
  Implemented 16-bit integer coordinate quantization mapping for 3D vertices, memory-stable `IndexBufferLayout::Width16` / `IndexBufferLayout::Width32` layouts backed by `Triangle16` / `Triangle32` faces with vertex index validation, and zero-allocation cryptographic SHA-256 content-addressable asset hashing for mesh verification inside `bunny-mesh`.
* **WASM Headless Test Runner** (Completed Goalpost v0.1.1-GP3, 2026-06-13):
  Configured Node.js headless WebAssembly unit testing via `wasm-pack test` for
  every WASM-compatible library crate (`bunny-num`, `bunny-linalg`,
  `bunny-geom`, `bunny-contract`, `bunny-query`, `bunny-broadphase`, and
  `bunny-mesh`) using `wasm-bindgen-test(unsupported = test)` fallback for
  native host compilation. Host-side binary/tooling crates (`bunny-wesley` and
  `xtask`) are intentionally covered by native workspace tests instead. Added
  automated WASM check and WASM test jobs in GitHub Actions CI suite.
* **Broadphase Sweep-and-Prune Solver** (Completed Goalpost v0.3.0-GP2, 2026-06-13):
  Implemented a zero-allocation, multi-axis Sweep-and-Prune broadphase overlap query solver with stable lexicographical index sorting. Decomposed broadphase crate into modularized submodules (`bvh`, `sweep_and_prune`, `traversal`, `utils`) to strictly comply with the 300-line file limit.
* **Stable BVH Tree** (Completed Goalpost v0.3.0-GP1, 2026-06-13):
  Implemented a flat array-backed bounding volume hierarchy (BVH) with Surface
  Area Heuristic (SAH) construction and stack-based traversal solvers. The BVH
  builder and traversal paths now use checked buffer/stack access, reject
  malformed inputs without panics, and have a native counting-allocator witness
  for zero heap allocation on the caller-owned buffer API surface.
* **Geometry Intersection and Closest Point Queries** (Completed Release v0.2.0, 2026-06-12):
  Shipped `FixedRay3`, `FixedAabb3`, `FixedSphere3` shapes, ray-sphere/ray-AABB/ray-triangle intersection solvers, and point-to-triangle/segment-to-segment/AABB-to-sphere closest point solvers.
* **Compiler Directives and Numeric Safeguards** (Completed Release v0.1.1, 2026-06-12):
  Shipped directive-driven scalar mapping, Checked Division math guards, and vector saturation boundary verification suites.

## What feels wrong?

* **Pre-GP2 Audit Debt**:
  The dishonest completed-claim labels have been fact-checked and finished off.
  Keep future completed labels tied to implementation, test, CI, and document
  evidence.
* **Missing Matrix and Quaternion Math**:
  `bunny-linalg` lacks matrix and quaternion profiles (`FixedMat3`, `FixedMat4`, `FixedQuat`), which will be needed for transformation queries.
