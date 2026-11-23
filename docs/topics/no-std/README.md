# `no_std`

This chapter says which Bunny crates build without `std`, behind which feature,
what each one used from `std` before, and how the gate proves it. The
verification design is [`test-plan.md`](test-plan.md). It answers
flyingrobots/bunny#198 (`bunny-num`) and flyingrobots/bunny#156 (`bunny-linalg`,
`bunny-geom` and `bunny-query`).

## Status

Living topic chapter. `bunny-num`, `bunny-linalg`, `bunny-geom` and
`bunny-query` build without `std`; every other Bunny crate still requires it.
None of the four needs `alloc`.

## Features

Each of the four crates has one feature, `std`, on by default.

| Build | Crate attribute | Needs | Public items |
| --- | --- | --- | --- |
| default (`std` on) | none | `std` | all |
| `default-features = false` | `#![no_std]` | `core` only, no `alloc` | all |

The `std` feature adds no item and changes no code path: every public item, the
`Display` and `core::error::Error` implementations of the error types included,
exists in both builds and returns the same bits. `std::error::Error` is a
re-export of `core::error::Error`, so in a `std` build the error types are `std`
errors, as they were before the feature existed.

`bunny-num` has no dependencies. The other three depend on the Bunny crates
below them with `default-features = false`, and their `std` feature turns on
those crates' `std` features:

| Crate | Bunny dependencies | `std` turns on |
| --- | --- | --- |
| `bunny-num` | none | nothing else |
| `bunny-linalg` | `bunny-num` | `bunny-num/std` |
| `bunny-geom` | `bunny-linalg`, `bunny-num` | `bunny-linalg/std`, `bunny-num/std` |
| `bunny-query` | `bunny-geom`, `bunny-linalg`, `bunny-num` | `bunny-geom/std`, `bunny-linalg/std`, `bunny-num/std` |

So a default build links the same `std` features as before, and a build with
default features off reaches no `std` through a dependency. Every other
workspace member depends on these crates with default features, so the
workspace build is unchanged. A `no_std` dependent writes, for example:

```toml
[dependencies]
bunny-query = { version = "0.7", default-features = false }
```

The crates' tests, unit tests included, always link `std`.

## Audit

The tables list every `std` use in the `src` of `bunny-linalg`, `bunny-geom`
and `bunny-query` on `main` at `b306708`, before the change, by the line it
had then. Every use was an item that `core` has under the same name, so each
was replaced by its `core` path with nothing else changed. "Public" means the
use is part of the public API: a trait implementation on a public type, whose
trait is the same with either path.

None of the three crates' library code used a collection, `Vec`, `String`,
`Box`, `format!` or any other allocating item, so none needs `alloc`. None
called a float method that lives only in `std` (`sqrt`, `powi`, `floor`,
`abs` and the like on `f32` or `f64`): the float boundary types (`Vec2`,
`Vec3`, `Ray3`, `Aabb3`, `Sphere3`) only compare floats and convert them with
`bunny-num`'s `try_from_f32`, `from_f32`, `to_f32` and `is_finite`, which
need only `core`. The `abs`, `signum`, `unsigned_abs`, `div_euclid`,
`rem_euclid` and `sqrt` calls in these crates are on integers or on
`FixedQ32_32`, all `core`. The one unit-test module in library code
(`bunny-query`'s `trace/wide.rs`) uses only `core` macros.

### `bunny-linalg`

Verdict: practical with `core` substitutions only; adopted.

| File:line | Use | Needs | Public | Replaced by |
| --- | --- | --- | --- | --- |
| `catmull_rom3.rs:7` | `use std::fmt;` | `Display` for `CatmullRomError` | yes (the impl) | `core::fmt` |
| `catmull_rom3.rs:40` | `impl std::error::Error for CatmullRomError` | the error trait | yes | `core::error::Error` |
| `fixed_mat4_look_at.rs:15` | `use std::fmt;` | `Display` for `LookAtError` | yes (the impl) | `core::fmt` |
| `fixed_mat4_look_at.rs:47` | `impl std::error::Error for LookAtError` | the error trait | yes | `core::error::Error` |
| `fixed_mat4_projection.rs:8` | `use std::fmt;` | `Display` for `ProjectionError` | yes (the impl) | `core::fmt` |
| `fixed_mat4_projection.rs:39` | `impl std::error::Error for ProjectionError` | the error trait | yes | `core::error::Error` |
| `fixed_vec2.rs:3` | `use std::ops::{Add, .., SubAssign};` | the operator traits on `FixedVec2` | yes (the impls) | `core::ops` |
| `fixed_vec3.rs:3` | `use std::ops::{Add, .., SubAssign};` | the operator traits on `FixedVec3` | yes (the impls) | `core::ops` |
| `viewport.rs:10` | `use std::fmt;` | `Display` for `ViewportError` | yes (the impl) | `core::fmt` |
| `viewport.rs:36` | `impl std::error::Error for ViewportError` | the error trait | yes | `core::error::Error` |

Dependencies: `bunny-num` (`no_std` with default features off). Dev
dependency: `wasm-bindgen-test`, used only by the tests, which link `std`.

### `bunny-geom`

Verdict: practical with `core` substitutions only; adopted. `lib.rs` already
imported `core::fmt`.

| File:line | Use | Needs | Public | Replaced by |
| --- | --- | --- | --- | --- |
| `lib.rs:61` | `impl std::error::Error for GeomError` | the error trait | yes | `core::error::Error` |
| `plane/polygon.rs:44` | `impl std::error::Error for PolygonClipError` | the error trait | yes | `core::error::Error` |

Dependencies: `bunny-linalg` and `bunny-num` (both `no_std` with default
features off). Dev dependency: `wasm-bindgen-test`, tests only.

### `bunny-query`

Verdict: practical with `core` substitutions only; adopted.

| File:line | Use | Needs | Public | Replaced by |
| --- | --- | --- | --- | --- |
| `frame.rs:63` | `impl std::error::Error for ShapeFrameError` | the error trait | yes | `core::error::Error` |
| `ray.rs:3` | `use std::cmp::{max, min};` | `max` and `min` in the slab test | no | `core::cmp` |
| `ray/aabb_wide.rs:9` | `use std::cmp::Ordering;` | a private comparison | no | `core::cmp` |
| `trace/box_hit.rs:13` | `use std::cmp::Ordering;` | a private comparison | no | `core::cmp` |
| `trace/box_hit.rs:14` | `use std::fmt;` | `Display` for `TraceBoxError` | yes (the impl) | `core::fmt` |
| `trace/box_hit.rs:45` | `impl std::error::Error for TraceBoxError` | the error trait | yes | `core::error::Error` |
| `trace/box_hit.rs:126` | `std::array::from_fn` | a private unit normal | no | `core::array::from_fn` |
| `trace/cylinder.rs:19` | `use std::fmt;` | `Display` for `TraceCylinderError` | yes (the impl) | `core::fmt` |
| `trace/cylinder.rs:57` | `impl std::error::Error for TraceCylinderError` | the error trait | yes | `core::error::Error` |
| `trace/cylinder/span.rs:9` | `use std::cmp::Ordering;` | a private comparison | no | `core::cmp` |
| `trace/disc.rs:15` | `use std::fmt;` | `Display` for `TraceDiscError` | yes (the impl) | `core::fmt` |
| `trace/disc.rs:49` | `impl std::error::Error for TraceDiscError` | the error trait | yes | `core::error::Error` |
| `trace/sphere.rs:14` | `use std::fmt;` | `Display` for `TraceSphereError` | yes (the impl) | `core::fmt` |
| `trace/sphere.rs:48` | `impl std::error::Error for TraceSphereError` | the error trait | yes | `core::error::Error` |

Dependencies: `bunny-geom`, `bunny-linalg` and `bunny-num` (all `no_std` with
default features off). Dev dependency: `wasm-bindgen-test`, tests only.

## The Gate

`cargo run --locked -p xtask -- code-dojo-no-std`, also run by the full gate
(`code-dojo --all`, the CI Dojo job), installs `thumbv7em-none-eabihf` and
`wasm32-unknown-unknown` with `rustup target add`, then runs, for `bunny-num`,
`bunny-linalg`, `bunny-geom` and `bunny-query` in that order, each with
`--locked -p <crate> --no-default-features`:

- `cargo build --target thumbv7em-none-eabihf`, a target that has no `std`, so
  a `std` path in the crate, or a dependency that links `std`, fails the build;
- `cargo build --target wasm32-unknown-unknown` and a native `cargo build`;
- `cargo clippy --all-targets -- -D warnings` and `cargo test`.

Each command selects one crate, so no other workspace member turns `std` back
on through feature unification. The commands are
`xtask/src/feature_configs.rs::no_std_commands` (the target install and
`bunny-num`) and `no_std_stack_commands` (the other three).
