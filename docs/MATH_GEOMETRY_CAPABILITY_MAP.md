# Bunny Math and Geometry Capability Map

This document defines Bunny's durable math and geometry capability boundary. It
is written for planning and orientation: what Bunny owns, what it refuses to
own, what already exists, and what should be built next.

This is not a release checklist, pull request ledger, or CI status page.
`ROADMAP.md` records versioned delivery. GitHub Issues track executable backlog
items. Source code, tests, fixtures, and crate docs remain the final evidence
for what the repository actually implements.

## Reader's Map

Use this page when you need the shape of the math and geometry stack. Use the
linked current-truth documents when you need a binding contract for a specific
concept.

| Need | Start here |
| --- | --- |
| Project boundary | [Ownership Boundary](#ownership-boundary) |
| Hard invariants | [Non-Negotiables](#non-negotiables) |
| Layer inventory | [Capability Layers](#capability-layers) |
| Build sequence | [Build Order](#build-order) |
| Coordinate law | `docs/topics/coordinate-law/` |
| Numeric law | `docs/NUMERIC_CONSTITUTION.md` |
| Version plan | `ROADMAP.md` |
| Completed evidence | `docs/goalposts/` |

## Ownership Boundary

Bunny owns deterministic, reusable primitives that downstream projects can use
as a mathematical substrate. The library should produce stable numbers,
records, buffers, and query results. It should not decide how an application
simulates, renders, edits, stores, or presents those results.

Bunny owns:

- fixed-point numeric law, rounding, checked ingress, and golden vectors
- linear algebra primitives, coordinate transforms, and orientation math
- geometry shapes, predicates, degeneracy policy, and bounds utilities
- ray, closest-point, overlap, swept, and contact-query algorithms
- broadphase and spatial acceleration structures with stable traversal order
- mesh layouts, topology helpers, quantization, hashing, and codecs
- visibility, occlusion, frustum, and ray-tracing primitive intersection math
- optics and lighting math that can be evaluated deterministically
- validation fixtures, property tests, native/WASM parity, and benchmark gates
- optional acceleration backends only when they prove scalar parity

Bunny does not own:

- physics world stepping, force integration, sleep islands, or gameplay policy
- renderer backends, shader systems, material authoring, or texture pipelines
- scene graphs, entity-component frameworks, editors, or asset browsers
- Echo causality, Geordi receipt semantics, or jedit user-interface workflows
- nondeterministic fast-math behavior as canonical output

## Non-Negotiables

These rules are the floor for future math and geometry work. A feature that
violates them belongs outside Bunny or needs a quarantined boundary with an
explicit deterministic contract.

| Rule | Proof |
| --- | --- |
| Fixed-point is canonical. | Raw-value tests |
| Float ingress is fallible. | Rejection tests |
| Float egress is diagnostic. | Fixed-value assertions |
| Invalid input is explicit. | Error-kind tests |
| Scalar is the reference path. | Reference fixtures |
| SIMD is optional acceleration. | Scalar parity tests |
| Allocation claims are proved. | Allocation witnesses |
| Golden vectors include ugly cases. | Edge-case fixtures |

Canonical math uses Bunny-defined fixed-point values unless an API explicitly
says it is a boundary format. Floats are acceptable at ingress, egress,
diagnostic, and adapter boundaries, but they do not define equality truth inside
core algorithms.

Invalid geometry, malformed payloads, degenerate inputs, overflow, and division
by zero must return explicit `Result` or `Option` outcomes. Panic paths,
ambient state, unordered iteration, and target-specific fast math are not
acceptable sources of canonical behavior.

## Capability Layers

Bunny's stack should grow bottom-up. Lower layers define the laws and primitive
contracts that higher layers consume. Higher layers may depend on lower layers;
lower layers should not absorb renderer, editor, physics-engine, or application
policy from their consumers.

| Layer | Current | Next |
| --- | --- | --- |
| Numeric law | Q32.32 scalar math | Saturation audit |
| Linear algebra | Vectors, unit vectors | Matrices and transforms |
| Geometry primitives | Rays, AABBs, spheres | Degeneracy and shapes |
| Collision and contact | Ray and closest queries | Narrowphase coverage |
| Acceleration | BVH, sweep-and-prune | Dynamic structures |
| Mesh and codecs | Quantized mesh, PLY/OBJ, decoder | Topology and encoder |
| Visibility and optics | Ray and BVH ingredients | Camera and visibility math |
| SIMD and performance | Scalar reference | Parity and benchmarks |
| Consumer confidence | Docs, witnesses, gates | Examples and fixtures |

### Numeric Law

`bunny-num` owns the canonical Q32.32 scalar profile. Its job is to make the
raw numeric truth boring: deterministic rounding, explicit construction policy,
fallible float ingress, documented overflow behavior, and raw-value equality.
The detailed arithmetic contract lives in `docs/NUMERIC_CONSTITUTION.md`.

Current surface:

- `FixedQ32_32`
- raw round-trip and golden-vector tests
- deterministic arithmetic operators
- fallible float ingress through `try_from_f32`
- compatibility egress and diagnostic float helpers

Open work:

| Capability | Issues |
| --- | --- |
| Saturating arithmetic audit | #114 |
| Property-based numeric tests | #129 |

### Linear Algebra

`bunny-linalg` owns deterministic vector math and the frame algebra that will
support projection, orientation, and camera work. Coordinate handedness and unit
policy are now defined in `docs/topics/coordinate-law/`; current matrix and
affine transform APIs respect that law.

Current surface:

- `FixedVec2` and `FixedVec3`
- dot and cross products
- length and normalization
- fixed unit-vector wrappers
- `FixedMat2`, `FixedMat3`, and `FixedMat4`
- `FixedAffine2` and `FixedAffine3`
- coordinate-law convention tests

Open work:

| Capability | Issues |
| --- | --- |
| Quaternion rotations | #109 |
| Angle and trigonometry policy | #110 |
| Interpolation and remap helpers | #111 |
| Projection and viewport mapping | #150 |
| Curves and splines | #152 |

### Geometry Primitives

Geometry primitives define the vocabulary for later query, collision,
visibility, and mesh operations. Bunny already has the first fixed shape set,
but the shape library needs richer coverage and a clear degeneracy law before
more algorithms build on top of it.

Current surface:

- `FixedRay3`
- `FixedAabb3`
- `FixedSphere3`
- validated fixed/float boundary conversions
- ray and closest-point query inputs

Open work:

| Capability | Issues |
| --- | --- |
| Shape library expansion | #115, #117, #146, #149 |
| Degeneracy policy | #116 |
| Robust predicates | #145 |
| Clipping and half-spaces | #162 |
| Bounds propagation and merge | #119, #151 |

### Collision and Contact

Collision work should provide deterministic query primitives and contact
records, not a physics engine. Bunny can own primitive-pair truth, support
mapping, manifolds, and swept queries while leaving stepping, forces, sleeping,
and gameplay policy to downstream systems.

Current surface:

- ray/sphere, ray/AABB, and ray/triangle queries
- point/triangle, segment/segment, and AABB/sphere closest queries
- sweep-and-prune broadphase pairs
- BVH build and traversal primitives

Open work:

| Capability | Issues |
| --- | --- |
| Narrowphase shape coverage | #117 |
| SAT helpers | #147 |
| Support mapping and GJK/EPA | #139 |
| Contact manifolds | #118 |
| Swept queries | #148 |
| Collision architecture | #157 |

### Acceleration

Acceleration structures make existing query semantics scale. They must preserve
stable output order and deterministic failure behavior, because consumers will
often treat broadphase and visibility outputs as canonical records.

Current surface:

- static BVH construction
- BVH traversal
- sweep-and-prune active pairs
- stable traversal and pair-order tests
- zero-allocation witnesses for claimed paths

Open work:

| Capability | Issues |
| --- | --- |
| Dynamic and refit APIs | #120 |
| Spatial hash or uniform grid | #121 |
| Mesh BVH adapters | #123 |
| Occlusion structures | #158 |

### Mesh and Codecs

Mesh work owns deterministic asset data structures and bounded interchange
profiles. Bunny should support compact mesh buffers, topology helpers,
canonical encoders/decoders, corruption detection, and replayable malformed
input tests without becoming a full asset pipeline.

Current surface:

- quantized vertex buffers
- stable triangle index buffers
- mesh content hashing
- zero-copy PLY and OBJ parsers
- Bunny compressed mesh decoder
- codec fixtures and allocation witnesses

Open work:

| Capability | Issues |
| --- | --- |
| Topology and adjacency helpers | #122 |
| Mass properties | #163 |
| Compressed mesh encoder | #124 |
| Checksums and corruption detection | #125 |
| glTF and STL adapters | #126, #127 |
| Codec fuzzing | #128 |

### Visibility and Optics

Visibility and optics should expose deterministic math records that renderers,
editors, and simulators can consume. Bunny should not become a renderer, but it
can own the camera, ray generation, visibility, hit ordering, and lighting math
that those systems need to agree on.

Current surface:

- ray primitives
- ray intersection queries
- BVH traversal ingredients
- coordinate law reservations for future projection work

Open work:

| Capability | Issues |
| --- | --- |
| Occlusion and visibility queries | #158 |
| Ray tracing hit suites | #159 |
| Camera and ray generation | #138 |
| Lighting and BRDF inputs | #160 |

### SIMD and Performance

Performance work is welcome only after the scalar path is stable enough to act
as a reference. SIMD and target-specific implementations must be optional,
feature-gated or target-detected, and mechanically compared against the same
scalar fixtures.

Current surface:

- scalar fixed-point reference implementation
- Code Dojo quality gates
- native and WASM test gates
- cargo-deny dependency policy

Open work:

| Capability | Issues |
| --- | --- |
| Deterministic SIMD backends | #161 |
| Benchmark suites | #130 |
| `no_std` and allocation review | #156 |

SIMD rules:

- Keep scalar as the default and reference path.
- Prefer integer SIMD over floating-point SIMD for canonical math.
- Never allow target `fast-math` behavior to define Bunny truth.
- Use deterministic scalar fallback.
- Compare SIMD and scalar results against the same golden corpora.
- Avoid lane reductions whose order changes externally visible results.

### Consumer Confidence

Consumer confidence work makes the library easier to adopt and harder to
misuse. It includes examples, fixture crates, generated-contract parity,
rustdoc, docs.rs readiness, and validation material that downstream projects can
reuse.

Current surface:

- crate READMEs
- goalpost evidence documents
- generated contract witnesses
- Code Dojo gates
- release and testing docs

Open work:

| Capability | Issues |
| --- | --- |
| Public examples | #131 |
| TypeScript parity tests | #132 |
| Stable fixture crate | #137 |
| Rustdoc and docs.rs polish | #155 |

## Build Order

The missing stack should land bottom-up. Coordinate law and numeric
preconditions come first because every later transform, geometry query, camera,
collision, visibility, and optics API depends on those choices. Collision and
visibility should not be built on ambiguous shape, transform, or degeneracy
semantics.

1. **Coordinate law and math foundations**: lock spaces, units, angles,
   orientation, matrices, transforms, and projection basics. See #164, #107,
   #108, #109, #110.
2. **Geometry law and primitive coverage**: define degeneracy, robust
   predicates, richer shapes, clipping, bounds, and 2D coverage. See #116,
   #145, #115, #162, #151, #149.
3. **Query and collision coverage**: add narrowphase, SAT, GJK/EPA, manifolds,
   swept queries, and collision architecture. See #117, #147, #139, #118,
   #148, #157.
4. **Acceleration and visibility scaling**: add dynamic broadphase, grids, mesh
   BVHs, occlusion, and ray tracing hit suites. See #120, #121, #123, #158,
   #159.
5. **Higher math consumers and performance**: add camera rays, optics, SIMD,
   benchmarks, and codec encoders. See #138, #160, #161, #130, #124, #125.
6. **Consumer confidence**: add examples, fixtures, parity tests, fuzzing,
   rustdoc, and docs.rs polish. See #131, #137, #132, #128, #155.

## Documentation Rule

Update this file when Bunny changes what it owns, what it refuses to own, what
order foundational math and geometry work should follow, or which current-truth
document owns a capability.

Do not use this file for branch status, pull request state, CI snapshots, or
per-slice evidence. Those belong in GitHub Issues, goalpost docs,
`CHANGELOG.md`, and repository receipts.
