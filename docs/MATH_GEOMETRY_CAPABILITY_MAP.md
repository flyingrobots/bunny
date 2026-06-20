# Bunny Math and Geometry Capability Map

This document defines the durable capability boundary for Bunny's deterministic
math and geometry stack.

It is not a release checklist, pull request ledger, or current CI status page.
`ROADMAP.md` records versioned delivery. GitHub Issues track executable backlog
items. Source code, tests, and checked-in fixtures remain the final evidence for
what the repository actually implements.

## Scope Contract

Bunny owns deterministic, reusable primitives that downstream projects can trust
as a mathematical substrate:

- Fixed-point numeric law, rounding, checked ingress, and golden vectors.
- Linear algebra primitives, coordinate transforms, and orientation math.
- Geometry shapes, robust predicates, degeneracy policy, and bounds utilities.
- Ray, closest-point, overlap, swept, and contact-query algorithms.
- Broadphase and spatial acceleration structures with stable traversal order.
- Mesh layouts, topology helpers, quantization, hashing, and codecs.
- Visibility, occlusion, frustum, and ray-tracing primitive intersection math.
- Optics and lighting math that can be evaluated deterministically.
- Validation fixtures, property tests, native/WASM parity, and benchmark gates.
- Optional acceleration backends only when they prove scalar parity.

Bunny does not own higher-level application behavior:

- Physics world stepping, force integration, sleep islands, or gameplay policy.
- Renderer backends, shader systems, material authoring, or texture pipelines.
- Scene graphs, entity-component frameworks, editors, or asset browsers.
- Echo causality, Geordi receipt semantics, or jedit user-interface workflows.
- Nondeterministic fast-math behavior as canonical output.

Those projects may consume Bunny primitives, but Bunny should not absorb their
domain policy.

## Non-Negotiables

- Canonical math is fixed-point unless a boundary API explicitly says otherwise.
- Floating-point ingress is fallible and must reject non-finite or
  out-of-range inputs before canonicalization.
- Floating-point egress is diagnostic or presentation oriented, not equality
  truth.
- Public APIs must return explicit errors or `Option` for invalid geometry,
  malformed payloads, degenerate inputs, overflow, and division by zero.
- The scalar implementation is the reference implementation.
- SIMD, target intrinsics, and future GPU kernels are optional accelerators.
  They must prove byte-for-byte or raw-value parity with the scalar path.
- Algorithms that claim zero allocation, stable ordering, or bounded memory must
  carry tests that measure or directly prove those claims.
- Golden vectors must exercise edge cases, not just pleasant examples.

## Capability Layers

### Numeric Law

Baseline surface:

- `bunny-num` exposes canonical Q32.32 fixed-point math.
- Arithmetic uses deterministic rounding and checked or documented overflow
  behavior.
- Float ingress is now validated through fallible conversion APIs.
- `docs/NUMERIC_CONSTITUTION.md` defines the arithmetic law.

Backlog pressure:

- Audit saturating arithmetic and decide where checked arithmetic should replace
  saturation. See #114.
- Add property-based numeric tests and generated edge-case corpora. See #129.

### Linear Algebra

Baseline surface:

- `bunny-linalg` provides deterministic 2D and 3D vectors.
- Dot, cross, length, normalization, and unit-vector wrappers exist for the
  current fixed-point shape.

Missing surface:

- Matrix types and multiplication policy. See #107.
- Transform types for points, vectors, normals, and frames. See #108.
- Quaternion or other orientation representation. See #109.
- Deterministic angle and trigonometry profile. See #110.
- Interpolation, easing, and spline-safe blend operations. See #111.
- Projection and camera-space math. See #150.
- Curves, splines, and path primitives. See #152.
- Coordinate-space, handedness, and units policy is defined by
  `docs/topics/coordinate-law/`. See #164.

### Geometry Primitives

Baseline surface:

- `bunny-geom` defines rays, AABBs, spheres, fixed variants, and validated
  ingress.
- `bunny-query` implements the first ray and closest-point solvers.

Missing surface:

- Richer shape library: planes, OBBs, capsules, cylinders, cones, segments,
  triangles, polygons, frusta, and 2D variants. See #115, #117, #146, #149.
- Shared degeneracy policy for zero area, zero length, coplanarity, and
  coincident shapes. See #116.
- Robust geometric predicates with deterministic tolerance-free semantics where
  possible. See #145.
- Clipping, half-space, and constructive primitive operations. See #162.
- Bounds propagation and bounds utility APIs. See #119, #151.

### Collision and Contact Queries

Baseline surface:

- Ray/sphere, ray/AABB, ray/triangle, point/triangle,
  segment/segment, and AABB/sphere query families exist.
- Sweep-and-prune and BVH broadphase primitives exist.

Missing surface:

- Narrowphase overlap tests for the full supported shape set. See #117.
- Separating Axis Theorem helpers for convex shapes. See #147.
- Support mapping plus GJK/EPA for convex distance and penetration. See #139.
- Contact manifolds with stable point ordering. See #118.
- Swept queries and time-of-impact calculations. See #148.
- A deterministic collision-detection architecture that composes broadphase,
  narrowphase, filtering, and contact output without owning simulation policy.
  See #157.

### Broadphase and Spatial Acceleration

Baseline surface:

- `bunny-broadphase` provides BVH construction/traversal and sweep-and-prune
  overlap generation.
- Traversal and active-pair output have deterministic ordering contracts.

Missing surface:

- Dynamic, refit, and incremental broadphase update paths. See #120.
- Spatial hash and uniform grid alternatives for dense local worlds. See #121.
- Mesh BVH construction and triangle-level query adapters. See #123.
- Occlusion and visibility query structures. See #158.

### Mesh and Topology

Baseline surface:

- `bunny-mesh` provides quantized vertex buffers, triangle index buffers, and
  content hashing.
- `bunny-codec` parses PLY/OBJ and decodes the Bunny compressed mesh profile.

Missing surface:

- Mesh topology and adjacency helpers. See #122.
- Deterministic mass properties for primitives and meshes. See #163.
- Canonical encoder for the Bunny compressed mesh profile. See #124.
- Stronger compressed-profile checksums and corruption detection. See #125.
- glTF and STL adapters with explicitly bounded semantics. See #126, #127.
- Fuzzing harnesses for codecs and malformed mesh inputs. See #128.

### Visibility, Ray Tracing, and Optics

Baseline surface:

- Bunny currently owns the low-level ray and BVH pieces needed to build more
  complete visibility queries.

Missing surface:

- Occlusion tests, visibility masks, and deterministic visibility query
  batching. See #158.
- Ray tracing primitive hit records, hit ordering, and intersection suites.
  See #159.
- Camera and ray-generation math. See #138.
- Lighting vectors, reflection/refraction helpers, attenuation, and BRDF math.
  See #160.

These are math primitives, not a renderer. The output should be deterministic
numbers and records that a renderer or simulator can consume.

### SIMD and Performance

Baseline surface:

- Scalar fixed-point code is the canonical implementation.
- Code Dojo, native tests, WASM tests, and cargo-deny are the current quality
  gates.

Missing surface:

- Optional SIMD backends with scalar parity gates. See #161.
- Criterion or equivalent benchmark suites for numeric, query, broadphase,
  mesh, and codec hot paths. See #130.
- `no_std` feasibility and allocation-boundary review. See #156.

SIMD policy:

- Keep scalar code as the default and reference path.
- Prefer integer SIMD over floating-point SIMD for canonical math.
- Never allow target `fast-math` behavior to define Bunny truth.
- Use feature flags or target detection with deterministic scalar fallback.
- Test SIMD and scalar results against the same golden corpora.
- Avoid algorithms whose lane reduction order changes externally visible
  results.

### Validation and Consumer Confidence

Baseline surface:

- Code Dojo enforces source-shape policy, deterministic receipt checks,
  formatting, Clippy, tests, cargo-deny, and WASM checks.
- Crate READMEs and goalpost documents describe completed feature contracts.

Missing surface:

- Examples that show correct public API usage without shortcuts. See #131.
- TypeScript/generated-contract parity tests. See #132.
- Stable fixture crate for downstream consumers. See #137.
- Docs.rs and rustdoc polish for public crates. See #155.

## Recommended Build Order

The missing stack should land bottom-up. Do not build collision or optics on
ambiguous coordinate or transform semantics.

1. Lock coordinate law: spaces, handedness, units, angle policy, and transform
   composition. `docs/topics/coordinate-law/` defines the current law; see
   #164, #107, #108, #109, #110.
2. Harden geometry law: degeneracy, predicates, richer shape types, clipping,
   bounds, and 2D coverage. See #116, #145, #115, #162, #151, #149.
3. Expand query coverage: OBB/capsule/plane tests, SAT, GJK/EPA, manifolds,
   swept queries, and collision architecture. See #117, #147, #139, #118,
   #148, #157.
4. Scale query execution: dynamic broadphase, grids, mesh BVHs, occlusion, and
   ray-tracing hit suites. See #120, #121, #123, #158, #159.
5. Add higher math consumers: camera/ray generation, optics and lighting math,
   deterministic SIMD, benchmarks, and codec encoders. See #138, #160, #161,
   #130, #124, #125.
6. Strengthen consumer confidence: examples, fixtures, TypeScript parity,
   fuzzing, rustdoc, and docs.rs polish. See #131, #137, #132, #128, #155.

## Documentation Rule

Update this file when Bunny changes what it owns, refuses to own, or sequences
as foundational math work. Do not use it for branch status, pull request state,
CI snapshots, or per-slice evidence. Those belong in issues, goalpost docs,
`CHANGELOG.md`, and repository receipts.
