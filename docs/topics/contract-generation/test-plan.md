# Contract Generation Test Plan

This plan defines how Bunny proves the schema-to-artifact generation surface.

The current generation chapter is [`README.md`](README.md). This test plan is a
living verification design, not a historical proposal.

## Scope

This plan covers `bunny-wesley` generation of Rust DTOs, TypeScript DTOs,
manifest witnesses, scalar mappings, Bunny-owned object filtering, and checked-in
generated artifact version/profile witnesses.

Byte reader and writer functions are outside the current executable surface.
They must add byte-vector evidence when implemented.

## Test Goals

- Prove generated artifacts carry stable generator, `wesley-core`, schema hash,
  and output-path witnesses.
- Prove scalar and object output ordering is deterministic.
- Prove nullable fields and lists map to the documented Rust and TypeScript
  boundary shapes.
- Prove only Bunny-prefixed object types are emitted as DTOs.
- Prove custom scalar profiles are consumed through Bunny's closed profile
  registry and exposed in generated artifacts.
- Prove unsupported custom scalar profiles fail generation.

## Non-Goals

- This plan does not prove byte encoding or decoding yet.
- This plan does not test downstream runtime constructors.
- This plan does not make `wesley-core` responsible for Bunny profile semantics.

## Requirements

The table is the human-readable view. The fenced TOML block immediately after
it is the contract graph consumed by `cargo run --locked -p xtask --
topic-docs`. Tooling reads only the structured block, not visual Markdown table
formatting.

| ID | Requirement | Current Source |
| --- | --- | --- |
| CG-REQ-001 | The generator emits Rust, TypeScript, and manifest artifacts with generator, Wesley, schema, output, and schema-hash witnesses. | `README.md#generation-boundary` |
| CG-REQ-002 | Nullable fields and lists map to documented Rust and TypeScript boundary shapes. | `README.md#dto-shape` |
| CG-REQ-003 | Only object types whose names start with `Bunny` are emitted as DTOs. | `README.md#dto-shape` |
| CG-REQ-004 | Scalar aliases and scalar-profile witnesses are ordered deterministically by scalar name. | `README.md#determinism` |
| CG-REQ-005 | Object DTOs are ordered deterministically by object name. | `README.md#determinism` |
| CG-REQ-006 | Checked-in generated artifacts match the released `bunny-wesley` generator version. | `README.md#generation-boundary` |
| CG-REQ-007 | Checked-in generated artifacts expose scalar-profile witnesses. | `README.md#scalar-profiles` |
| CG-REQ-008 | Unsupported or missing custom scalar profiles fail generation. | `README.md#scalar-profiles` |
| CG-REQ-009 | Schema type names that collide with generated helper type names fail generation. | `README.md#dto-shape` |

```toml
[[requirement]]
id = "CG-REQ-001"
summary = "The generator emits Rust, TypeScript, and manifest artifacts with generator, Wesley, schema, output, and schema-hash witnesses."
status = "active"

[[requirement]]
id = "CG-REQ-002"
summary = "Nullable fields and lists map to documented Rust and TypeScript boundary shapes."
status = "active"

[[requirement]]
id = "CG-REQ-003"
summary = "Only object types whose names start with Bunny are emitted as DTOs."
status = "active"

[[requirement]]
id = "CG-REQ-004"
summary = "Scalar aliases and scalar-profile witnesses are ordered deterministically by scalar name."
status = "active"

[[requirement]]
id = "CG-REQ-005"
summary = "Object DTOs are ordered deterministically by object name."
status = "active"

[[requirement]]
id = "CG-REQ-006"
summary = "Checked-in generated artifacts match the released bunny-wesley generator version."
status = "active"

[[requirement]]
id = "CG-REQ-007"
summary = "Checked-in generated artifacts expose scalar-profile witnesses."
status = "active"

[[requirement]]
id = "CG-REQ-008"
summary = "Unsupported or missing custom scalar profiles fail generation."
status = "active"

[[requirement]]
id = "CG-REQ-009"
summary = "Schema type names that collide with generated helper type names fail generation."
status = "active"
```

## Fixtures

The current generator tests use inline GraphQL schemas for small behavior
fixtures. The checked-in graphics contract artifacts are integration fixtures:

- `crates/bunny-contract/src/generated/graphics.rs`
- `generated/typescript/bunny-graphics.ts`
- `generated/bunny-graphics.manifest.json`

Regenerate them with:

```sh
cargo run --locked -p xtask -- generate
```

## Test Cases

| ID | Category | Requirements | Oracle | Test |
| --- | --- | --- | --- | --- |
| CG-TP-001 | Type mapping | CG-REQ-002 | Exact Rust and TypeScript outputs for nullable scalar and non-null list references. | `crates/bunny-wesley/src/render.rs::tests::maps_nullable_scalar_and_non_null_list` |
| CG-TP-002 | Lowering and built-in scalars | CG-REQ-002 | Lowered schema order and GraphQL `Int` nullable field mapping are stable. | `crates/bunny-wesley/src/render.rs::tests::test_lower_schema_sdl_preserves_type_order` |
| CG-TP-003 | Rust and TypeScript witnesses | CG-REQ-001 | Rendered Rust and TypeScript artifacts include source, schema hash, generator, and Wesley witnesses. | `crates/bunny-wesley/src/render.rs::tests::rendered_artifacts_include_schema_and_generator_witnesses` |
| CG-TP-004 | Manifest witnesses | CG-REQ-001, CG-REQ-007 | Rendered manifest includes generator, Wesley, schema, schema-hash, output-path, and scalar-profile witnesses. | `crates/bunny-wesley/src/main.rs::tests::manifest_records_generation_witnesses_and_scalar_profiles` |
| CG-TP-005 | Scalar profile mapping | CG-REQ-004, CG-REQ-008 | Supported scalar directives resolve to exact Rust and TypeScript types. | `crates/bunny-wesley/src/render.rs::tests::test_directive_scalar_mapping` |
| CG-TP-006 | Bunny object boundary | CG-REQ-003, CG-REQ-005 | Rendered Rust and TypeScript artifacts include `Bunny*` DTOs in name order and exclude non-Bunny DTOs. | `crates/bunny-wesley/src/render.rs::tests::render_only_emits_bunny_prefixed_object_dtos_in_name_order` |
| CG-TP-007 | Reserved generated names | CG-REQ-009 | Rendering rejects a schema type named `BunnyScalarProfile`. | `crates/bunny-wesley/src/render.rs::tests::render_rejects_schema_type_names_reserved_for_generated_helpers` |
| CG-TP-008 | Missing profile fail-closed path | CG-REQ-008 | Rendering rejects a custom scalar without `@bunnyScalarProfile`. | `crates/bunny-wesley/src/render.rs::tests::render_rejects_custom_scalars_without_profiles` |
| CG-TP-009 | Unsupported profile fail-closed path | CG-REQ-008 | Unsupported profile names return scalar type resolution errors. | `crates/bunny-wesley/src/render.rs::tests::test_invalid_directive_profile_errors` |
| CG-TP-010 | Profile witness rendering | CG-REQ-004, CG-REQ-007 | Exact rendered Rust, TypeScript, and manifest witness substrings for an envelope-shaped schema. | `crates/bunny-wesley/src/profile.rs::tests::deterministic_contract_profiles_render_for_wire_envelopes` |
| CG-TP-011 | Generated version witnesses | CG-REQ-001, CG-REQ-006 | Checked-in Rust, TypeScript, and manifest artifacts name the released generator version. | `crates/bunny-contract/tests/generated_version_tests.rs::generated_witnesses_match_the_released_generator_version` |
| CG-TP-012 | Generated scalar-profile witnesses | CG-REQ-004, CG-REQ-007 | Checked-in Rust, TypeScript, and manifest artifacts expose matching scalar-profile witnesses. | `crates/bunny-contract/tests/generated_version_tests.rs::generated_scalar_profile_witnesses_cover_checked_in_artifacts` |

```toml
[[case]]
id = "CG-TP-001"
requirements = ["CG-REQ-002"]
evidence = "test"
test = "crates/bunny-wesley/src/render.rs::tests::maps_nullable_scalar_and_non_null_list"
oracle = "Exact Rust and TypeScript outputs for nullable scalar and non-null list references."
tier = "fast"
status = "implemented"

[[case]]
id = "CG-TP-002"
requirements = ["CG-REQ-002"]
evidence = "test"
test = "crates/bunny-wesley/src/render.rs::tests::test_lower_schema_sdl_preserves_type_order"
oracle = "Lowered schema order and GraphQL Int nullable field mapping are stable."
tier = "fast"
status = "implemented"

[[case]]
id = "CG-TP-003"
requirements = ["CG-REQ-001"]
evidence = "test"
test = "crates/bunny-wesley/src/render.rs::tests::rendered_artifacts_include_schema_and_generator_witnesses"
oracle = "Rendered Rust and TypeScript artifacts include source, schema hash, generator, and Wesley witnesses."
tier = "fast"
status = "implemented"

[[case]]
id = "CG-TP-004"
requirements = ["CG-REQ-001", "CG-REQ-007"]
evidence = "test"
test = "crates/bunny-wesley/src/main.rs::tests::manifest_records_generation_witnesses_and_scalar_profiles"
oracle = "Rendered manifest includes generator, Wesley, schema, schema-hash, output-path, and scalar-profile witnesses."
tier = "fast"
status = "implemented"

[[case]]
id = "CG-TP-005"
requirements = ["CG-REQ-004", "CG-REQ-008"]
evidence = "test"
test = "crates/bunny-wesley/src/render.rs::tests::test_directive_scalar_mapping"
oracle = "Supported scalar directives resolve to exact Rust and TypeScript types."
tier = "fast"
status = "implemented"

[[case]]
id = "CG-TP-006"
requirements = ["CG-REQ-003", "CG-REQ-005"]
evidence = "test"
test = "crates/bunny-wesley/src/render.rs::tests::render_only_emits_bunny_prefixed_object_dtos_in_name_order"
oracle = "Rendered Rust and TypeScript artifacts include Bunny-prefixed DTOs in name order and exclude non-Bunny DTOs."
tier = "fast"
status = "implemented"

[[case]]
id = "CG-TP-007"
requirements = ["CG-REQ-009"]
evidence = "test"
test = "crates/bunny-wesley/src/render.rs::tests::render_rejects_schema_type_names_reserved_for_generated_helpers"
oracle = "Rendering rejects a schema type named BunnyScalarProfile."
tier = "fast"
status = "implemented"

[[case]]
id = "CG-TP-008"
requirements = ["CG-REQ-008"]
evidence = "test"
test = "crates/bunny-wesley/src/render.rs::tests::render_rejects_custom_scalars_without_profiles"
oracle = "Rendering rejects a custom scalar without @bunnyScalarProfile."
tier = "fast"
status = "implemented"

[[case]]
id = "CG-TP-009"
requirements = ["CG-REQ-008"]
evidence = "test"
test = "crates/bunny-wesley/src/render.rs::tests::test_invalid_directive_profile_errors"
oracle = "Unsupported profile names return scalar type resolution errors."
tier = "fast"
status = "implemented"

[[case]]
id = "CG-TP-010"
requirements = ["CG-REQ-004", "CG-REQ-007"]
evidence = "test"
test = "crates/bunny-wesley/src/profile.rs::tests::deterministic_contract_profiles_render_for_wire_envelopes"
oracle = "Exact rendered Rust, TypeScript, and manifest witness substrings for an envelope-shaped schema."
tier = "fast"
status = "implemented"

[[case]]
id = "CG-TP-011"
requirements = ["CG-REQ-001", "CG-REQ-006"]
evidence = "test"
test = "crates/bunny-contract/tests/generated_version_tests.rs::generated_witnesses_match_the_released_generator_version"
oracle = "Checked-in Rust, TypeScript, and manifest artifacts name the released generator version."
tier = "fast"
status = "implemented"

[[case]]
id = "CG-TP-012"
requirements = ["CG-REQ-004", "CG-REQ-007"]
evidence = "test"
test = "crates/bunny-contract/tests/generated_version_tests.rs::generated_scalar_profile_witnesses_cover_checked_in_artifacts"
oracle = "Checked-in Rust, TypeScript, and manifest artifacts expose matching scalar-profile witnesses."
tier = "fast"
status = "implemented"
```

## Determinism Obligations And Evidence

The tests assert stable generated strings, schema hashes, generator witnesses,
scalar profile witness contents, and checked-in generated artifacts. They do not
use time, randomness, network access, stdout, stderr, logs, or documentation
prose as an oracle.

## Known Failures

There are no known failing contract-generation cases in the current executable
surface.

## Edge Cases And Unusual Inputs

Current tests cover:

- nullable scalar fields;
- non-null list fields;
- GraphQL built-in scalar mapping;
- supported custom scalar profiles;
- missing custom scalar profiles;
- unsupported custom scalar profile names;
- reserved generated helper type name collisions;
- Bunny-prefixed and non-Bunny object names;
- checked-in generated artifact witness metadata.

Future tests must cover:

- field-level scalar-profile override semantics if implemented;
- explicit maximum-bound parameters for bounded bytes and strings;
- byte reader and writer golden vectors;
- manifest schema validation beyond current witness-string checks.

## Stress And Fuzz

No fuzz target exists for contract generation today. Fuzzing becomes useful after
the generator emits byte readers and writers or accepts a broader schema grammar.
Every minimized fuzz failure must become a committed deterministic regression
case with a stable test name and, when appropriate, a golden byte vector.

## Open Gaps

| Gap | Blocking API |
| --- | --- |
| LE-binary byte reader and writer evidence. | Codec emitters. |
| Field-level scalar-profile override evidence. | Field-level profile semantics. |
| Explicit maximum bounds for variable bytes and strings. | Profile arguments or named bounded profiles. |
| Strong manifest schema validation. | Published manifest schema or typed manifest reader. |
