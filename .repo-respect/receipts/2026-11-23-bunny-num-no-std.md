# Receipt: Bunny Num No Std

Task:
Make `bunny-num` `no_std` compatible (flyingrobots/bunny#198): a default `std` feature, `#![cfg_attr(not(feature = "std"), no_std)]`, `core` paths in place of `std` paths, and a Code Dojo gate that builds the crate with default features off natively, for `wasm32-unknown-unknown` and for `thumbv7em-none-eabihf` (a target with no `std`). The default build keeps its public API and bits. The first commit adds the gate and the planned cases; the gate fails on `main`'s crate.

Files read:
- `CONTRIBUTING.md`
- `CODE_STANDARDS.md`
- `crates/bunny-num/Cargo.toml`
- `crates/bunny-num/README.md`
- `crates/bunny-num/src/lib.rs`
- `crates/bunny-num/src/fixed_q32_32.rs`
- `crates/bunny-num/src/fixed_q32_32/conversions.rs`
- `crates/bunny-num/src/u32_div_round_even.rs`
- `crates/bunny-num/src/u32_ellipse_classify.rs`
- `xtask/src/feature_configs.rs`
- `xtask/src/code_dojo.rs`
- `xtask/src/main.rs`
- `xtask/src/topic_docs.rs`
- `.github/workflows/code-dojo.yml`
- `rust-toolchain.toml`
- `docs/CODE_DOJO.md`
- `docs/README.md`
- `docs/topics/lint-policy/test-plan.md` (format)
- `docs/topics/ascii-ply/test-plan.md` (no-default-features gate)

Files edited:
- `crates/bunny-num/Cargo.toml`
- `crates/bunny-num/src/lib.rs`
- `crates/bunny-num/src/fixed_q32_32.rs`
- `crates/bunny-num/src/fixed_q32_32/conversions.rs`
- `crates/bunny-num/tests/std_feature_tests.rs`
- `crates/bunny-num/README.md`
- `docs/topics/no-std/README.md`
- `docs/README.md`
- `docs/CODE_DOJO.md`
- `CHANGELOG.md`
- `xtask/src/feature_configs.rs`
- `xtask/src/code_dojo.rs`
- `xtask/src/main.rs`
- `docs/topics/no-std/test-plan.md`
- `.repo-respect/receipts/2026-11-23-bunny-num-no-std.md`

Topic docs:
- `docs/topics/no-std/README.md`
- `docs/topics/no-std/test-plan.md` (NOSTD-REQ-001, NOSTD-REQ-002, NOSTD-TP-001 to NOSTD-TP-006)

Generated artifacts:
None.

Checks run:
- `cargo test --locked -p xtask no_std` (NOSTD-TP-001 passes)
- `RUSTC_BOOTSTRAP=1 cargo build --locked -p bunny-num --no-default-features --target thumbv7em-none-eabihf -Zbuild-std=core` on the local 1.100.0 toolchain (RED on `main`'s crate: `can't find crate for std`, 68 errors). Local only: the toolchain has no rustup, so the gate's `rustup target add` cannot run here; CI's Dojo job runs the gate.
- `cargo run --locked -p xtask -- topic-docs`
- After the change, with the same `-Zbuild-std=core` workaround: `bunny-num --no-default-features` builds for `thumbv7em-none-eabihf` and `wasm32-unknown-unknown` (GREEN); natively `cargo build`, `cargo clippy --all-targets -- -D warnings` and `cargo test` without default features pass (60 tests, the same test set as the default build).
- Seeded `std` uses (NOSTD-TP-003): `std::ops`, `std::error::Error` and a bare `vec!` in the unit tests each fail the build without default features; each seed restored.
- Default API audit (NOSTD-TP-004): rustdoc HTML of the default build at `b14d05e` and at the change differ only by source line anchors and the new crate-level paragraph.
- `cargo fmt --all --check`
- `cargo test --locked --workspace --all-targets --all-features`
- `cargo run --locked -p xtask -- code-dojo-generated`
- `cargo run --locked -p xtask -- code-dojo-rust --all`
- `cargo run --locked -p xtask -- code-dojo-determinism --enforce`
- `cargo run --locked -p xtask -- code-dojo-perf-groups`
- `cargo run --locked -p xtask -- repo-respect check --branch`
- Not run locally: `code-dojo-no-std` and `code-dojo --all` as written (they call `rustup`, absent here), `cargo deny`, markdownlint. CI's Dojo job runs them.

Known risks:
- The gate installs `thumbv7em-none-eabihf` with `rustup target add` at gate time, as the full gate already does for `wasm32-unknown-unknown`; that needs network in CI.
- `cargo clippy` on the local 1.100.0 toolchain reports lints that exist only on that toolchain; the pinned 1.96.0 toolchain runs in CI.

Human reviewer:
@flyingrobots
