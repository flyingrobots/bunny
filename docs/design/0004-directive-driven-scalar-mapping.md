# Design RFC: 0004 — Directive-Driven Scalar Mapping

## Legend
COMPILER

## Description
This document describes the design for replacing hardcoded scalar profile name checks in `bunny-wesley` with dynamic directive-driven mapping based on the `@bunnyScalarProfile` AST directive.

---

## Goals
1. Parse and extract arguments from the `@bunnyScalarProfile` directive on GraphQL scalar types in the Wesley IR.
2. Replace hardcoded scalar names (`BunnyScalar`, `BunnyFixedQ32_32Raw`) in `bunny-wesley/src/main.rs` with dynamic mapping lookups driven by directive arguments.
3. Keep the code generator extensible so that new scalar profiles can be introduced in the schema without editing the compiler source code.

---

## Implementation Details

### 1. Schema Directive Expansion
We will update `schemas/bunny/v0/graphics.graphql` to attach the `@bunnyScalarProfile` directive to all custom scalars, including `BunnyScalar`:
```graphql
scalar BunnyScalar
  @bunnyScalarProfile(name: "f32")
  @bunnyInvariant(description: "Bunny scalar values must be finite under the active numeric profile.")
```

### 2. AST Extraction in Wesley IR
In `bunny-wesley/src/main.rs`, we will parse the `directives` IndexMap attached to each `TypeDefinition` of kind `TypeKind::Scalar`:
1. Find the directive named `"bunnyScalarProfile"`.
2. Extract the `"name"` argument value.
3. Map the profile name to corresponding target types:

| Profile Name | Rust Type | TypeScript Type |
| :--- | :--- | :--- |
| `"f32"` | `f32` | `number` |
| `"q32.32"` | `i64` | `bigint` |

If a custom scalar lacks the directive, it will fall back to `String` (Rust) and `unknown` (TypeScript).

---

## Verification Plan

### 1. Automated Tests
* Create unit tests in `crates/bunny-wesley/src/main.rs` (under the test module) verifying that:
  * An IR containing a scalar with `@bunnyScalarProfile(name: "q32.32")` correctly generates `i64` in Rust and `bigint` in TypeScript.
  * An IR containing a scalar with `@bunnyScalarProfile(name: "f32")` correctly generates `f32` in Rust and `number` in TypeScript.
  * A scalar without the directive fallback behaves correctly.

### 2. Integration
* Run `cargo run --bin xtask generate` to confirm DTO regeneration and check that the manifest SHA-256 remains valid and the compiler builds successfully.
