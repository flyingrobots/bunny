# Deterministic Contract Profile Test Plan

This plan defines how Bunny proves the deterministic contract profile.

The current profile chapter is [`README.md`](README.md). This test plan is a
living verification design, not a historical proposal.

## Scope

This plan covers the `bunny-wesley` scalar profile registry, generated
Rust/TypeScript profile witnesses, generated manifest witnesses, and fail-closed
behavior for unsupported custom scalars.

Codec byte reader and writer functions are outside the current executable
surface. They must add golden byte-vector evidence when implemented.

## Test Goals

- Prove profile resolution is closed and registry-driven.
- Prove the registry covers the current deterministic scalar vocabulary.
- Prove generated Rust, TypeScript, and manifest outputs expose profile
  witnesses.
- Prove envelope-shaped schemas can model version, kind, object-id, payload,
  counter, and label fields without git-warp owning Bunny's registry.
- Prove unsupported custom scalars fail generation.
- Prove field-level scalar-profile overrides fail generation until field
  semantics exist.

## Non-Goals

- This plan does not prove full byte encoding or decoding yet.
- This plan does not test Continuum family semantics.
- This plan does not test git-warp runtime constructors.

## Requirements

The table is the human-readable view. The fenced TOML block immediately after
it is the contract graph consumed by `cargo run --locked -p xtask --
topic-docs`. Tooling reads only the structured block, not visual Markdown table
formatting.

| ID | Requirement | Current Source |
| --- | --- | --- |
| DCP-REQ-001 | Custom scalars resolve through the closed Bunny scalar-profile registry. | `README.md#authoring-model` |
| DCP-REQ-002 | The registry includes deterministic integer, byte, UTF-8, fixed-point, and boundary-float profiles. | `README.md#current-profile-vocabulary` |
| DCP-REQ-003 | Generated Rust artifacts expose scalar-profile witnesses. | `README.md#generated-witnesses` |
| DCP-REQ-004 | Generated TypeScript artifacts expose scalar-profile witnesses. | `README.md#generated-witnesses` |
| DCP-REQ-005 | Generated manifests expose scalar-profile witnesses. | `README.md#generated-witnesses` |
| DCP-REQ-006 | Unsupported custom scalars fail generation. | `README.md#authoring-model` |
| DCP-REQ-007 | Field-level scalar-profile directives fail generation until field semantics exist. | `README.md#authoring-model` |

```toml
[[requirement]]
id = "DCP-REQ-001"
summary = "Custom scalars resolve through the closed Bunny scalar-profile registry."
status = "active"

[[requirement]]
id = "DCP-REQ-002"
summary = "The registry includes deterministic integer, byte, UTF-8, fixed-point, and boundary-float profiles."
status = "active"

[[requirement]]
id = "DCP-REQ-003"
summary = "Generated Rust artifacts expose scalar-profile witnesses."
status = "active"

[[requirement]]
id = "DCP-REQ-004"
summary = "Generated TypeScript artifacts expose scalar-profile witnesses."
status = "active"

[[requirement]]
id = "DCP-REQ-005"
summary = "Generated manifests expose scalar-profile witnesses."
status = "active"

[[requirement]]
id = "DCP-REQ-006"
summary = "Unsupported custom scalars fail generation."
status = "active"

[[requirement]]
id = "DCP-REQ-007"
summary = "Field-level scalar-profile directives fail generation until field semantics exist."
status = "active"
```

## Fixtures

The current tests use inline GraphQL schemas as fixtures. They are intentionally
small enough for the expected Rust, TypeScript, and manifest witnesses to be
audited in the test itself.

Checked-in generated artifacts are also evidence:

- `crates/bunny-contract/src/generated/graphics.rs`
- `generated/typescript/bunny-graphics.ts`
- `generated/bunny-graphics.manifest.json`

## Test Cases

| ID | Category | Requirements | Oracle | Test |
| --- | --- | --- | --- | --- |
| DCP-TP-001 | Registry | DCP-REQ-001, DCP-REQ-002 | Exact registry metadata for supported and unsupported profile names. | `crates/bunny-wesley/src/profile.rs::tests::scalar_profiles_are_registry_driven` |
| DCP-TP-002 | Generation | DCP-REQ-002, DCP-REQ-003, DCP-REQ-004, DCP-REQ-005 | Exact rendered Rust, TypeScript, and manifest witness substrings for an envelope-shaped schema. | `crates/bunny-wesley/src/profile.rs::tests::deterministic_contract_profiles_render_for_wire_envelopes` |
| DCP-TP-003 | Generated artifacts | DCP-REQ-003, DCP-REQ-004, DCP-REQ-005 | Checked-in Rust, TypeScript, and manifest artifacts expose matching profile witnesses. | `crates/bunny-contract/tests/generated_version_tests.rs::generated_scalar_profile_witnesses_cover_checked_in_artifacts` |
| DCP-TP-004 | Fail closed | DCP-REQ-006 | Rendering rejects a custom scalar without `@bunnyScalarProfile`. | `crates/bunny-wesley/src/render.rs::tests::render_rejects_custom_scalars_without_profiles` |
| DCP-TP-005 | Reserved field placement | DCP-REQ-007 | Rust, TypeScript, and manifest rendering reject field-level `@bunnyScalarProfile`. | `crates/bunny-wesley/src/profile.rs::tests::field_level_scalar_profiles_fail_closed_until_supported` |

```toml
[[case]]
id = "DCP-TP-001"
requirements = ["DCP-REQ-001", "DCP-REQ-002"]
evidence = "test"
test = "crates/bunny-wesley/src/profile.rs::tests::scalar_profiles_are_registry_driven"
oracle = "Exact registry metadata for supported and unsupported profile names."
tier = "fast"
status = "implemented"

[[case]]
id = "DCP-TP-002"
requirements = ["DCP-REQ-002", "DCP-REQ-003", "DCP-REQ-004", "DCP-REQ-005"]
evidence = "test"
test = "crates/bunny-wesley/src/profile.rs::tests::deterministic_contract_profiles_render_for_wire_envelopes"
oracle = "Exact rendered Rust, TypeScript, and manifest witness substrings for an envelope-shaped schema."
tier = "fast"
status = "implemented"

[[case]]
id = "DCP-TP-003"
requirements = ["DCP-REQ-003", "DCP-REQ-004", "DCP-REQ-005"]
evidence = "test"
test = "crates/bunny-contract/tests/generated_version_tests.rs::generated_scalar_profile_witnesses_cover_checked_in_artifacts"
oracle = "Checked-in Rust, TypeScript, and manifest artifacts expose matching profile witnesses."
tier = "fast"
status = "implemented"

[[case]]
id = "DCP-TP-004"
requirements = ["DCP-REQ-006"]
evidence = "test"
test = "crates/bunny-wesley/src/render.rs::tests::render_rejects_custom_scalars_without_profiles"
oracle = "Rendering rejects a custom scalar without @bunnyScalarProfile."
tier = "fast"
status = "implemented"

[[case]]
id = "DCP-TP-005"
requirements = ["DCP-REQ-007"]
evidence = "test"
test = "crates/bunny-wesley/src/profile.rs::tests::field_level_scalar_profiles_fail_closed_until_supported"
oracle = "Rust, TypeScript, and manifest rendering reject field-level @bunnyScalarProfile."
tier = "fast"
status = "implemented"
```

## Determinism Obligations And Evidence

The tests assert exact profile names, Rust types, TypeScript types, wire-profile
names, and byte widths. They do not use time, randomness, environment variables,
filesystem-derived ordering, network access, stdout, stderr, or documentation
prose as an oracle.

Profile arrays are sorted by scalar name before rendering, so generated witness
order does not depend on schema parse order.

## Known Failures

There are no known failing deterministic contract-profile cases in the current
executable surface.

## Edge Cases And Unusual Inputs

Current tests cover:

- custom scalars with supported profile directives
- custom scalars without profile directives
- unsupported profile names
- field-level scalar-profile directives
- fixed-width integer profiles
- fixed-length bytes
- variable bytes
- bounded UTF-8 strings
- generated artifact witness metadata

Future tests must cover:

- explicit maximum-bound parameters
- zero-length byte and string payloads
- maximum-length byte and string payloads
- canonical map or sorted-entry profiles
- byte reader and writer golden vectors
- field-level scalar-profile override semantics

## Stress And Fuzz

No fuzz target exists for contract profiles today. Fuzzing becomes useful after
byte reader and writer emitters exist. Every minimized fuzz failure must become
a committed deterministic regression case with a stable test name and, when
appropriate, a golden byte vector.

## Open Gaps

| Gap | Blocking API |
| --- | --- |
| LE-binary byte reader and writer evidence. | Codec emitters. |
| Field-level scalar-profile override evidence. | Field-level profile semantics. |
| Explicit maximum bounds for variable bytes and strings. | Profile arguments or named bounded profiles. |
| Canonical map or sorted-entry support. | Collection profile design. |
| Cross-language golden byte vectors. | Codec emitters in at least two target languages. |
