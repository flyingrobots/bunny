# `no_std` Test Plan

This plan defines how Bunny verifies that `bunny-num` (flyingrobots/bunny#198)
and `bunny-linalg`, `bunny-geom` and `bunny-query` (flyingrobots/bunny#156)
build without `std`. The current chapter is [`README.md`](README.md).

## Scope

It covers the `std` feature of `bunny-num`, `bunny-linalg`, `bunny-geom` and
`bunny-query`, their builds with default features off, the audit of every
`std` use those three crates had, and the Code Dojo gate that proves it. Other
Bunny crates still require `std` and are out of scope.

## Test Goals

- Prove that `bunny-num`, `bunny-linalg`, `bunny-geom` and `bunny-query` each
  build with default features off for a target that has no `std`, for
  `wasm32-unknown-unknown` and natively.
- Prove that the gate fails when a `std` use comes back.
- Prove that the default build is unchanged: the same public items, the same
  tests and golden vectors passing, the same bits.

## Requirements

| ID | Requirement | Current Source |
| --- | --- | --- |
| NOSTD-REQ-001 | `cargo build -p bunny-num --no-default-features` succeeds natively, for `wasm32-unknown-unknown` and for `thumbv7em-none-eabihf`, a target with no `std`; the library's Clippy and tests pass in that configuration. Code Dojo's full gate runs these builds and fails if any of them fails. | `README.md#the-gate` |
| NOSTD-REQ-002 | The default build of `bunny-num` (the `std` feature on) exposes the same public items, with the same trait implementations, as before the feature existed, and every existing test and golden vector passes unchanged. | `README.md#features` |
| NOSTD-REQ-010 | `cargo build --no-default-features` of `bunny-linalg`, `bunny-geom` and `bunny-query`, one package at a time, succeeds natively, for `wasm32-unknown-unknown` and for `thumbv7em-none-eabihf`; each library's Clippy and tests pass in that configuration. Code Dojo's full gate runs these builds and fails if any of them fails. | `README.md#the-gate` |
| NOSTD-REQ-011 | The default builds of `bunny-linalg`, `bunny-geom` and `bunny-query` expose the same public items, with the same trait implementations, as before their `std` feature existed, and every existing test and golden vector passes unchanged. | `README.md#features` |
| NOSTD-REQ-012 | Each of the three crates depends on its Bunny dependencies with `default-features = false`, and its `std` feature turns on theirs, so the default build links the same `std` features as before and a build with default features off links no `std` through a dependency. | `README.md#features` |
| NOSTD-REQ-013 | The chapter lists every `std` use the three crates had, by file and line, what it needed, whether it was public, and the `core` item that replaced it; and states that none of the four crates needs `alloc`. | `README.md#audit` |

```toml
[[requirement]]
id = "NOSTD-REQ-001"
summary = "cargo build -p bunny-num --no-default-features succeeds natively, for wasm32-unknown-unknown and for thumbv7em-none-eabihf, a target with no std; the library's Clippy and tests pass in that configuration. Code Dojo's full gate runs these builds and fails if any of them fails."
status = "active"

[[requirement]]
id = "NOSTD-REQ-002"
summary = "The default build of bunny-num (the std feature on) exposes the same public items, with the same trait implementations, as before the feature existed, and every existing test and golden vector passes unchanged."
status = "active"

[[requirement]]
id = "NOSTD-REQ-010"
summary = "cargo build --no-default-features of bunny-linalg, bunny-geom and bunny-query, one package at a time, succeeds natively, for wasm32-unknown-unknown and for thumbv7em-none-eabihf; each library's Clippy and tests pass in that configuration. Code Dojo's full gate runs these builds and fails if any of them fails."
status = "active"

[[requirement]]
id = "NOSTD-REQ-011"
summary = "The default builds of bunny-linalg, bunny-geom and bunny-query expose the same public items, with the same trait implementations, as before their std feature existed, and every existing test and golden vector passes unchanged."
status = "active"

[[requirement]]
id = "NOSTD-REQ-012"
summary = "Each of the three crates depends on its Bunny dependencies with default-features = false, and its std feature turns on theirs, so the default build links the same std features as before and a build with default features off links no std through a dependency."
status = "active"

[[requirement]]
id = "NOSTD-REQ-013"
summary = "The chapter lists every std use the three crates had, by file and line, what it needed, whether it was public, and the core item that replaced it; and states that none of the four crates needs alloc."
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
| NOSTD-TP-006 | Error types | NOSTD-REQ-002 | In the default build `FloatConversionError`, `DivisionByZero` and `EllipseDomainError` box into `Box<dyn std::error::Error>`, print their published messages and have no source. | `crates/bunny-num/tests/std_feature_tests.rs::no_std_tp_006_error_types_are_std_errors_with_their_messages` |
| NOSTD-TP-010 | Gate shape | NOSTD-REQ-010 | The stack gate runs `bunny-num`'s five commands for `bunny-linalg`, `bunny-geom` and `bunny-query` in dependency order, each command selecting one package with `--no-default-features` and no feature flag. | `xtask/src/feature_configs.rs::tests::no_std_tp_010_linalg_geom_and_query_are_built_without_std_one_package_at_a_time` |
| NOSTD-TP-011 | Build | NOSTD-REQ-010, NOSTD-REQ-012 | `cargo run --locked -p xtask -- code-dojo-no-std` exits 0. Before the change the `thumbv7em-none-eabihf` build of `bunny-linalg` fails, because it pulled `bunny-num` with `std` on. | `xtask/src/code_dojo.rs::handle_no_std` |
| NOSTD-TP-012 | Seeded regression | NOSTD-REQ-010 | Putting a `std` path back in each crate, or dropping `default-features = false` from a Bunny dependency line, makes the build without default features fail. | [Seeded `std` uses in the stack](#seeded-std-uses-in-the-stack) |
| NOSTD-TP-013 | Default API | NOSTD-REQ-011 | The rustdoc HTML of the three crates with default features, built at the base and at the change, lists the same items and implementations. | [Default API audit of the stack](#default-api-audit-of-the-stack) |
| NOSTD-TP-014 | Golden vectors | NOSTD-REQ-011 | `cargo test --locked --workspace --all-targets --all-features` passes with no test or golden value changed. | `crates/bunny-query/tests/determinism.rs` |
| NOSTD-TP-015 | Error types | NOSTD-REQ-011 | In the default build each crate's public error types box into `Box<dyn std::error::Error>`, print their published messages and have no source. | `crates/bunny-linalg/tests/std_feature_tests.rs::no_std_tp_015_linalg_error_types_are_std_errors_with_their_messages`, `crates/bunny-geom/tests/std_feature_tests.rs::no_std_tp_015_geom_error_types_are_std_errors_with_their_messages`, `crates/bunny-query/tests/std_feature_tests.rs::no_std_tp_015_query_error_types_are_std_errors_with_their_messages` |
| NOSTD-TP-016 | Feature wiring | NOSTD-REQ-012 | `cargo tree -e features` of `bunny-query` lists `bunny-num feature "std"` with default features and no `std` feature of any Bunny crate without them. | [Feature wiring](#feature-wiring) |
| NOSTD-TP-017 | Audit | NOSTD-REQ-013 | Every `std` match of `git grep -n std` in the three crates' `src` at the base appears in the audit tables. | [`README.md#audit`](README.md#audit) |

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
status = "implemented"

[[case]]
id = "NOSTD-TP-003"
requirements = ["NOSTD-REQ-001"]
evidence = "manual-audit"
artifact = "docs/topics/no-std/test-plan.md#seeded-std-uses"
oracle = "Each seeded std use makes the build without default features fail."
tier = "manual"
status = "implemented"

[[case]]
id = "NOSTD-TP-004"
requirements = ["NOSTD-REQ-002"]
evidence = "manual-audit"
artifact = "docs/topics/no-std/test-plan.md#default-api-audit"
oracle = "The rustdoc HTML of the default build at the base and at the change lists the same items and implementations."
tier = "manual"
status = "implemented"

[[case]]
id = "NOSTD-TP-005"
requirements = ["NOSTD-REQ-002"]
evidence = "static-check"
artifact = "crates/bunny-num/tests/determinism.rs"
oracle = "The workspace tests, golden vectors included, pass with no test or golden value changed."
tier = "fast"
status = "implemented"

[[case]]
id = "NOSTD-TP-006"
requirements = ["NOSTD-REQ-002"]
evidence = "test"
test = "crates/bunny-num/tests/std_feature_tests.rs::no_std_tp_006_error_types_are_std_errors_with_their_messages"
oracle = "The three error types box into Box<dyn std::error::Error>, print their published messages and have no source."
tier = "fast"
status = "implemented"

[[case]]
id = "NOSTD-TP-010"
requirements = ["NOSTD-REQ-010"]
evidence = "test"
test = "xtask/src/feature_configs.rs::tests::no_std_tp_010_linalg_geom_and_query_are_built_without_std_one_package_at_a_time"
oracle = "The stack command list equals bunny-num's five commands with the package replaced by bunny-linalg, bunny-geom and bunny-query in that order, one package per command, --no-default-features and no feature flag."
tier = "fast"
status = "implemented"

[[case]]
id = "NOSTD-TP-011"
requirements = ["NOSTD-REQ-010", "NOSTD-REQ-012"]
evidence = "static-check"
artifact = "xtask/src/code_dojo.rs"
oracle = "code-dojo-no-std exits 0; before the change the thumbv7em-none-eabihf build of bunny-linalg fails because bunny-num is built with std."
tier = "slow"
status = "planned"

[[case]]
id = "NOSTD-TP-012"
requirements = ["NOSTD-REQ-010"]
evidence = "manual-audit"
artifact = "docs/topics/no-std/test-plan.md#seeded-std-uses-in-the-stack"
oracle = "Each seeded std use, and a Bunny dependency line without default-features = false, makes the build without default features fail."
tier = "manual"
status = "planned"

[[case]]
id = "NOSTD-TP-013"
requirements = ["NOSTD-REQ-011"]
evidence = "manual-audit"
artifact = "docs/topics/no-std/test-plan.md#default-api-audit-of-the-stack"
oracle = "The rustdoc HTML of the three crates' default builds at the base and at the change lists the same items and implementations."
tier = "manual"
status = "planned"

[[case]]
id = "NOSTD-TP-014"
requirements = ["NOSTD-REQ-011"]
evidence = "static-check"
artifact = "crates/bunny-query/tests/determinism.rs"
oracle = "The workspace tests, golden vectors included, pass with no test or golden value changed."
tier = "fast"
status = "planned"

[[case]]
id = "NOSTD-TP-015"
requirements = ["NOSTD-REQ-011"]
evidence = "test"
test = "crates/bunny-query/tests/std_feature_tests.rs::no_std_tp_015_query_error_types_are_std_errors_with_their_messages"
oracle = "One variant of each public error type of bunny-linalg, bunny-geom and bunny-query boxes into Box<dyn std::error::Error>, prints its published message and has no source; the bunny-linalg and bunny-geom tests of the same name check their crates."
tier = "fast"
status = "implemented"

[[case]]
id = "NOSTD-TP-016"
requirements = ["NOSTD-REQ-012"]
evidence = "manual-audit"
artifact = "docs/topics/no-std/test-plan.md#feature-wiring"
oracle = "cargo tree -e features of bunny-query lists bunny-num feature std with default features, and no std feature of a Bunny crate without them."
tier = "manual"
status = "planned"

[[case]]
id = "NOSTD-TP-017"
requirements = ["NOSTD-REQ-013"]
evidence = "manual-audit"
artifact = "docs/topics/no-std/README.md#audit"
oracle = "Every std match of git grep in the three crates' src at the base appears in the audit tables."
tier = "manual"
status = "planned"
```

## Seeded `std` Uses

One seed at a time was applied to the working tree, the build run, and the seed
restored. Native builds used the local 1.100.0 toolchain; the bare-metal build
used `RUSTC_BOOTSTRAP=1 cargo build -Zbuild-std=core`, because that toolchain
has no rustup to install the target. CI runs the gate itself on 1.96.0.

| Seed | Command (`-p bunny-num --no-default-features`) | Result |
| --- | --- | --- |
| none, at the first commit (the crate as on `main`) | `cargo build --target thumbv7em-none-eabihf` | fails: `can't find crate for std`, 68 errors |
| `use std::ops::{..}` in `fixed_q32_32.rs` | `cargo build` | fails: `cannot find module or crate std in this scope` |
| the same | `cargo build --target thumbv7em-none-eabihf` | fails: the same error |
| `impl std::error::Error` in `conversions.rs` | `cargo build` | fails: the same error |
| `vec![` for `std::vec![` in the `lib.rs` unit tests | `cargo test --lib` | fails: `cannot find macro vec in this scope` |

The last seed is the issue's known failure mode, a hidden `std` use in a test
helper: the crate's unit tests reach `std` only through `extern crate std`
under `cfg(all(test, not(feature = "std")))`.

## Default API Audit

`cargo doc --locked --no-deps -p bunny-num` with default features was built at
the base (`origin/main` at `b14d05e`) and at the change, and the two
`target/doc/bunny_num` trees compared with `diff -r` (38 HTML files each). After
replacing the `src/bunny_num/lib.rs.html#<lines>` source anchors, which moved
because `lib.rs` gained lines, the only difference is the new crate-level
paragraph on `index.html` describing the `std` feature. No file was added or
removed. The `FloatConversionError` page links its `Error` implementation to
`core/error/trait.Error.html` at the base and at the change alike: rustdoc
already rendered `impl std::error::Error` as the `core` trait, because `std`
re-exports it (`pub use core::error::Error;` in `library/std/src/error.rs`).
