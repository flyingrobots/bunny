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

## Release v0.1.1: Compiler Directive Tuning (The Compiler Commons)
* **Status**: Planned
* **Description**: Enhances the code generator to dynamically resolve scalar profiles.

### Goalpost 1: Directive-Driven Scalar Mapping (`bunny-wesley`)
* **Description**: Parse `@bunnyScalarProfile` arguments from schema AST instead of using hardcoded string matching.
* **Slice Budget**: 2 Slices
* **Slices**:
  * **Slice 1.1**: Parse and extract `@bunnyScalarProfile` directive arguments from Wesley IR [Issue #1]
  * **Slice 1.2**: Implement dynamic mapping config based on extracted profiles and deprecate hardcoded name checks

---

## Release v0.2.0: Spatial Geometry & Intersection Solvers (The Query Commons)
* **Status**: Planned
* **Description**: Introduces bounding shapes and ray-casting query solvers.

### Goalpost 1: Core Bounding Shapes (`bunny-geom`)
* **Description**: Implement core shapes using fixed-point vectors.
* **Slice Budget**: 2 Slices
* **Slices**:
  * **Slice 1.1**: Implement `FixedRay3`, `FixedAabb3`, and `FixedSphere3` using `FixedVec3` coordinates.
  * **Slice 1.2**: Implement shape boundary conversion traits (`From`/`Into`) for float boundaries.

### Goalpost 2: Ray-Casting Queries (`bunny-query`)
* **Description**: Implement ray-intersection math solvers.
* **Slice Budget**: 3 Slices
* **Slices**:
  * **Slice 2.1**: Implement ray-sphere intersection solver.
  * **Slice 2.2**: Implement ray-AABB intersection solver.
  * **Slice 2.3**: Implement ray-triangle intersection solver.

### Goalpost 3: Closest Point Queries (`bunny-query`)
* **Description**: Implement minimum-distance calculations between shapes.
* **Slice Budget**: 3 Slices
* **Slices**:
  * **Slice 3.1**: Implement Point-to-Triangle closest point solver.
  * **Slice 3.2**: Implement Segment-to-Segment closest point solver.
  * **Slice 3.3**: Implement AABB-to-Sphere closest point solver.

---

## Release v0.3.0: Spatial Partitioning & Broadphase (The Acceleration Commons)
* **Status**: Planned
* **Description**: Introduces spatial partitioning systems to handle large-scale intersection checks.

### Goalpost 1: Stable BVH Tree (`bunny-broadphase`)
* **Description**: Build a memory-stable, zero-allocation bounding volume hierarchy (BVH).
* **Slice Budget**: 4 Slices
* **Slices**:
  * **Slice 1.1**: Define BVH node layout and array-backed tree layout.
  * **Slice 1.2**: Implement SAH (Surface Area Heuristic) tree building algorithm.
  * **Slice 1.3**: Implement deterministic BVH ray-traversal solver.
  * **Slice 1.4**: Implement BVH box overlap query.

### Goalpost 2: Sweep-and-Prune Solver (`bunny-broadphase`)
* **Description**: Implement multi-axis collision sweeps.
* **Slice Budget**: 2 Slices
* **Slices**:
  * **Slice 2.1**: Implement 1D/3D sorting and sweep overlap queries.
  * **Slice 2.2**: Implement active-pair generator with stable sorting.

---

## Release v0.4.0: Quantized Meshes & Codecs (The Mesh Commons)
* **Status**: Planned
* **Description**: Adds compact mesh layouts, PLY/OBJ parser contracts, and compression decoders.

### Goalpost 1: Compressed Mesh Layouts (`bunny-mesh`)
* **Description**: Quantize vertex layouts to reduce memory footprints.
* **Slice Budget**: 3 Slices
* **Slices**:
  * **Slice 1.1**: Implement 16-bit integer quantization mapping for vertices.
  * **Slice 1.2**: Implement index buffer triangulation layouts.
  * **Slice 1.3**: Implement content-addressable hashing (SHA-256) for mesh assets.

### Goalpost 2: File Format Adapters (`bunny-codec`)
* **Description**: Zero-copy mesh deserialization.
* **Slice Budget**: 3 Slices
* **Slices**:
  * **Slice 2.1**: Implement zero-copy PLY binary parser.
  * **Slice 2.2**: Implement zero-copy OBJ parser.
  * **Slice 2.3**: Create fixture regression suites using Stanford Bunny sample meshes.
