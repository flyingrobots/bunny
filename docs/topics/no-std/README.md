# `no_std`

This chapter says which Bunny crates build without `std`, behind which feature,
and how the gate proves it. The verification design is
[`test-plan.md`](test-plan.md). It answers flyingrobots/bunny#198.

## Status

Living topic chapter. Only `bunny-num` builds without `std`; every other Bunny
crate still requires it.

## Features

`bunny-num` has one feature, `std`, on by default.

| Build | Crate attribute | Needs | Public items |
| --- | --- | --- | --- |
| default (`std` on) | none | `std` | all |
| `default-features = false` | `#![no_std]` | `core` only, no `alloc` | all |

The `std` feature adds no item and changes no code path: every public item, the
`Display` and `core::error::Error` implementations of `FloatConversionError`,
`DivisionByZero` and `EllipseDomainError` included, exists in both builds and
returns the same bits. `std::error::Error` is a re-export of
`core::error::Error`, so in a `std` build the error types are `std` errors, as
they were before the feature existed. The crate has no dependencies, so turning
the feature off removes nothing from a dependency graph.

A `no_std` dependent writes:

```toml
[dependencies]
bunny-num = { version = "0.7", default-features = false }
```

The crate's tests, unit tests included, always link `std`.

## The Gate

`cargo run --locked -p xtask -- code-dojo-no-std`, also run by the full gate
(`code-dojo --all`, the CI Dojo job), installs `thumbv7em-none-eabihf` and
`wasm32-unknown-unknown` with `rustup target add`, then runs, each with
`--locked -p bunny-num --no-default-features`:

- `cargo build --target thumbv7em-none-eabihf`, a target that has no `std`, so
  a `std` path in the crate, or a dependency that links `std`, fails the build;
- `cargo build --target wasm32-unknown-unknown` and a native `cargo build`;
- `cargo clippy --all-targets -- -D warnings` and `cargo test`.

Each command selects only `bunny-num`, so no other workspace member turns `std`
back on through feature unification. The commands are
`xtask/src/feature_configs.rs::no_std_commands`.
