# Bunny

Bunny is a neutral, open-source Rust graphics commons that provides deterministic math, geometry, ray-casting queries, mesh layouts, optics, and render-contract primitives. 

Named after the iconic **Stanford Bunny** (a computer graphics 3D test model), the project exists to establish absolute, bit-level CPU mathematical determinism across all platforms and compilation targets. By decoupling these primitives into a shared, runtime-neutral library, downstream systems can compute consistent graphics and geometric invariants without importing heavy app behaviors, causal database runtimes, or rendering backends.

---

## Rationale: Why Determinism?

In modern multi-platform applications, minor differences in CPU floating-point calculations (e.g., due to compiler optimizations, CPU instructions like FMA, or target architectures) can lead to drift. Over time, these minute deviations cause split-brain behavior in simulated physics, collision detection, and ray-casting. 

Bunny solves this by utilizing a fixed-point numerical profile (`FixedQ32_32`) and ensuring that all geometric operations (like dot products, normalization, and square roots) produce identical bitwise results across Linux, macOS, Windows, and WebAssembly (`wasm32-unknown-unknown`).

---

## Role

Bunny answers:
```text
What is the deterministic graphics, math, geometry, or render-contract operation?
```

It does not answer:
```text
What causal database events occurred?
What hardware renderer drew or rasterized a frame?
What interactive editor workflows or product layouts are active?
```

Those jobs belong to downstream projects.

---

## Ecosystem Context & Relationships

To understand Bunny, it helps to understand the other downstream projects in the workspace and ecosystem:

*   **Echo (Causal Database & Runtime)**: Echo is a causal database and transaction engine. It tracks causal histories (strands, braids, transactions, and provenance) to coordinate and replicate state across users. Echo depends on Bunny to compute deterministic geometric results, wrapping them as immutable causal facts.
*   **Geordi (Deterministic Rendering Backend)**: Geordi is a rendering engine and backend IR. It translates scene descriptions into rendered pixels, negotiates GPU features, and issues cryptographic proofs/receipts of rendering completion. Geordi consumes Bunny's math, mesh, and optics specifications to guarantee that the scene geometry it draws matches the geometry computed by the database.
*   **jedit / Jim (Interactive Editor & Workspace)**: `jedit` is the user-facing editor application and workspace interface. It defines product behavior, panels, and user workflows. It consumes Bunny and Echo to present visual editor states to the user.
*   **Wesley (Schema Compiler)**: Wesley is a compiler that translates schema files (`.graphql` SDL) into language-specific DTOs (Data Transfer Objects). Bunny uses a custom code generator (`bunny-wesley`) extending `wesley-core` to compile the graphics schemas under `schemas/bunny/` into Rust and TypeScript types.


## Initial Crate Map

```text
crates/
  bunny-num
    deterministic scalar profiles, finite-number policy, Q32.32 helpers

  bunny-linalg
    vectors, matrices, quaternions, transforms

  bunny-geom
    shapes, contacts, rays, segments, triangles, boxes, spheres

  bunny-contract
    schema and canonical contract helpers

  bunny-wesley
    Bunny-owned schema parser and DTO generator
```

Planned crates include `bunny-query`, `bunny-broadphase`, `bunny-mesh`,
`bunny-optics`, `bunny-codec`, `bunny-fixtures`, `bunny-echo`, and
`bunny-geordi`.

## Contract Generation

Bunny owns its shared graphics schemas under `schemas/bunny`.

Regenerate checked-in DTO witnesses with:

```bash
bash scripts/generate-contracts.sh
```

The current generator emits:

- Rust DTOs for `bunny-contract`
- TypeScript DTOs for downstream consumers
- a manifest with the schema SHA-256 hash and output paths

The generator extends published Wesley lowering:

- `wesley-core` lowers Bunny SDL into Wesley IR.
- Bunny maps Wesley IR into graphics-specific Rust and TypeScript DTOs.
- Bunny records the Wesley core version in generated witnesses.

## Numeric Profiles

Bunny currently defines:

- `BunnyScalar`: finite `f32` graphics scalar profile.
- `BunnyFixedQ32_32Raw`: signed Q32.32 fixed-point raw `i64` profile.

Q32.32 conversion helpers live in `bunny-num::fixed_q32_32`.

## Invariants

- Bunny is project-neutral.
- Bunny owns its own GraphQL schema files.
- Bunny/Wesley generates shared Rust and TypeScript contract types.
- Echo deterministic math primitives should migrate into Bunny when they are
  generally useful graphics primitives.
- Echo keeps causal wrappers, provenance, and runtime authority.
- Geordi keeps IR, render backends, text-rendering policy, receipts, and proof
  claims.
- Bunny core crates do not know about Echo strands, Geordi receipts, jedit
  editor state, DOM nodes, Unity objects, or browser compatibility quirks.
- Deterministic CPU semantics come before GPU acceleration.

## License

Apache-2.0.
