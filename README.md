# Bunny

Bunny is a neutral, open-source Rust graphics commons that provides
deterministic math, geometry, ray-casting queries, broadphase acceleration,
mesh layouts, mesh codecs, and render-contract primitives.

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

To understand Bunny, it helps to understand the downstream projects in the ecosystem:

| Project | Role | Integration |
| :--- | :--- | :--- |
| **[Echo](https://github.com/flyingrobots/echo)** | Causal database & transaction engine tracking Strands, Braids, and Provenance. | Depends on Bunny to compute deterministic geometric results, wrapping them as causal facts. |
| **[Geordi](https://github.com/flyingrobots/geordi)** | Deterministic rendering backend. | Consumes Bunny's math, mesh, codec, and planned optics specifications to guarantee rendered geometry matches calculated inputs. |
| **[jedit](https://github.com/flyingrobots/jedit)** | Interactive editor application & workspace interface. | Consumes Bunny and Echo to present visual editor states and behaviors to the user. |
| **[Wesley](https://github.com/flyingrobots/wesley)** | Schema compiler translating GraphQL SDL to DTOs. | Extended by `bunny-wesley` to generate shared Rust and TypeScript types from graphics schemas. |

## Workspace Crate Map

```text
crates/
  bunny-num
    deterministic scalar profiles, finite-number policy, Q32.32 helpers

  bunny-linalg
    deterministic 2D/3D vectors and unit-vector invariants

  bunny-geom
    validated rays, AABBs, spheres, and fixed/float boundary conversion

  bunny-query
    deterministic ray intersections and closest-point solvers

  bunny-broadphase
    BVH and sweep-and-prune broadphase helpers

  bunny-mesh
    quantized vertex layouts, triangle buffers, and mesh hashes

  bunny-codec
    zero-copy PLY/OBJ parsers and Bunny compressed mesh decoder

  bunny-contract
    schema and canonical contract helpers

  bunny-wesley
    Wesley-backed schema extension and DTO generator

xtask/
  host-side automation for generation and Code Dojo quality gates
```

Future crate candidates in the roadmap include `bunny-optics`,
`bunny-fixtures`, and optional boundary adapter crates. Matrices, transforms,
quaternions, optics, deterministic SIMD, richer collision, and public examples
are tracked in `ROADMAP.md` and `docs/MATH_GEOMETRY_CAPABILITY_MAP.md`.

## Contract Generation

Bunny owns its shared graphics schemas under `schemas/bunny`.

Regenerate checked-in DTO witnesses with:

```bash
cargo run --locked -p xtask -- generate
```

The current generator emits:

* **Rust DTOs**: Emitted directly into the shared contracts package at `crates/bunny-contract/src/generated/graphics.rs`.
* **TypeScript DTOs**: Emitted for downstream browser/Node.js clients at `generated/typescript/bunny-graphics.ts`.
* **Manifest**: Emitted at `generated/bunny-graphics.manifest.json`, detailing the schema SHA-256 hash and generated output file paths.

The generator uses published Wesley SDL lowering:

* `bunny-wesley` lowers Bunny SDL through `wesley-core`.
* Bunny maps the Wesley IR into graphics-specific Rust and TypeScript DTOs.
* Bunny records the `wesley-core` version in generated witnesses.

## Numeric Profiles

Bunny currently defines:

* `BunnyScalar`: finite `f32` graphics scalar profile.
* `BunnyFixedQ32_32Raw`: signed Q32.32 fixed-point raw `i64` profile.

Q32.32 conversion helpers live in `bunny-num::fixed_q32_32`.

## Invariants

* Bunny is project-neutral.
* Bunny owns its own GraphQL schema files.
* `bunny-wesley` generates shared Rust and TypeScript contract types, and the
  generated witnesses record the generator and `wesley-core` versions.
* Echo deterministic math primitives should migrate into Bunny when they are
  generally useful graphics primitives.
* Echo keeps causal wrappers, provenance, and runtime authority.
* Geordi keeps IR, render backends, text-rendering policy, receipts, and proof
  claims.
* Bunny core crates do not know about Echo strands, Geordi receipts, jedit
  editor state, DOM nodes, Unity objects, or browser compatibility quirks.
* Deterministic CPU semantics come before GPU acceleration.

## License

Apache-2.0.
