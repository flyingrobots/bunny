# 0003 — Bunny Q32.32 Numeric Profile

## Purpose

Bunny needs deterministic numeric profiles that can be shared by Echo, Geordi,
jedit/Jim, and future graphics-adjacent projects without making Echo the common
dependency.

This slice extracts the Q32.32 fixed-point conversion policy from Echo's
`warp-math` crate into Bunny.

## Contract

The Q32.32 representation is:

```text
raw: i64
real_value = raw / 2^32
```

Bunny owns:

- fractional-bit constant
- one-value raw constant
- deterministic `f32 -> i64` conversion
- deterministic `i64 -> f32` conversion
- NaN, infinity, rounding, and saturation policy

Echo may depend on Bunny and re-export or wrap this profile, but Echo should not
remain the long-term owner of the implementation.

## Conversion Policy

`from_f32` follows these rules:

- `NaN` maps to `0`.
- `+infinity` maps to `i64::MAX`.
- `-infinity` maps to `i64::MIN`.
- finite values are rounded to nearest at the Q32.32 boundary.
- ties round to even.
- out-of-range values saturate.

`to_f32` follows these rules:

- `0` maps to `+0.0`.
- values round to nearest at the `f32` boundary.
- ties round to even.

## Schema Profile

The SDL profile is:

```graphql
scalar BunnyFixedQ32_32Raw
  @bunnyScalarProfile(name: "q32.32")
  @bunnyInvariant(description: "Signed Q32.32 fixed-point raw i64.")
```

The generated Rust DTO maps this scalar to `i64`. The generated TypeScript DTO
maps it to `bigint`.

## Wesley Status

The current `bunny-wesley` crate is a Bunny-specific Wesley extension:

- `wesley-core` lowers Bunny SDL and computes the registry hash.
- Bunny maps the resulting Wesley IR into graphics-specific Rust and
  TypeScript DTOs.
- Generated witnesses record both the Bunny generator id and Wesley core
  version.

Bunny may later move more emitter logic to published Wesley emitter crates when
those emitters expose the required DTO hooks. Bunny still owns the SDL,
generated witnesses, and graphics-specific scalar mapping.

## Validation

Run:

```bash
bash scripts/generate-contracts.sh
cargo fmt --check --all
cargo test --workspace
npx --yes markdownlint-cli README.md docs/design/*.md
git diff --check
```
