# Stanford Bunny: Development Roadmap

This document outlines the versioned releases, goalposts, and slices for the **Bunny** project, using the METHOD lightweight process framework.

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
  * **Slice 3.1**: Configure GitHub Actions to execute the full workspace unit test suite inside a headless Node.js/V8 WASM runner via `wasm-pack test` [Done]

---

## Release v0.2.0: Spatial Geometry & Intersection Solvers (The Query Commons)

* **Status**: Complete
* **Description**: Introduces bounding shapes and ray-casting query solvers.

### Goalpost 1: Core Bounding Shapes (`bunny-geom` / `bunny-linalg`)

* **Description**: Implement core shapes and type-safe normalized coordinate wrappers.
* **Slice Budget**: 3 Slices
* **Slices**:
  * **Slice 1.1**: Implement `FixedRay3`, `FixedAabb3`, and `FixedSphere3` using `FixedVec3` coordinates [Done]
  * **Slice 1.2**: Implement shape boundary conversion traits (`From`/`Into`) for float boundaries [Done]
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

* **Status**: Active, blocked before Goalpost 2 by the Pre-GP2 Completion
  Integrity Gate.
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
    is complete. [Active]

#### Outstanding Completed-Claim Acceptance Criteria

These items were previously marked complete, but the current evidence does not
fully satisfy the written acceptance criteria.

| Status | Claim Area | Outstanding Acceptance Criterion | Required Resolution |
| --- | --- | --- | --- |
| Done | v0.1.1-GP1 Directive-Driven Scalar Mapping | Acceptance text requires directive-driven scalar mapping without hardcoded scalar-name checks. | `bunny-wesley` now uses explicit scalar-profile registry data, directive-driven lookup tests, and a source-level guard against legacy hardcoded scalar-name branches. |
| Done | v0.1.1-GP3 Headless WebAssembly Verification | Goalpost claims the full workspace unit suite runs under headless WASM, but CI only runs WASM tests for the runtime crates. | CI now runs `wasm-pack test --node --locked` for every WASM-compatible library crate, including `bunny-contract`; `bunny-wesley` and `xtask` are documented host-side tooling exclusions covered by native tests. |
| Done | v0.2.0-GP1 Core Bounding Shapes | Acceptance text says `From`/`Into` maps float geometries into fixed-point spaces, while safe ingress is now fallible `TryFrom`. | `Ray3`, `Aabb3`, and `Sphere3` now expose validating `try_new` and `try_into_fixed` ingress APIs; fixed types expose `try_from_float`, `into_float`, and standard `TryFrom`/`From` implementations with rejection-path tests. |
| Done | v0.2.0-GP1 Normalized Wrappers | Roadmap calls `FixedUnitVec2` / `FixedUnitVec3` compile-time normalized wrappers, but the invariant is runtime-validated. | `FixedUnitVec2` and `FixedUnitVec3` now expose const `try_from_unit` exact-unit proof APIs and axis constants while preserving runtime normalization through `new`; native and WASM tests cover valid and rejected const proofs. |
| Pending | v0.2.0-GP2 Ray-Casting Queries | Goalpost claims a fixed RNG-seed corpus and cross-platform bitwise/epsilon determinism gate. Current tests are deterministic examples, not a corpus/divergence gate. | Add canonical raw-output regression fixtures for ray queries and execute them in CI across native and WASM gates. |
| Pending | v0.2.0-GP3 Closest Point Queries | Goalpost claims byte-for-byte correctness, but tests mostly assert `to_f32()` values. | Add raw Q32.32 assertions for closest-point outputs. |
| Pending | v0.3.0-GP1 Stable BVH Tree | Goalpost claims zero runtime heap allocation, and standards ban panics in library code, but `bunny-broadphase` still has guarded `unwrap()` and unchecked indexing in builder internals. | Remove panic-capable library operations, add regression coverage for malformed inputs, and add an allocation proof/check for the zero-allocation BVH builder/traversal API surface. |

### Goalpost 2: File Format Adapters (`bunny-codec`)

* **Description**: Zero-copy mesh deserialization.
* **Status**: Blocked on the Pre-GP2 Completion Integrity Gate.
* **Slice Budget**: 3 Slices
* **Slices**:
  * **Slice 2.1**: Implement zero-copy PLY binary parser.
  * **Slice 2.2**: Implement zero-copy OBJ parser.
  * **Slice 2.3**: Create fixture regression suites using Stanford Bunny sample meshes.
