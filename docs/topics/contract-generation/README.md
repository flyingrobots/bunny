# Contract Generation

This topic defines Bunny's current schema-to-artifact generation surface.

`bunny-wesley` reads a Bunny GraphQL SDL schema, lowers it through
`wesley-core`, and emits checked-in contract artifacts for Bunny-owned graphics
DTOs. The generator is a build-time tool. It does not define runtime semantics,
perform byte encoding or decoding, or make downstream repositories the owners of
Bunny's contract vocabulary.

## Status

This is a living topic chapter. It describes behavior present in `HEAD`.

The current checked-in generation target is the Bunny graphics schema:

- input schema: `schemas/bunny/v0/graphics.graphql`
- Rust output: `crates/bunny-contract/src/generated/graphics.rs`
- TypeScript output: `generated/typescript/bunny-graphics.ts`
- manifest output: `generated/bunny-graphics.manifest.json`

Regenerate those artifacts with:

```sh
cargo run --locked -p xtask -- generate
```

## Ownership

Bunny owns:

- the graphics contract schema under `schemas/bunny/`;
- the generated Rust DTO surface in `bunny-contract`;
- the generated TypeScript DTO surface under `generated/typescript/`;
- the manifest fields that identify generator version, `wesley-core` version,
  source schema, source schema hash, generated outputs, and scalar profile
  witnesses;
- the Bunny-specific scalar profile interpretation layered on top of Wesley.

`wesley-core` remains profile-neutral. It lowers GraphQL SDL into a schema IR.
`bunny-wesley` owns the Bunny-specific decisions that turn that IR into
contract artifacts.

## Generation Boundary

`bunny-wesley` currently emits:

- a Rust source file with schema hash constants, generator witnesses, scalar type
  aliases, scalar-profile metadata, and DTO structs;
- a TypeScript source file with matching schema hash constants, generator
  witnesses, scalar type aliases, scalar-profile metadata, and DTO interfaces;
- a JSON manifest with generator, `wesley-core`, schema, schema hash, output
  paths, and scalar-profile metadata.

The generator writes only the configured output paths. It creates parent
directories for those outputs when needed.

## DTO Shape

Generated DTOs are boundary shapes:

- Rust DTOs derive `Clone`, `Debug`, and `PartialEq`.
- Rust nullable fields become `Option<T>`.
- Rust lists become `Vec<T>`, with nullable list items represented as
  `Vec<Option<T>>`.
- TypeScript nullable fields become `T | null`.
- TypeScript lists become `T[]`, with nullable list items represented as
  `(T | null)[]`.
- Built-in GraphQL scalar mappings are `String`/`ID` to Rust `String` and
  TypeScript `string`, `Int` to Rust `i32` and TypeScript `number`, `Float` to
  Rust `f64` and TypeScript `number`, and `Boolean` to Rust `bool` and
  TypeScript `boolean`.

Only object types whose names start with `Bunny` are emitted as DTOs. This keeps
the generated contract surface Bunny-owned even when a schema contains auxiliary
or external object definitions.

Schema type names must not collide with generated helper type names. The current
reserved helper type name is `BunnyScalarProfile`, which backs generated
scalar-profile witness arrays in Rust and TypeScript.

## Determinism

The generator stabilizes artifact identity through:

- SHA-256 hashing of the source schema contents;
- explicit generator and `wesley-core` version witnesses;
- lexicographic ordering for emitted scalar aliases;
- lexicographic ordering for emitted object DTOs;
- lexicographic ordering for emitted scalar-profile witnesses;
- checked-in generated artifacts with regression tests.

The schema lowering order remains a `wesley-core` concern. Bunny generator output
does not use time, randomness, network state, or environment-derived ordering.

## Scalar Profiles

Custom scalars are resolved through the Bunny scalar profile registry described
in [`../deterministic-contract-profile/`](../deterministic-contract-profile/).

Generation fails closed when a custom scalar lacks a supported
`@bunnyScalarProfile` directive. The directive is currently consumed on scalar
definitions. Although the schema declares `FIELD_DEFINITION` as an allowed
directive location, field-level scalar-profile overrides are reserved behavior
and fail generation until that semantics exists.

## Open Gaps

The current generator deliberately stops at DTOs and witnesses. The remaining
gaps are:

- byte reader and writer emitters;
- cross-language golden byte vectors for emitted codecs;
- explicit maximum-bound parameters for bounded byte and UTF-8 profiles;
- field-level scalar-profile override semantics;
- runtime constructor handoff tests in downstream consumers;
- richer manifest schema validation beyond current string witness checks.

## Documentation Rule

This topic folder is the current reference for Bunny contract generation.
Historical design notes and GitHub issues may explain why the generator changed,
but they must not become competing current references.
