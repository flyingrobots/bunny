# Bunny

Bunny is a neutral Rust graphics commons for deterministic math, geometry,
query, mesh, optics, and render-contract primitives.

It is named after the Stanford Bunny and exists so projects can share graphics
infrastructure without depending on Echo, Geordi, jedit, or any other product
runtime.

## Role

Bunny answers:

```text
What is the deterministic graphics, math, geometry, or render-contract
operation?
```

It does not answer:

```text
What causally happened?
What renderer proved a frame?
What editor or app workflow is active?
```

Those jobs belong to downstream projects.

## Relationship To Other Projects

Bunny owns math, geometry, shape/query contracts, mesh, and optics substrate.
It is the source of reusable graphics primitives.

Echo owns causal runtime, strands, braids, transactions, and provenance. It may
depend on Bunny and wrap Bunny results as causal facts.

Geordi owns render IR, renderer backends, receipts, and feature negotiation. It
may depend on Bunny for math, geometry, mesh, optics, and schema contracts.

jedit/Jim owns editor product behavior and user workflows. It should consume
Bunny directly only for justified hot paths.

## Initial Crate Map

```text
crates/
  bunny-num
    deterministic scalar profiles and finite-number policy

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
