# 0002 — Bunny Wesley Generation

## Purpose

Bunny owns shared graphics schemas and must prove that those schemas can drive
cross-language DTO generation without making Echo or Geordi the schema owner.

This slice establishes a minimal Bunny/Wesley generation path.

## Scope

In scope:

- parse `schemas/bunny/v0/graphics.graphql`
- emit Rust DTOs for `bunny-contract`
- emit TypeScript DTOs for downstream projects
- emit a manifest containing the schema SHA-256 hash
- keep generated behavior out of the generator

Out of scope:

- full Wesley plugin integration
- canonical binary codecs
- Echo or Geordi adapter crates
- geometry algorithms
- renderer or GPU behavior

## Contract

The authored schema remains the source:

```text
schemas/bunny/v0/graphics.graphql
```

The generator emits witnesses:

```text
crates/bunny-contract/src/generated/graphics.rs
generated/typescript/bunny-graphics.ts
generated/bunny-graphics.manifest.json
```

The manifest records:

- generator id
- schema path
- schema SHA-256 hash
- generated output paths

## Invariants

- Generated DTOs are data contracts only.
- Bunny core behavior stays handwritten and tested.
- Field order follows schema order.
- Nullable GraphQL fields become nullable TypeScript fields and `Option<T>` in
  Rust.
- Bunny schemas remain Bunny-owned even when downstream projects generate
  bindings from them.

## Validation

Run:

```bash
bash scripts/generate-contracts.sh
cargo fmt --check --all
cargo test --workspace
npx --yes markdownlint-cli README.md docs/design/*.md
git diff --check
```
