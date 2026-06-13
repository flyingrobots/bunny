# bunny-wesley

GraphQL DTO code generator and schema compiler for the Bunny project.

This crate parses Bunny's GraphQL schema Definition Language (SDL) and generates Rust structs, TypeScript interfaces, and integration manifests.

## Core Features

* **Wesley lowering**: Extends published `wesley-core` compiler logic to process GraphQL schema ASTs.
* **Rust Emitter**: Emits type-safe data transfer objects (DTOs) with type alias mappings for custom scalars (`BunnyScalar` -> `f32`, `BunnyFixedQ32_32Raw` -> `i64`).
* **TypeScript Emitter**: Emits read-only interfaces and type bindings mapping custom scalars (`BunnyScalar` -> `number`, `BunnyFixedQ32_32Raw` -> `bigint`).
* **Manifest Emitter**: Generates an integrity JSON manifest containing compilation paths, generator versions, and the schema's SHA-256 hash.

## Usage

```bash
bunny-wesley <schema.graphql> --rust <path> --typescript <path> --manifest <path>
```

Options:
* `--rust`: Target path for generated Rust DTOs.
* `--typescript`: Target path for generated TypeScript DTOs.
* `--manifest`: Target path for compilation integrity manifest.

## License

Apache-2.0.
