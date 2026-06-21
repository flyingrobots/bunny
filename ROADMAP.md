# Stanford Bunny: Development Roadmap

This document outlines the versioned releases, goalposts, and slices for the **Bunny** project, using the METHOD lightweight process framework.

The stable ownership and backlog surface for deterministic math, geometry,
collision, ray, visibility, optics, SIMD, and validation work lives in
`docs/MATH_GEOMETRY_CAPABILITY_MAP.md`.

---

## Release v0.1.0: Core Deterministic Math (The Math Commons)

* **Status**: Complete
* **Description**: Delivers the baseline fixed-point scalar math, linear algebra vector implementations, and workspace test validation gates.

### Goalpost 1: Deterministic Scalar Profile (`bunny-num`)

* **Description**: Implement software-defined fixed-point math to guarantee bit-level CPU determinism.
* **Slice Budget**: 4 Slices
* **Slices**:
  * **Slice 1.1**: Setup workspace and numeric conversion helpers (`from_f32`, `to_f32`) [Done]
  * **Slice 1.2**: Implement type-safe `FixedQ32_32` wrapper and standard addition/subtraction operators [Done]
  * **Slice 1.3**: Implement multiplication/division operator overloads with intermediate promotion and Banker's rounding [Done]
  * **Slice 1.4**: Implement deterministic square root (`sqrt`) on wide integers and integration tests [Done]

### Goalpost 2: Linear Algebra Primitives (`bunny-linalg`)

* **Description**: Build 2D and 3D vector representations using deterministic math coordinates.
* **Slice Budget**: 2 Slices
* **Slices**:
  * **Slice 2.1**: Define `Vec2`/`Vec3` and `FixedVec2`/`FixedVec3` layouts and operators [Done]
  * **Slice 2.2**: Implement dot products, cross products, normalization, and integration tests [Done]

### Goalpost 3: Workspace Infrastructure and Code Quality Gates

* **Description**: Establish code formatting, Clippy, and cross-platform verification pipelines.
* **Slice Budget**: 2 Slices
* **Slices**:
  * **Slice 3.1**: Establish `CODE_STANDARDS.md` and enforce linter policies (`clippy::pedantic`) [Done]
  * **Slice 3.2**: Implement GitHub Actions workflow (`ci.yml`) for multi-platform (Linux/macOS/Windows) determinism and WebAssembly checks [Done]

---

## Release v0.1.1: Compiler Directive Tuning & Workspace Safeguards (The Tuning Commons)

* **Status**: Complete
* **Description**: Enhances the code generator, improves mathematical safeguards, and configures headless WASM execution gates.

### Goalpost 1: Directive-Driven Scalar Mapping (`bunny-wesley`)

* **Description**: Parse `@bunnyScalarProfile` arguments from schema AST instead of using hardcoded string matching.
* **Slice Budget**: 2 Slices
* **Slices**:
  * **Slice 1.1**: Parse and extract `@bunnyScalarProfile` directive arguments from Wesley IR [Done]
  * **Slice 1.2**: Implement dynamic mapping config based on extracted profiles and deprecate hardcoded name checks [Done]

### Goalpost 2: Numeric Safeguards & Saturation Verification (`bunny-num` / `bunny-linalg`)

* **Description**: Introduce checked mathematical division and verify vector boundary conditions under saturation.
* **Slice Budget**: 2 Slices
* **Slices**:
  * **Slice 2.1**: Implement safe checked division (`checked_div`) returning `Option<FixedQ32_32>` for math guards [Done]
  * **Slice 2.2**: Implement comprehensive boundary-condition unit tests for vector operations under Q32.32 coordinate saturation [Done]

### Goalpost 3: Headless WebAssembly Verification (`bunny-infra`)

* **Description**: Upgrade the CI workflow to execute unit tests inside actual headless WebAssembly environments.
* **Slice Budget**: 1 Slice
* **Slices**:
  * **Slice 3.1**: Configure GitHub Actions to execute every WASM-compatible library crate unit suite inside a headless Node.js/V8 WASM runner via `wasm-pack test`, with host-side tooling covered by native tests [Done]

---

## Release v0.2.0: Spatial Geometry & Intersection Solvers (The Query Commons)

* **Status**: Complete
* **Description**: Introduces bounding shapes and ray-casting query solvers.

### Goalpost 1: Core Bounding Shapes (`bunny-geom` / `bunny-linalg`)

* **Description**: Implement core shapes and type-safe normalized coordinate wrappers.
* **Slice Budget**: 3 Slices
* **Slices**:
  * **Slice 1.1**: Implement `FixedRay3`, `FixedAabb3`, and `FixedSphere3` using `FixedVec3` coordinates [Done]
  * **Slice 1.2**: Implement validating float-to-fixed boundary conversion APIs and infallible fixed-to-float egress [Done]
  * **Slice 1.3**: Implement compile-time normalized wrappers `FixedUnitVec2` and `FixedUnitVec3` to enforce normalization invariants [Done]

### Goalpost 2: Ray-Casting Queries (`bunny-query`)

* **Description**: Implement ray-intersection math solvers.
* **Slice Budget**: 3 Slices
* **Slices**:
  * **Slice 2.1**: Implement ray-sphere intersection solver [Done]
  * **Slice 2.2**: Implement ray-AABB intersection solver [Done]
  * **Slice 2.3**: Implement ray-triangle intersection solver [Done]

### Goalpost 3: Closest Point Queries (`bunny-query`)

* **Description**: Implement minimum-distance calculations between shapes.
* **Slice Budget**: 3 Slices
* **Slices**:
  * **Slice 3.1**: Implement Point-to-Triangle closest point solver [Done]
  * **Slice 3.2**: Implement Segment-to-Segment closest point solver [Done]
  * **Slice 3.3**: Implement AABB-to-Sphere closest point solver [Done]

---

## Release v0.3.0: Spatial Partitioning & Broadphase (The Acceleration Commons)

* **Status**: Complete
* **Description**: Introduces spatial partitioning systems to handle large-scale intersection checks.

### Goalpost 1: Stable BVH Tree (`bunny-broadphase`)

* **Description**: Build a memory-stable, zero-allocation bounding volume hierarchy (BVH).
* **Slice Budget**: 4 Slices
* **Slices**:
  * **Slice 1.1**: Define BVH node layout and array-backed tree layout [Done]
  * **Slice 1.2**: Implement SAH (Surface Area Heuristic) tree building algorithm [Done]
  * **Slice 1.3**: Implement deterministic BVH ray-traversal solver [Done]
  * **Slice 1.4**: Implement BVH box overlap query [Done]

### Goalpost 2: Sweep-and-Prune Solver (`bunny-broadphase`)

* **Description**: Implement multi-axis collision sweeps.
* **Slice Budget**: 2 Slices
* **Slices**:
  * **Slice 2.1**: Implement 1D/3D sorting and sweep overlap queries [Done]
  * **Slice 2.2**: Implement active-pair generator with stable sorting [Done]

---

## Release v0.4.0: Quantized Meshes & Codecs (The Mesh Commons)

* **Status**: Complete; GP1, GP2, and GP3 have merged into `main`. The release
  gate is `v0.4.0` publication through the crates.io release workflow.
* **Description**: Adds compact mesh layouts, PLY/OBJ parser contracts, and compression decoders.

### Goalpost 1: Compressed Mesh Layouts (`bunny-mesh`)

* **Description**: Quantize vertex layouts to reduce memory footprints.
* **Slice Budget**: 3 Slices
* **Slices**:
  * **Slice 1.1**: Implement 16-bit integer quantization mapping for vertices [Done]
  * **Slice 1.2**: Implement index buffer triangulation layouts [Done]
  * **Slice 1.3**: Implement content-addressable hashing (SHA-256) for mesh assets [Done]

### Pre-GP2 Completion Integrity Gate

* **Description**: Re-audit every completed roadmap claim against code, tests,
  and CI before starting new codec work.
* **Slice Budget**: 1 discovery slice plus one or more finish-off slices.
* **Rule**: Goalpost 2 must not start until every outstanding completed-claim
  acceptance criterion below is implemented, tested, and documented. Acceptance
  wording may be clarified only after the implementation satisfies the original
  claim's intent.
* **Slices**:
  * **Slice A**: Fact-check completed roadmap claims and publish honest
    documents listing verified claims, partial claims, and remaining finish-off
    work. [Done]
  * **Slice B+**: Finish the outstanding acceptance criteria discovered by
    Slice A. The number of slices is intentionally open-ended until the audit
    is complete. [Done]
* **Audit Artifact**: `docs/goalposts/v0.4.0-pre-gp2.md` now records verified,
  partial, and unsubstantiated completed-claim findings with repo-truth
  evidence anchors.

#### Outstanding Completed-Claim Acceptance Criteria

These items were previously marked complete, but the current evidence does not
fully satisfy the written acceptance criteria.

| Status | Claim Area | Outstanding Acceptance Criterion | Required Resolution |
| --- | --- | --- | --- |
| Done | v0.1.1-GP1 Directive-Driven Scalar Mapping | Acceptance text requires directive-driven scalar mapping without hardcoded scalar-name checks. | `bunny-wesley` now uses explicit scalar-profile registry data, directive-driven lookup tests, missing-profile rejection tests, and a source-level guard against legacy hardcoded scalar-name branches. |
| Done | v0.1.1-GP3 Headless WebAssembly Verification | Goalpost originally overclaimed a full-workspace WASM run, but host-side tooling crates are not WASM library crates. | CI now runs `wasm-pack test --node --locked` for every WASM-compatible library crate, including `bunny-contract`; `bunny-wesley` and `xtask` are documented host-side tooling exclusions covered by native tests. |
| Done | v0.2.0-GP1 Core Bounding Shapes | Acceptance text previously implied infallible `From`/`Into` float-to-fixed ingress, while safe ingress is fallible by design. | `Ray3`, `Aabb3`, and `Sphere3` now expose validating `try_new` and `try_into_fixed` ingress APIs; fixed types expose `try_from_float`, `into_float`, and standard `TryFrom`/`From` implementations with rejection-path tests. |
| Done | v0.2.0-GP1 Normalized Wrappers | Roadmap calls `FixedUnitVec2` / `FixedUnitVec3` compile-time normalized wrappers, but dynamic inputs require runtime normalization. | `FixedUnitVec2` and `FixedUnitVec3` now expose const `try_from_unit` fixed-unit proof APIs and axis constants while preserving runtime normalization through `new`; native and WASM tests cover valid and rejected const proofs. |
| Done | v0.2.0-GP2 Ray-Casting Queries | Goalpost claims a fixed RNG-seed corpus and cross-platform bitwise/epsilon determinism gate. | `ray_determinism_tests.rs` now defines `RAY_DETERMINISM_CORPUS_SEED`, generates seeded corpus cases from it, and asserts raw Q32.32 expected outputs for ray-sphere, ray-AABB, and ray-triangle cases; the corpus runs in native workspace tests and the WASM `bunny-query` gate. |
| Done | v0.2.0-GP3 Closest Point Queries | Goalpost claims byte-for-byte correctness, but tests mostly assert `to_f32()` values. | `closest_raw_tests.rs` now asserts raw Q32.32 outputs for AABB, AABB/sphere, triangle, and segment closest-point cases, including a fractional half-unit projection, under native and WASM query tests. |
| Done | v0.3.0-GP1 Stable BVH Tree | Goalpost claims zero runtime heap allocation, and standards ban panics in library code, but `bunny-broadphase` still has guarded `unwrap()` and unchecked indexing in builder internals. | `build_bvh` and traversal now use checked buffer/stack access with explicit errors or `None`; malformed builder buffers are tested under native and WASM, and a native counting-allocator test proves build/traversal zero heap allocations. |

### Goalpost 2: File Format Adapters (`bunny-codec`)

* **Description**: Zero-copy mesh deserialization.
* **Status**: Completed.
* **Slice Budget**: 3 Slices
* **Slices**:
  * **Slice 2.1**: Implement zero-copy PLY binary parser. [Done]
  * **Slice 2.2**: Implement zero-copy OBJ parser. [Done]
  * **Slice 2.3**: Create fixture regression suites using Stanford Bunny sample meshes. [Done]

### Goalpost 3: Compression Decoders (`bunny-codec`)

* **Description**: Decode a canonical Bunny compressed mesh byte stream into
  validated `bunny-mesh` buffers.
* **Status**: Merged via PR #105 with green GitHub CI, CodeRabbit approval, and
  no unresolved inline review threads.
* **Slice Budget**: 5 Slices
* **Slices**:
  * **Slice 3.1**: Specify the canonical decoder byte profile, error model, and
    deterministic validation gates. [Done]
  * **Slice 3.2**: Expose the public decoder API, borrowed view, index-width
    model, typed triangle variants, and explicit error model. [Done]
  * **Slice 3.3**: Implement checked header, payload length, count, Q32.32 bounds,
    and triangle-index validation. [Done]
  * **Slice 3.4**: Add checked-in fixture bytes, width-16 and width-32 accepted
    cases, malformed-input corpus tests, and WASM coverage. [Done]
  * **Slice 3.5**: Prove the zero-allocation accepted path natively and record
    evidence in the changelog and goalpost document. [Done]

---

## Post-v0.4.0 Standards Alignment

* **Status**: Completed.
* **Description**: Replaces the older Rust code standards and CI-only quality
  flow with the Rust Code Standards Editor's Edition and Code Dojo repository
  enforcement layer.
* **Goalpost Artifact**:
  `docs/goalposts/post-v0.4.0-standards-alignment.md`

### Goalpost 1: Align with the new code standards and pass all quality gates

* **Description**: Install Code Dojo, make it the active local and CI quality
  gate, then bring code, tests, docs, and generated witnesses into compliance.
* **Slice Budget**: Closed after the full local Code Dojo, headless WASM, and
  release archive verification gates passed.
* **Slices**:
  * **Slice 1.1**: Install standards docs, hooks, scripts, workflow, config, and
    workspace lint inheritance. [Done]
  * **Slice 1.2**: Run Code Dojo and record all failing alignment items. [Done]
  * **Slice 1.3**: Replace regex Rust policy checks with an AST-backed gate
    and package-scoped strict Clippy enforcement. [Done]
  * **Slice 1.4**: Fix standards violations until Code Dojo, headless WASM,
    Markdown, and release archive gates pass. [Done]

---

## Maintenance Rail: v0.4.x Source-of-Truth Hygiene

* **Status**: Planned; six source-of-truth cleanup slices are completed by
  PR #184 and the signpost refresh, and the remaining slices stay open.
* **GitHub Milestone**:
  `v0.4.x Maintenance and Source-of-Truth Hygiene`
* **Description**: Keeps publication, stale tracker cleanup, repo-truth docs,
  and standards ratchets out of the feature release train.

### Goalpost 1: Repo Truth and Release Hygiene

* **Tracker**: #165
* **Description**: Close non-feature work that blocks clean planning and
  publication discipline.
* **Slice Budget**: 12 Slices
* **Slices**:
  * **Slice 1.1**: Make `FixedQ32_32` raw field private.
    [Done in PR #184: #112]
  * **Slice 1.2**: Align `FixedQ32_32` float ingress with rejection policy.
    [Done in PR #184: #113]
  * **Slice 1.3**: Delete or replace `VISION` and `BEARING` as current-state
    docs. [Done: #133]
  * **Slice 1.4**: Refresh README, crate READMEs, and technical teardown to
    current repo surface. [Done: #134]
  * **Slice 1.5**: Close stale completed roadmap issues and normalize backlog
    source of truth. [Planned: #135]
  * **Slice 1.6**: Track the `v0.4.0` crates.io publication gate.
    [Planned: #136]
  * **Slice 1.7**: Remove or tightly gate `ALLOW_DIRTY` release packaging.
    [Planned: #141]
  * **Slice 1.8**: Ratchet workspace lint allowances or document each permanent
    exception. [Planned: #142]
  * **Slice 1.9**: Make README examples model fallible APIs instead of
    unwrap-heavy happy paths. [Planned: #143]
  * **Slice 1.10**: Clarify historical evidence links that reference retired
    CI workflow names. [Planned: #144]
  * **Slice 1.11**: Derive `Hash` for `FixedQ32_32` as required by the Numeric
    Constitution. [Done in PR #184: #153]
  * **Slice 1.12**: Align the canonical raw accessor name with the Numeric
    Constitution. [Done in PR #184: #154]

---

## Release v0.5.0: Standards, Numeric Law, and Release Hygiene

* **Status**: Published.
* **Description**: Ships the repository standards alignment, Code Dojo quality
  gates, numeric law hardening, validated fixed-point ingress, release archive
  verification, and roadmap source-of-truth cleanup completed after `v0.4.0`.
* **Evidence**: `CHANGELOG.md` records the `v0.5.0` release contents; PR #184
  landed the numeric law and roadmap source-of-truth work, and PR #185 merged
  the publishable workspace version at
  `5c15363b6e6da609df76bd37db8bc3b41215ff05`.
* **Release Cut**: Tagged at `v0.5.0` and published through the GitHub Release
  workflow on 2026-06-19.

---

## Planned Release Train

The following releases implement the build order in
`docs/MATH_GEOMETRY_CAPABILITY_MAP.md`: math foundations first, geometry law
second, collision third, acceleration and visibility fourth, optics/performance
fifth, and consumer confidence last.

Issue numbers are GitHub tracker items. They are planning anchors, not evidence
of completion.

Release headings are the durable release milestones. A matching GitHub milestone
is the normal execution surface for the issues and pull requests that complete
that release. The release is cut only after the milestone's must-ship scope has
landed, the release-prep pull request has bumped the workspace version, Code
Dojo and release archive verification are green, and `CHANGELOG.md` records the
shipped surface.

Patch releases are reserved for bug fixes, packaging fixes, standards-gate
repairs, and documentation corrections for behavior that already exists. New
math, geometry, collision, visibility, optics, SIMD, or codec capability belongs
on the next minor release train unless the roadmap explicitly moves the release
cut.

| Release | Cut Trigger | Scope |
| --- | --- | --- |
| `v0.6.0` | `v0.6.0 Math Foundations` milestone complete. | Coordinate law, numeric preconditions, matrices, transforms, projections, orientation, interpolation, and curves. |
| `v0.7.0` | `v0.7.0 Geometry Law and Primitive Coverage` milestone complete. | Degeneracy law, robust predicates, shape expansion, 2D coverage, clipping, bounds, topology, and mass properties. |
| `v0.8.0` | `v0.8.0 Collision and Contact` milestone complete. | Narrowphase, support mapping, GJK/EPA, collision architecture, manifolds, and swept queries. |
| `v0.9.0` | `v0.9.0 Acceleration, Visibility, and Ray Tracing` milestone complete. | Dynamic broadphase, spatial hashing, mesh BVHs, visibility, occlusion, and ray-tracing hit suites. |
| `v0.10.0` | `v0.10.0 Optics, SIMD, and Codec Completion` milestone complete. | Optics, deterministic SIMD exploration, benchmarks, no-std audit, codec encoder, external mesh profiles, and fuzzing. |
| `v0.11.0` | `v0.11.0 Consumer Confidence and Public Readiness` milestone complete. | Examples, fixtures, generated-contract parity, docs.rs readiness, and boundary adapter decisions. |

---

## Release v0.6.0: Math Foundations (The Frame Commons)

* **Status**: Active; Goalpost 1 has merged to `main`.
* **GitHub Milestone**: `v0.6.0 Math Foundations`
* **Description**: Locks coordinate law, numeric preconditions, matrices,
  transforms, projection, orientation, angle, interpolation, and curve math.
* **Release Cut**: Cut `v0.6.0` when Goalposts 1-3 are merged, the release-prep
  PR bumps the workspace version to `0.6.0`, Code Dojo is green, release archive
  verification passes, and the changelog records the math-foundation surface.
* **Must Ship**: Coordinate law, numeric preconditions, deterministic matrices,
  affine transforms, projection and viewport mapping, quaternion orientation,
  angle policy, interpolation helpers, and curve primitives.
* **May Slip**: Nonessential convenience overloads and extended examples that do
  not change the core math contract.
* **Not Included**: New collision solvers, new visibility acceleration
  structures, optics, SIMD backends, and codec feature expansion.

### Goalpost 1: Coordinate Law and Numeric Preconditions

* **Tracker**: #166
* **Status**: Done; merged through PR #189.
* **Description**: Define coordinate spaces, handedness, units, and numeric
  preconditions before more geometry consumes them.
* **Slice Budget**: 3 Slices
* **Slices**:
  * **Slice 1.1**: Define coordinate-space, handedness, and units policy.
    [Done: #164]
  * **Slice 1.2**: Audit saturating arithmetic before geometry consumes invalid
    values. [Done: #114]
  * **Slice 1.3**: Add property tests for Q32.32 and geometry invariants.
    [Done: #129]

### Goalpost 2: Matrix and Transform Primitives

* **Tracker**: #167
* **Description**: Add deterministic matrix, affine transform, projection, and
  bounds-propagation primitives.
* **Slice Budget**: 4 Slices
* **Slices**:
  * **Slice 2.1**: Implement deterministic fixed matrix types.
    [Done: #107, PR #191]
  * **Slice 2.2**: Implement deterministic affine transform types.
    [Done: #108]
  * **Slice 2.3**: Add transform-aware bounds propagation.
    [Planned: #119]
  * **Slice 2.4**: Add deterministic projection, unprojection, and viewport
    mapping. [Planned: #150]

### Goalpost 3: Orientation, Angles, Interpolation, and Curves

* **Tracker**: #168
* **Description**: Provide orientation and parametric math that collision,
  visibility, and optics work can reuse.
* **Slice Budget**: 4 Slices
* **Slices**:
  * **Slice 3.1**: Implement deterministic quaternion rotations.
    [Planned: #109]
  * **Slice 3.2**: Define canonical angle and trigonometry policy.
    [Planned: #110]
  * **Slice 3.3**: Add fixed interpolation, clamp, min-max, and remap
    primitives. [Planned: #111]
  * **Slice 3.4**: Add deterministic curve primitives for Bezier and splines.
    [Planned: #152]

---

## Release v0.7.0: Geometry Law and Primitive Coverage

* **Status**: Planned.
* **GitHub Milestone**: `v0.7.0 Geometry Law and Primitive Coverage`
* **Description**: Defines degeneracy behavior, robust predicates, richer shape
  families, 2D coverage, clipping, bounds, topology, and mass properties.
* **Release Cut**: Cut `v0.7.0` when Goalposts 1-3 are merged, the release-prep
  PR bumps the workspace version to `0.7.0`, Code Dojo is green, release archive
  verification passes, and the changelog records the geometry-law surface.
* **Must Ship**: Degeneracy policy, robust predicates, expanded fixed shapes,
  frustum and culling primitives, 2D query coverage, clipping, bounds merging,
  topology validation, and deterministic mass properties.
* **May Slip**: Extra shape families whose semantics are not needed by the
  collision train.
* **Not Included**: Contact manifold generation, GJK/EPA, dynamic broadphase,
  optics, SIMD, and codec encoder work.

### Goalpost 1: Degeneracy and Robust Predicate Law

* **Tracker**: #169
* **Description**: Define invalid and degenerate geometry behavior before
  expanding the supported shape set.
* **Slice Budget**: 2 Slices
* **Slices**:
  * **Slice 1.1**: Define a degeneracy policy for every shape and query.
    [Planned: #116]
  * **Slice 1.2**: Add robust fixed geometric predicates.
    [Planned: #145]

### Goalpost 2: Shape Library and 2D Coverage

* **Tracker**: #170
* **Description**: Expand Bunny's fixed geometry vocabulary while keeping
  construction and validation policy consistent.
* **Slice Budget**: 3 Slices
* **Slices**:
  * **Slice 2.1**: Expand fixed shape library beyond rays, AABBs, and spheres.
    [Planned: #115]
  * **Slice 2.2**: Add deterministic frustum and culling primitives.
    [Planned: #146]
  * **Slice 2.3**: Add a 2D primitive and query suite. [Planned: #149]

### Goalpost 3: Clipping, Bounds, Topology, and Mass Properties

* **Tracker**: #171
* **Description**: Add geometry utilities that make later collision and mesh
  queries composable.
* **Slice Budget**: 4 Slices
* **Slices**:
  * **Slice 3.1**: Add deterministic clipping and half-space operations.
    [Planned: #162]
  * **Slice 3.2**: Add bounding volume conversion and merge utilities.
    [Planned: #151]
  * **Slice 3.3**: Add topology validation for quantized triangle meshes.
    [Planned: #122]
  * **Slice 3.4**: Add deterministic mass properties for primitive shapes and
    meshes. [Planned: #163]

---

## Release v0.8.0: Collision and Contact

* **Status**: Planned.
* **GitHub Milestone**: `v0.8.0 Collision and Contact`
* **Description**: Adds deterministic narrowphase, SAT, support mapping,
  GJK/EPA, contact manifolds, swept queries, and collision architecture.
* **Release Cut**: Cut `v0.8.0` when Goalposts 1-3 are merged, the release-prep
  PR bumps the workspace version to `0.8.0`, Code Dojo is green, release archive
  verification passes, and the changelog records the collision/contact surface.
* **Must Ship**: Primitive-pair narrowphase coverage, SAT helpers,
  support-mapping primitives, GJK/EPA, collision query architecture, contact
  manifolds, and continuous/swept queries.
* **May Slip**: Higher-level physics stepping, solver integration helpers, and
  broad scene-management conveniences.
* **Not Included**: Owning a physics engine, renderer integration, mesh BVH
  acceleration, optics, SIMD, and codec encoder work.

### Goalpost 1: Primitive Narrowphase Coverage

* **Tracker**: #172
* **Description**: Fill out deterministic primitive-pair query coverage for the
  shape families Bunny owns.
* **Slice Budget**: 2 Slices
* **Slices**:
  * **Slice 1.1**: Implement OBB, capsule, and plane query solvers.
    [Planned: #117]
  * **Slice 1.2**: Add SAT-based convex collision helpers.
    [Planned: #147]

### Goalpost 2: Support Mapping, GJK, and EPA

* **Tracker**: #173
* **Description**: Add support-mapping primitives and convex
  distance/penetration algorithms without importing simulation policy.
* **Slice Budget**: 1 Slice
* **Slices**:
  * **Slice 2.1**: Add support-mapping and GJK/EPA collision primitives.
    [Planned: #139]

### Goalpost 3: Collision Architecture, Manifolds, and Swept Queries

* **Tracker**: #174
* **Description**: Define a composable collision query architecture that returns
  deterministic contacts without owning physics stepping.
* **Slice Budget**: 3 Slices
* **Slices**:
  * **Slice 3.1**: Define deterministic collision detection architecture.
    [Planned: #157]
  * **Slice 3.2**: Add deterministic contact manifold generation.
    [Planned: #118]
  * **Slice 3.3**: Add continuous collision and swept shape queries.
    [Planned: #148]

---

## Release v0.9.0: Acceleration, Visibility, and Ray Tracing

* **Status**: Planned.
* **GitHub Milestone**:
  `v0.9.0 Acceleration, Visibility, and Ray Tracing`
* **Description**: Adds dynamic broadphase, spatial hashing, mesh BVHs,
  occlusion, visibility batching, and ray-tracing hit suites.
* **Release Cut**: Cut `v0.9.0` when Goalposts 1-3 are merged, the release-prep
  PR bumps the workspace version to `0.9.0`, Code Dojo is green, release archive
  verification passes, and the changelog records the acceleration and visibility
  surface.
* **Must Ship**: Dynamic broadphase update/refit APIs, spatial hash or uniform
  grid solver, BVH-backed mesh ray and closest-point queries, visibility and
  occlusion primitives, and ray-tracing primitive intersection records.
* **May Slip**: Renderer-specific batching conveniences and optional
  acceleration-structure variants beyond the primary deterministic design.
* **Not Included**: Lighting/BRDF math, SIMD backend selection, compressed mesh
  encoder, external mesh profiles, and public readiness polish.

### Goalpost 1: Dynamic Broadphase and Spatial Hashing

* **Tracker**: #175
* **Description**: Add update-friendly spatial acceleration structures for
  larger dynamic scenes while preserving stable output order.
* **Slice Budget**: 2 Slices
* **Slices**:
  * **Slice 1.1**: Add deterministic dynamic update and refit APIs.
    [Planned: #120]
  * **Slice 1.2**: Add deterministic spatial hash or uniform grid solver.
    [Planned: #121]

### Goalpost 2: Mesh BVH and Triangle Query Acceleration

* **Tracker**: #176
* **Description**: Connect mesh buffers to deterministic acceleration structures
  for ray and closest-point work.
* **Slice Budget**: 1 Slice
* **Slices**:
  * **Slice 2.1**: Add BVH-backed mesh ray and closest-point queries.
    [Planned: #123]

### Goalpost 3: Visibility and Ray Tracing Hit Suites

* **Tracker**: #177
* **Description**: Expose deterministic visibility, occlusion, and ray hit
  records that renderers and simulators can consume.
* **Slice Budget**: 2 Slices
* **Slices**:
  * **Slice 3.1**: Add deterministic occlusion and visibility query primitives.
    [Planned: #158]
  * **Slice 3.2**: Add deterministic ray tracing primitive intersection suite.
    [Planned: #159]

---

## Release v0.10.0: Optics, SIMD, and Codec Completion

* **Status**: Planned.
* **GitHub Milestone**: `v0.10.0 Optics, SIMD, and Codec Completion`
* **Description**: Adds camera and optics math, deterministic SIMD exploration,
  benchmark evidence, no_std audit, codec encoder, checksums, external mesh
  profiles, and fuzzing.
* **Release Cut**: Cut `v0.10.0` when Goalposts 1-3 are merged, the
  release-prep PR bumps the workspace version to `0.10.0`, Code Dojo is green,
  release archive verification passes, and the changelog records the optics,
  performance, and codec-completion surface.
* **Must Ship**: Camera and ray-generation primitives, lighting and BRDF math,
  deterministic SIMD parity exploration, benchmark harnesses, no-std audit,
  compressed mesh encoder, checksum profile, glTF/STL import profiles, and
  decoder fuzzing harnesses.
* **May Slip**: SIMD implementation backends that fail scalar parity or do not
  beat scalar code with clear evidence.
* **Not Included**: Downstream application adapters, broad public tutorial
  expansion, and generated TypeScript parity beyond existing contract checks.

### Goalpost 1: Deterministic Camera and Optics Math

* **Tracker**: #178
* **Description**: Add deterministic camera, ray-generation, lighting, and
  optics primitives without becoming a renderer.
* **Slice Budget**: 2 Slices
* **Slices**:
  * **Slice 1.1**: Add `bunny-optics` deterministic camera and ray-generation
    primitives. [Planned: #138]
  * **Slice 1.2**: Add deterministic lighting and BRDF math primitives.
    [Planned: #160]

### Goalpost 2: Deterministic SIMD and Performance Evidence

* **Tracker**: #179
* **Description**: Explore acceleration only after scalar behavior is locked and
  measurable.
* **Slice Budget**: 3 Slices
* **Slices**:
  * **Slice 2.1**: Explore optional deterministic SIMD backends with scalar
    parity gates. [Planned: #161]
  * **Slice 2.2**: Add benchmark harnesses for math, codecs, mesh, and
    broadphase. [Planned: #130]
  * **Slice 2.3**: Audit `no_std` support for core math crates.
    [Planned: #156]

### Goalpost 3: Codec Encoder and External Mesh Profiles

* **Tracker**: #180
* **Description**: Complete Bunny's mesh codec story with canonical encoding,
  stronger validation, external profiles, and fuzzing.
* **Slice Budget**: 5 Slices
* **Slices**:
  * **Slice 3.1**: Implement Bunny compressed mesh encoder.
    [Planned: #124]
  * **Slice 3.2**: Define compressed mesh profile v2 with checksum support.
    [Planned: #125]
  * **Slice 3.3**: Add deterministic glTF mesh import profile.
    [Planned: #126]
  * **Slice 3.4**: Add deterministic STL mesh import profile.
    [Planned: #127]
  * **Slice 3.5**: Add fuzzing harnesses for PLY, OBJ, and compressed mesh
    decoders. [Planned: #128]

---

## Release v0.11.0: Consumer Confidence and Public Readiness

* **Status**: Planned.
* **GitHub Milestone**:
  `v0.11.0 Consumer Confidence and Public Readiness`
* **Description**: Adds examples, fixtures, generated-contract parity,
  docs.rs readiness, and optional boundary adapter exploration.
* **Release Cut**: Cut `v0.11.0` when Goalposts 1-3 are merged, the
  release-prep PR bumps the workspace version to `0.11.0`, Code Dojo is green,
  release archive verification passes, and the changelog records the public
  readiness surface.
* **Must Ship**: Runnable examples, shared deterministic fixtures, TypeScript
  DTO compile/parity checks, rustdoc/docs.rs readiness checks, and an explicit
  decision on Echo/Geordi boundary adapters.
* **May Slip**: Optional adapter crates if they compromise Bunny's
  project-neutral core or lack stable downstream contracts.
* **Not Included**: New foundational math, collision, visibility, optics, SIMD,
  or codec capabilities that should have shipped in earlier release trains.

### Goalpost 1: Examples and Fixture Corpora

* **Tracker**: #181
* **Description**: Make public APIs easier to consume correctly by adding
  runnable examples and shared deterministic fixtures.
* **Slice Budget**: 2 Slices
* **Slices**:
  * **Slice 1.1**: Add runnable examples for downstream consumers.
    [Planned: #131]
  * **Slice 1.2**: Add `bunny-fixtures` crate for deterministic corpora and
    parity witnesses. [Planned: #137]

### Goalpost 2: Generated Contract and Public Documentation Readiness

* **Tracker**: #182
* **Description**: Prove generated contracts and public docs are ready for
  downstream consumers.
* **Slice Budget**: 2 Slices
* **Slices**:
  * **Slice 2.1**: Add TypeScript DTO compile and parity checks.
    [Planned: #132]
  * **Slice 2.2**: Add rustdoc and docs.rs readiness checks for public crates.
    [Planned: #155]

### Goalpost 3: Boundary Adapter Exploration

* **Tracker**: #183
* **Description**: Decide whether optional Echo and Geordi adapter crates belong
  at Bunny's repository boundary without compromising the project-neutral core.
* **Slice Budget**: 1 Slice
* **Slices**:
  * **Slice 3.1**: Add optional Echo and Geordi adapter crates at repository
    boundaries. [Planned: #140]
