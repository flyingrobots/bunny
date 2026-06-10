# 0001 — Bunny Graphics Commons

## Purpose

Bunny is a standalone Rust-first graphics commons for deterministic graphics
math, geometry, query, mesh, optics, and render-contract primitives.

It exists because Echo, Geordi, jedit/Jim, and future projects need the same
graphics-related substrate without forcing every consumer to depend on Echo or
Geordi.

## Core Proposition

Graphics primitives that are generally useful across projects belong in Bunny.
Project-specific authority stays in the downstream project.

```text
Bunny
  deterministic graphics substrate

Echo
  causal runtime and provenance

Geordi
  render artifact pipeline and receipts

jedit/Jim
  editor product and workflows
```

## Answers To Current Questions

### Does Bunny Absorb Echo's Deterministic Math Primitives?

Yes, when those primitives are generally useful graphics primitives.

Echo's deterministic math and geometry crates should be treated as the seed
material for Bunny. The extraction rule is:

- Deterministic scalar, vector, matrix, quaternion, transform, shape, query,
  broad-phase, mesh, optics, and lighting primitives move toward Bunny.
- Echo keeps causal wrappers, transaction semantics, strands, braids,
  witnessed histories, readings, intents, and provenance.
- Echo may depend on Bunny, but Bunny must not depend on Echo.

This gives Echo the same deterministic math while preventing Echo from becoming
the required dependency for every graphics-adjacent project.

### Does Bunny Need Its Own GraphQL Schema Files?

Yes.

Bunny owns its primitive graphics schemas. Echo, Geordi, jedit, and future
projects can generate bindings from Bunny schemas, but they must not become the
schema owners for shared graphics primitives.

The intended shape is:

```text
schemas/bunny/v0/*.graphql
  authored Bunny SDL

bunny-wesley
  Bunny/Wesley generation extension

generated outputs
  Rust DTOs
  TypeScript DTOs
  canonical codec fixtures
  schema/hash witnesses
```

The Bunny/Wesley extension should generate data contracts only. Behavior,
algorithms, solvers, and runtime policy remain handwritten and tested in Bunny
crates.

### How Does Bunny Fit With Geordi's Ambition?

Geordi remains the portable authored-scene and render-proof pipeline:

```text
Figma / After Effects / focused CSS subset / GPVue / tools
  -> Geordi IR
  -> Geordi-WebGPU / WebGL / Metal / DirectX / Unity / Unreal / Godot / Bijou
  -> render receipts
```

Bunny sits underneath that stack:

```text
Bunny
  math, bounds, geometry, clipping, hit testing, mesh, optics, lighting

Geordi
  IR, text model, animation model, feature negotiation, renderers, receipts
```

Geordi can use Bunny, but Bunny must not learn Geordi IR, Geordi receipts,
Figma frames, browser DOM nodes, Unity objects, or game-engine entity models.

## Ownership Invariants

- Bunny is the neutral graphics substrate.
- Bunny core crates do not contain Echo, Geordi, jedit, DOM, Unity, Unreal,
  Godot, browser, or editor nouns.
- Bunny owns shared graphics schema files.
- Bunny/Wesley owns generated cross-language graphics DTOs.
- Echo owns causal authority, strands, braids, transactions, and provenance.
- Geordi owns render IR, renderer backends, strict text policy, animation
  profiles, feature negotiation, and render receipts.
- jedit/Jim owns editor workflows and product UX.
- Deterministic CPU semantics are defined before GPU acceleration.
- GPU, WASM, engine, and renderer integrations are adapters, not the source of
  truth for Bunny semantics.

## Initial Library Surface

The first Bunny workspace should stay small:

| Crate | Purpose |
| --- | --- |
| `bunny-num` | Deterministic scalar profiles and finite-number policy |
| `bunny-linalg` | Vectors, matrices, quaternions, transforms |
| `bunny-geom` | Shapes, rays, segments, contacts, bounds |
| `bunny-contract` | Schema/version helpers and generated contract support |

Planned follow-on crates:

- `bunny-query`: overlap, raycast, swept collision, closest point
- `bunny-broadphase`: BVH, grids, sweep-and-prune, stable pair generation
- `bunny-mesh`: mesh buffers, manifests, import adapters, content identity
- `bunny-optics`: cameras, sampling, material/light math, CPU shading helpers
- `bunny-codec`: canonical bytes and raw binary codecs
- `bunny-fixtures`: golden fixtures and cross-language parity witnesses
- `bunny-wesley`: schema generation extension
- `bunny-echo`: optional Echo adapter
- `bunny-geordi`: optional Geordi adapter

## Extraction Rule From Echo

An Echo primitive is eligible for Bunny when all are true:

1. It has no dependency on Echo runtime state, WSC storage, strands, braids,
   scheduler policy, or provenance.
2. It can be specified as deterministic data plus pure behavior.
3. It is useful to Geordi, jedit, or another graphics-adjacent project without
   importing Echo.
4. Its contract can be represented in Bunny-owned schema or explicit Rust API.

An Echo primitive stays in Echo when any are true:

1. It names causal authority.
2. It depends on witnessed history or retained readings.
3. It is an intent, transaction, aperture, optic, or strand operation.
4. Its result is meaningful only inside Echo's runtime model.

## Extraction Rule From Geordi

A Geordi primitive is eligible for Bunny when all are true:

1. It is general math, geometry, mesh, optics, bounds, clipping, or query logic.
2. It does not name Geordi IR, render receipts, renderer profiles, strict text
   proof, or feature negotiation.
3. It can be tested independently of a renderer backend.

A Geordi primitive stays in Geordi when any are true:

1. It defines authored-scene IR.
2. It defines text rendering policy or glyph evidence.
3. It defines renderer feature requirements.
4. It defines visual proof, receipts, or backend compatibility claims.

## Non-Goals

- Bunny is not a browser.
- Bunny is not a UI framework.
- Bunny is not a game engine.
- Bunny is not a renderer proof system.
- Bunny is not Echo without causality.
- Bunny is not Geordi without receipts.
- Bunny does not promise full CSS, DOM, Figma, After Effects, Unity, Unreal, or
  Godot compatibility.

## First Validation Standard

The first implementation goalpost should prove:

- the workspace builds with `cargo test --workspace`
- the authored GraphQL schema is lintable or parseable by the chosen tool
- generated Rust and TypeScript DTOs are planned but not hand-waved
- at least one primitive exists in Rust and SDL
- Echo and Geordi docs point to Bunny as the neutral graphics substrate
