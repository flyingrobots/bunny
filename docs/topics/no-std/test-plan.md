# `no_std` Test Plan

This plan defines how Bunny verifies that `bunny-num` builds without `std`
(flyingrobots/bunny#198). The current chapter is [`README.md`](README.md).

## Scope

It covers the `std` feature of `bunny-num`, its build with default features
off, and the Code Dojo gate that proves it. Other Bunny crates still require
`std` and are out of scope.

## Test Goals

- Prove that `bunny-num` builds with default features off for a target that
  has no `std`, for `wasm32-unknown-unknown` and natively.
- Prove that the gate fails when a `std` use comes back.
- Prove that the default build is unchanged: the same public items, the same
  tests and golden vectors passing, the same bits.

## Requirements

| ID | Requirement | Current Source |
| --- | --- | --- |
| NOSTD-REQ-001 | `cargo build -p bunny-num --no-default-features` succeeds natively, for `wasm32-unknown-unknown` and for `thumbv7em-none-eabihf`, a target with no `std`; the library's Clippy and tests pass in that configuration. Code Dojo's full gate runs these builds and fails if any of them fails. | `README.md#the-gate` |
| NOSTD-REQ-002 | The default build of `bunny-num` (the `std` feature on) exposes the same public items, with the same trait implementations, as before the feature existed, and every existing test and golden vector passes unchanged. | `README.md#features` |

```toml
[[requirement]]
id = "NOSTD-REQ-001"
summary = "cargo build -p bunny-num --no-default-features succeeds natively, for wasm32-unknown-unknown and for thumbv7em-none-eabihf, a target with no std; the library's Clippy and tests pass in that configuration. Code Dojo's full gate runs these builds and fails if any of them fails."
status = "active"

[[requirement]]
id = "NOSTD-REQ-002"
summary = "The default build of bunny-num (the std feature on) exposes the same public items, with the same trait implementations, as before the feature existed, and every existing test and golden vector passes unchanged."
status = "active"
```

## Test Cases

| ID | Category | Requirements | Oracle | Test |
| --- | --- | --- | --- | --- |
| NOSTD-TP-001 | Gate shape | NOSTD-REQ-001 | The gate installs `thumbv7em-none-eabihf` and `wasm32-unknown-unknown`, then builds `bunny-num` for each and natively, lints and tests it; every Cargo command selects only `bunny-num`, with `--no-default-features` and no feature flag. | `xtask/src/feature_configs.rs::tests::no_std_tp_001_bunny_num_is_built_without_std_for_a_bare_metal_target` |
| NOSTD-TP-002 | Build | NOSTD-REQ-001 | `cargo run --locked -p xtask -- code-dojo-no-std` exits 0. Before the change the `thumbv7em-none-eabihf` build fails with `can't find crate for std`. | `xtask/src/code_dojo.rs::handle_no_std` |
| NOSTD-TP-003 | Seeded regression | NOSTD-REQ-001 | Putting `std::ops` back in `fixed_q32_32.rs`, or `std::error::Error` back in `conversions.rs`, makes the build without default features fail. | [Seeded `std` uses](#seeded-std-uses) |
| NOSTD-TP-004 | Default API | NOSTD-REQ-002 | The rustdoc HTML of `bunny-num` with default features, built at the base and at the change, lists the same items and implementations. | [Default API audit](#default-api-audit) |
| NOSTD-TP-005 | Golden vectors | NOSTD-REQ-002 | `cargo test --locked --workspace --all-targets --all-features` passes with no test or golden value changed. | `crates/bunny-num/tests/determinism.rs` |

```toml
[[case]]
id = "NOSTD-TP-001"
requirements = ["NOSTD-REQ-001"]
evidence = "test"
test = "xtask/src/feature_configs.rs::tests::no_std_tp_001_bunny_num_is_built_without_std_for_a_bare_metal_target"
oracle = "The command list: the target install, then builds for thumbv7em-none-eabihf, wasm32-unknown-unknown and the host, Clippy and tests, each selecting only bunny-num with --no-default-features."
tier = "fast"
status = "implemented"

[[case]]
id = "NOSTD-TP-002"
requirements = ["NOSTD-REQ-001"]
evidence = "static-check"
artifact = "xtask/src/code_dojo.rs"
oracle = "code-dojo-no-std exits 0; before the change the thumbv7em-none-eabihf build fails with can't find crate for std."
tier = "slow"
status = "planned"

[[case]]
id = "NOSTD-TP-003"
requirements = ["NOSTD-REQ-001"]
evidence = "manual-audit"
artifact = "docs/topics/no-std/test-plan.md#seeded-std-uses"
oracle = "Each seeded std use makes the build without default features fail."
tier = "manual"
status = "planned"

[[case]]
id = "NOSTD-TP-004"
requirements = ["NOSTD-REQ-002"]
evidence = "manual-audit"
artifact = "docs/topics/no-std/test-plan.md#default-api-audit"
oracle = "The rustdoc HTML of the default build at the base and at the change lists the same items and implementations."
tier = "manual"
status = "planned"

[[case]]
id = "NOSTD-TP-005"
requirements = ["NOSTD-REQ-002"]
evidence = "static-check"
artifact = "crates/bunny-num/tests/determinism.rs"
oracle = "The workspace tests, golden vectors included, pass with no test or golden value changed."
tier = "fast"
status = "planned"
```

## Seeded `std` Uses

Planned: restore one `std` path at a time, build `bunny-num` with default
features off, and record the error.

## Default API Audit

Planned: build `cargo doc --no-deps -p bunny-num` with default features at the
base and at the change and compare the generated HTML.
