# Bunny Math and Geometry Capability Map

This document defines Bunny's durable math and geometry capability boundary.

It is a human-readable planning map: what Bunny owns, what it refuses to own,
what already exists, and what should be built next. It is not a release
checklist, pull request ledger, or current CI status page.

| Question | Source of truth |
| --- | --- |
| What does Bunny own as a math and geometry library? | This document. |
| What order should future capability work follow? | This document plus `ROADMAP.md`. |
| What version will deliver a slice? | `ROADMAP.md` and GitHub milestones. |
| What is currently implemented? | Source code, tests, fixtures, and crate docs. |
| What work is executable backlog? | GitHub Issues. |
| What changed in a release? | `CHANGELOG.md`. |
| What did a completed goalpost prove? | `docs/goalposts/`. |

## Reader's Map

| If you need to know... | Start here |
| --- | --- |
| The project boundary | [Ownership Boundary](#ownership-boundary) |
| Hard invariants that future work must preserve | [Non-Negotiables](#non-negotiables) |
| What exists and what is missing by layer | [Capability Layers](#capability-layers) |
| The recommended implementation order | [Build Order](#build-order) |
| Current coordinate-space law | `docs/topics/coordinate-law/` |
| Current numeric arithmetic law | `docs/NUMERIC_CONSTITUTION.md` |

## Ownership Boundary

Bunny owns deterministic, reusable primitives that downstream projects can use
as a mathematical substrate.

| Bunny owns | Examples |
| --- | --- |
| Fixed-point numeric law | Q32.32 representation, rounding, checked ingress, golden vectors. |
| Linear algebra | Vectors, future matrices, transforms, orientation, angle, interpolation. |
| Geometry primitives | Rays, bounds, spheres, future planes, OBBs, capsules, frusta, 2D shapes. |
| Geometry queries | Ray hits, closest points, overlaps, swept queries, contacts. |
| Spatial acceleration | BVH, sweep-and-prune, future dynamic broadphase, grids, mesh BVHs. |
| Mesh foundations | Quantized vertices, triangle buffers, topology helpers, hashing, codecs. |
| Visibility and ray tracing math | Occlusion, visibility records, deterministic ray-hit suites. |
| Optics and lighting math | Camera rays, reflection/refraction helpers, deterministic BRDF inputs. |
| Validation and confidence | Golden vectors, fixtures, property tests, native/WASM parity, benchmarks. |
| Optional acceleration | SIMD or target backends only when they prove scalar parity. |

Bunny deliberately does not own application or engine policy.

| Bunny does not own | Why |
| --- | --- |
| Physics world stepping | Bunny may provide collision/contact queries, but not simulation policy, force integration, sleep islands, or gameplay behavior. |
| Renderer backends | Bunny may provide camera, optics, and visibility math, but not shaders, materials, texture pipelines, or draw submission. |
| Scene graphs and ECS frameworks | Bunny provides reusable primitives; downstream apps own object models and lifecycle. |
| Editors and asset browsers | `jedit` and other consumers own workflow and UI behavior. |
| Echo and Geordi domain policy | Bunny results may become downstream facts or receipts, but Bunny does not own those systems' semantics. |
| Nondeterministic fast math | Target-specific float shortcuts cannot define canonical Bunny truth. |

The short version: Bunny owns deterministic math outputs and records. Consumers
own what those outputs mean inside an application.

## Non-Negotiables

| Rule | Meaning | Evidence required |
| --- | --- | --- |
| Fixed-point is canonical | Core math uses Bunny-defined fixed-point values unless an API explicitly says it is a boundary format. | Numeric law docs, raw-value tests, and golden vectors. |
| Float ingress is fallible | Non-finite and out-of-range floats must be rejected before canonicalization. | Rejection tests for boundary APIs. |
| Float egress is diagnostic | `to_f32`-style output is for display, debugging, or external adapters, not equality truth. | Tests assert raw values or structured fixed values. |
| Invalid input is explicit | Invalid geometry, malformed payloads, degenerates, overflow, and division by zero return `Result` or `Option`. | Stable error-kind or `None` tests. |
| Scalar is reference | Scalar fixed-point implementations define canonical output. | Optimized paths compare against scalar fixtures. |
| SIMD is optional | SIMD, intrinsics, and future GPU kernels must prove byte-for-byte or raw-value parity. | Scalar parity tests across the same corpus. |
| Claimed allocation bounds are proved | Zero-allocation, bounded-memory, and stable-order claims need tests. | Allocation witnesses, bounded-stack tests, or stable-order fixtures. |
| Golden vectors include ugly cases | Pleasant examples are not enough. | Edge, boundary, malformed, and degeneracy fixtures. |

## Capability Layers

This table gives the quick version. The sections below explain each layer.

| Layer | Current surface | Main pressure |
| --- | --- | --- |
| [Numeric Law](#numeric-law) | Q32.32 fixed-point scalar math. | Saturation audit and property tests. |
| [Linear Algebra](#linear-algebra) | 2D/3D fixed vectors and unit vectors. | Matrices, transforms, orientation, angles, curves. |
| [Geometry Primitives](#geometry-primitives) | Rays, AABBs, spheres, fixed/float boundaries. | Degeneracy law, predicates, richer shapes, clipping, 2D suite. |
| [Collision and Contact](#collision-and-contact) | First ray and closest-point solvers; broadphase primitives. | Narrowphase, SAT, GJK/EPA, manifolds, swept queries. |
| [Acceleration](#acceleration) | Static BVH and sweep-and-prune. | Dynamic updates, grids, mesh BVH, occlusion structures. |
| [Mesh and Codecs](#mesh-and-codecs) | Quantized mesh buffers, hashing, PLY/OBJ, compressed decoder. | Topology, mass properties, encoder, checksums, adapters, fuzzing. |
| [Visibility and Optics](#visibility-and-optics) | Low-level ray and BVH ingredients. | Visibility records, ray tracing hit suites, camera and lighting math. |
| [SIMD and Performance](#simd-and-performance) | Scalar reference plus Code Dojo gates. | Deterministic SIMD, benchmarks, allocation/no-std review. |
| [Consumer Confidence](#consumer-confidence) | Crate READMEs, goalpost docs, generated witnesses. | Examples, fixture crate, TypeScript parity, rustdoc/docs.rs polish. |

### Numeric Law

| Current contract | Evidence |
| --- | --- |
| `bunny-num` exposes canonical Q32.32 fixed-point math. | `FixedQ32_32` and numeric tests. |
| Arithmetic uses deterministic rounding and checked or documented overflow behavior. | `docs/NUMERIC_CONSTITUTION.md`. |
| Float ingress has validated fallible conversion APIs. | `try_from_f32` and conversion rejection tests. |
| Raw values are the equality and serialization truth. | Raw golden-vector tests. |

| Missing or pressured capability | Why it matters | Issue |
| --- | --- | --- |
| Audit saturating arithmetic. | Saturation is deterministic, but it can turn invalid geometry into plausible garbage. | #114 |
| Add property-based numeric tests. | Generated coverage catches invariant failures beyond enumerated examples. | #129 |

### Linear Algebra

| Current contract | Evidence |
| --- | --- |
| `bunny-linalg` provides deterministic 2D and 3D fixed vectors. | `FixedVec2`, `FixedVec3`, vector tests. |
| Dot, cross, length, normalization, and unit-vector wrappers exist. | Linalg tests and unit-vector tests. |
| Coordinate-space and handedness law is documented. | `docs/topics/coordinate-law/`. |

| Missing capability | Why it matters | Issue |
| --- | --- | --- |
| Matrix types and multiplication policy. | Transforms, projection, and frame conversion need a stable algebra. | #107 |
| Affine transforms for points, vectors, normals, and frames. | Prevents accidental mixing of value kinds and spaces. | #108 |
| Quaternion or orientation representation. | Needed for stable rotations and camera/collision work. | #109 |
| Canonical angle and trigonometry profile. | Rotation, curves, optics, and interpolation need deterministic angle semantics. | #110 |
| Interpolation, easing, clamp, min/max, and remap primitives. | Consumer code needs shared deterministic utility math. | #111 |
| Projection, unprojection, and viewport mapping. | Camera and picking workflows need one coordinate convention. | #150 |
| Curves, Bezier, and spline primitives. | Editors and geometry tools need deterministic parametric math. | #152 |

### Geometry Primitives

| Current contract | Evidence |
| --- | --- |
| `bunny-geom` defines rays, AABBs, spheres, fixed variants, and validated ingress. | Geometry constructors and rejection tests. |
| `bunny-query` implements initial ray and closest-point solvers. | Query tests and raw-output determinism corpora. |

| Missing capability | Why it matters | Issue |
| --- | --- | --- |
| Shape library beyond rays, AABBs, and spheres. | Collision, visibility, and editor tools need common primitives. | #115, #117, #146, #149 |
| Degeneracy policy for every shape and query. | Zero length, zero area, coplanarity, and contact cases must be intentional. | #116 |
| Robust fixed geometric predicates. | Predicate truth must not depend on float tolerances or platform behavior. | #145 |
| Clipping, half-space, and constructive primitive operations. | Geometry composition needs shared deterministic tools. | #162 |
| Bounds propagation and merge utilities. | Transforms, acceleration, culling, and mesh workflows need stable bounds. | #119, #151 |

### Collision and Contact

| Current contract | Evidence |
| --- | --- |
| Ray/sphere, ray/AABB, ray/triangle query families exist. | Query tests and ray determinism corpus. |
| Point/triangle, segment/segment, and AABB/sphere closest-point families exist. | Closest-point raw tests. |
| Sweep-and-prune and BVH broadphase primitives exist. | Broadphase tests and allocation witnesses. |

| Missing capability | Why it matters | Issue |
| --- | --- | --- |
| Narrowphase overlap tests for the supported shape set. | Broadphase candidates need deterministic primitive-pair truth. | #117 |
| SAT helpers for convex shapes. | Convex overlap needs reusable stable separating-axis logic. | #147 |
| Support mapping plus GJK/EPA. | Convex distance and penetration need deterministic algorithms. | #139 |
| Contact manifolds with stable point ordering. | Downstream physics can consume contacts without Bunny owning stepping policy. | #118 |
| Swept queries and time-of-impact calculations. | Continuous collision needs deterministic motion queries. | #148 |
| Collision detection architecture. | Broadphase, narrowphase, filtering, and contact output need a composable contract. | #157 |

### Acceleration

| Current contract | Evidence |
| --- | --- |
| `bunny-broadphase` provides BVH construction and traversal. | BVH tests and zero-allocation witnesses. |
| Sweep-and-prune overlap generation has deterministic ordering. | Sweep pair golden-vector tests. |

| Missing capability | Why it matters | Issue |
| --- | --- | --- |
| Dynamic, refit, and incremental broadphase updates. | Large scenes need update-friendly acceleration without output drift. | #120 |
| Spatial hash or uniform grid solver. | Dense local worlds need alternatives to tree-based broadphase. | #121 |
| Mesh BVH and triangle-level query adapters. | Mesh ray and closest-point work need acceleration over triangle buffers. | #123 |
| Occlusion and visibility query structures. | Renderers and editors need deterministic visibility culling primitives. | #158 |

### Mesh and Codecs

| Current contract | Evidence |
| --- | --- |
| `bunny-mesh` provides quantized vertices, triangle buffers, and content hashing. | Mesh tests and hash golden vectors. |
| `bunny-codec` parses PLY/OBJ and decodes the Bunny compressed mesh profile. | Codec tests, fixtures, and allocation witnesses. |

| Missing capability | Why it matters | Issue |
| --- | --- | --- |
| Mesh topology and adjacency helpers. | Geometry queries and validation need connectivity information. | #122 |
| Deterministic mass properties for primitives and meshes. | Collision and physics consumers need stable shape summaries. | #163 |
| Canonical compressed mesh encoder. | Decoder-only support is not enough for round-trip asset workflows. | #124 |
| Stronger checksums and corruption detection. | Mesh payload failures should be explicit and stable. | #125 |
| glTF and STL adapters with bounded semantics. | Common interchange formats need clear, limited Bunny contracts. | #126, #127 |
| Codec and malformed-input fuzzing harnesses. | Parsers need replayable pressure beyond hand-written fixtures. | #128 |

### Visibility and Optics

| Current contract | Evidence |
| --- | --- |
| Bunny owns the low-level ray and BVH ingredients for future visibility work. | `bunny-query` and `bunny-broadphase`. |
| Bunny does not own renderer policy. | Scope contract in this document. |

| Missing capability | Why it matters | Issue |
| --- | --- | --- |
| Occlusion tests, visibility masks, and visibility batching. | Renderers and editors need stable visibility records. | #158 |
| Ray tracing hit records, hit ordering, and primitive intersection suites. | Consumers need deterministic hit semantics, not just yes/no intersections. | #159 |
| Camera and ray-generation math. | Picking, projection, and rendering need shared deterministic camera primitives. | #138 |
| Lighting vectors, reflection/refraction, attenuation, and BRDF inputs. | Optics math can be deterministic without Bunny becoming a renderer. | #160 |

### SIMD and Performance

| Current contract | Evidence |
| --- | --- |
| Scalar fixed-point code is the canonical implementation. | Numeric law and existing scalar tests. |
| Code Dojo, native tests, WASM tests, and cargo-deny are the active quality gates. | `docs/CODE_DOJO.md` and CI workflow. |

| Missing capability | Why it matters | Issue |
| --- | --- | --- |
| Optional SIMD backends with scalar parity gates. | Speedups must not redefine canonical output. | #161 |
| Benchmark suites for numeric, query, broadphase, mesh, and codec hot paths. | Performance claims need repeatable evidence. | #130 |
| `no_std` feasibility and allocation-boundary review. | Some consumers need constrained runtime targets. | #156 |

SIMD policy:

| Rule | Reason |
| --- | --- |
| Scalar remains the default and reference path. | There must always be one boring source of truth. |
| Prefer integer SIMD over floating-point SIMD for canonical math. | Integer lanes are easier to make bit-stable. |
| Never allow target `fast-math` to define Bunny truth. | Fast-math can change results across targets. |
| Use feature flags or target detection with deterministic scalar fallback. | Acceleration should be optional and safe to disable. |
| Test SIMD and scalar results against the same golden corpora. | Parity must be mechanical, not assumed. |
| Avoid lane reductions whose order changes visible results. | Reduction order can create subtle nondeterminism. |

### Consumer Confidence

| Current contract | Evidence |
| --- | --- |
| Code Dojo enforces formatting, linting, AST policy, tests, dependency policy, receipts, and WASM checks. | `docs/CODE_DOJO.md`. |
| Crate READMEs and goalpost documents describe completed feature contracts. | Crate docs and `docs/goalposts/`. |

| Missing capability | Why it matters | Issue |
| --- | --- | --- |
| Public examples without shortcuts. | Users need correct fallible API patterns. | #131 |
| TypeScript and generated-contract parity tests. | DTO consumers need Rust and TypeScript surfaces to agree. | #132 |
| Stable fixture crate for downstream consumers. | External projects need reusable deterministic examples. | #137 |
| Docs.rs and rustdoc polish. | Public crates need discoverable API docs. | #155 |

## Build Order

The missing stack should land bottom-up. Do not build collision, visibility, or
optics on ambiguous coordinate or transform semantics.

| Order | Build theme | Why it comes here | Issues |
| --- | --- | --- | --- |
| 1 | Coordinate law and math foundations | Every later transform, query, and camera contract depends on spaces, units, angles, and orientation. | #164, #107, #108, #109, #110 |
| 2 | Geometry law and primitive coverage | Robust predicates, degeneracy, clipping, bounds, and 2D shapes define what later queries mean. | #116, #145, #115, #162, #151, #149 |
| 3 | Query and collision coverage | Narrowphase, SAT, GJK/EPA, manifolds, swept queries, and collision architecture need stable geometry law. | #117, #147, #139, #118, #148, #157 |
| 4 | Acceleration and visibility scaling | Dynamic broadphase, grids, mesh BVHs, occlusion, and ray tracing need stable query semantics first. | #120, #121, #123, #158, #159 |
| 5 | Higher math consumers and performance | Camera rays, optics, SIMD, benchmarks, and codec encoders build on stable math/query layers. | #138, #160, #161, #130, #124, #125 |
| 6 | Consumer confidence | Examples, fixtures, parity tests, fuzzing, rustdoc, and docs.rs polish make the surface easier to trust. | #131, #137, #132, #128, #155 |

## Documentation Rule

Update this file when Bunny changes:

- what it owns
- what it refuses to own
- what order foundational math and geometry work should follow
- which current-truth document owns a capability

Do not use this file for branch status, pull request state, CI snapshots, or
per-slice evidence. Those belong in GitHub Issues, goalpost docs,
`CHANGELOG.md`, and repository receipts.
