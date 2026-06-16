# Bunny - Vision

## Purpose

Bunny is a Rust graphics commons for deterministic math, geometry, mesh, and
codec primitives. Its job is to give downstream runtimes one portable source of
truth for graphics data that must behave the same on native CPU targets and
WebAssembly.

Bunny optimizes for:

* Bit-level deterministic numeric behavior.
* Small, validated value-object APIs.
* Borrowed, allocation-conscious data views at file and mesh boundaries.
* Clear contracts that can be consumed by other Flying Robots projects without
  importing their runtime assumptions.

## Boundary

Bunny is intentionally neutral infrastructure.

Core Bunny crates must not learn about Echo transaction strands, Geordi render
backend receipts, jedit UI workflows, Unity runtime state, browser state, or any
other downstream application concept. Downstream projects consume Bunny; Bunny
does not reach back into them.

## Current Shape

The repo is organized around narrow crates:

| Crate | Role |
| --- | --- |
| `bunny-num` | Q32.32 scalar profile, rounding, saturation, division, sqrt |
| `bunny-linalg` | Fixed-point vectors and unit-vector invariants |
| `bunny-geom` | Fixed rays, AABBs, spheres, and float-boundary conversion |
| `bunny-query` | Ray and closest-point query solvers |
| `bunny-broadphase` | BVH and sweep-and-prune broadphase queries |
| `bunny-mesh` | Quantized vertex layouts, triangle buffers, mesh hashes |
| `bunny-codec` | Zero-copy PLY/OBJ parsers and compressed mesh decoders |
| `bunny-contract` | Shared contract surface for generated DTO boundaries |
| `bunny-wesley` | Host-side GraphQL SDL contract generator |
| `xtask` | Host-side repository automation |

## What Must Stay True

* Fixed-point math is the canonical computational reality.
* Floating-point values are boundary convenience formats, never trusted internal
  geometry truth.
* Ingress APIs reject invalid or non-finite data before it enters deterministic
  geometry paths.
* Library crates do not panic on malformed caller input.
* Public claims in roadmap and signpost documents must be backed by code, tests,
  CI, or explicit evidence artifacts.

## Signposts

| Document | Responsibility |
| --- | --- |
| `README.md` | Public project entry point |
| `CODE_STANDARDS.md` | Determinism and Rust quality rules |
| `ROADMAP.md` | Versioned releases, goalposts, and slice state |
| `docs/VISION.md` | Long-lived purpose and boundaries |
| `docs/BEARING.md` | Current operating position and next work |
| `docs/PROCESS.md` | How work moves from goalpost to merge |
| `docs/TESTING.md` | Required verification strategy and commands |

## Near Horizon

Release `v0.4.0` is in the mesh commons track. GP1 shipped compressed mesh
layouts. GP2 merged file format adapters. GP3 now adds a Bunny-native compressed
mesh decoder with a documented byte profile, checked accessors, and native/WASM
evidence.
