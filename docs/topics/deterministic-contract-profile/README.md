# Deterministic Contract Profile

This topic defines Bunny's current deterministic contract profile surface.

The profile is the byte-level representation vocabulary that Bunny-owned
Wesley extensions attach to GraphQL scalars before emitting Rust, TypeScript,
manifests, and future codec artifacts. It is not a Continuum semantic family
and it is not a runtime model for downstream projects.

## Status

This is a living topic chapter. It describes behavior present in `HEAD`.

`bunny-wesley` currently emits DTOs plus scalar-profile witnesses. It does not
yet emit byte reader or writer functions. Codec emitters must consume these
profile witnesses instead of re-authoring the same scalar table by hand.

## Ownership

Bunny owns deterministic representation mechanics:

- scalar profile names
- Rust and TypeScript boundary representations
- wire-profile names
- fixed byte widths where the profile has one
- generated witness metadata
- regression tests that lock the profile mapping

Bunny does not own Continuum nouns such as receipts, witnessed suffix shells,
reading envelopes, or settlement plans. Continuum owns those shared family
semantics. Runtime repositories such as `git-warp` own their local domain
objects and must feed decoded boundary values into their own constructors.

## Authoring Model

Schemas opt into the profile with `@bunnyScalarProfile` on scalar definitions:

```graphql
directive @bunnyScalarProfile(name: String!) on SCALAR | FIELD_DEFINITION

scalar BunnyObjectId @bunnyScalarProfile(name: "bytes.fixed.20")
scalar BunnyCounter @bunnyScalarProfile(name: "u64")
```

`bunny-wesley` resolves custom scalars through a closed registry. A custom
scalar without a supported profile fails generation instead of silently
becoming a string or generic JSON value.

Although the schema declares `FIELD_DEFINITION` as an allowed directive
location, the current generator does not consume field-level scalar-profile
overrides. Field-level usage fails generation until that future semantics pass
exists.

## Current Profile Vocabulary

The current registry supports these profiles:

| Profile | Rust | TypeScript | Wire profile | Byte width |
| --- | --- | --- | --- | --- |
| `u8` | `u8` | `number` | `u8` | `1` |
| `u16` | `u16` | `number` | `u16-le` | `2` |
| `u32` | `u32` | `number` | `u32-le` | `4` |
| `u64` | `u64` | `bigint` | `u64-le` | `8` |
| `i32` | `i32` | `number` | `i32-le` | `4` |
| `i64` | `i64` | `bigint` | `i64-le` | `8` |
| `q32.32` | `i64` | `bigint` | `i64-le-q32.32` | `8` |
| `f32` | `f32` | `number` | `f32-le` | `4` |
| `bytes.fixed.20` | `[u8; 20]` | `Uint8Array` | `bytes-fixed-20` | `20` |
| `bytes.bounded.u32` | `Vec<u8>` | `Uint8Array` | `u32-len-bytes` | variable |
| `utf8.bounded.u32` | `String` | `string` | `u32-len-utf8` | variable |

Variable-width profiles use an explicit `u32` length prefix in the wire profile
name. A future profile may add a stricter maximum bound. Until then, runtime or
adapter code that needs a smaller bound must validate it at the boundary.

## Generated Witnesses

Generated Rust artifacts expose scalar metadata as
`BUNNY_GRAPHICS_SCALAR_PROFILES`. Generated TypeScript artifacts expose the same
metadata as `BUNNY_GRAPHICS_SCALAR_PROFILES`, and generated manifests include a
`scalarProfiles` array.

Those witnesses let downstream consumers verify which deterministic profile was
used without parsing generated source code or copying the registry.

## Envelope-Shaped Boundary Use

Versioned envelopes should model representation explicitly:

- version and kind fields use fixed-width integer profiles such as `u16`
- Git object identifiers can use `bytes.fixed.20`
- counters that exceed signed GraphQL `Int` use `u64`
- payload bytes use `bytes.bounded.u32`
- bounded labels or family names use `utf8.bounded.u32`

These are representation choices only. The schema that names a receipt, suffix,
optic, or reading envelope belongs to the semantic owner of that family.

## Open Gaps

The current implementation deliberately stops at profile metadata and DTO
surface generation. The remaining gaps are:

- byte reader and writer emitters
- explicit maximum-bound parameters for bounded bytes and strings
- canonical map or sorted-entry profile support
- cross-language golden byte vectors for emitted codecs
- field-level scalar-profile override semantics
- Continuum family adoption of these profiles
- runtime consumers proving decoded values enter validated domain constructors

## Documentation Rule

This topic folder is the current reference for Bunny's deterministic contract
profile. Historical design notes and GitHub issues may explain why the profile
changed, but they must not become competing current references.
