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
- `xtask/src/feature_configs.rs`
- `xtask/src/code_dojo.rs`
- `xtask/src/main.rs`
- `docs/topics/no-std/test-plan.md`
- `.repo-respect/receipts/2026-11-23-bunny-num-no-std.md`

Topic docs:
- `docs/topics/no-std/test-plan.md` (NOSTD-REQ-001, NOSTD-REQ-002, NOSTD-TP-001 to NOSTD-TP-005)

Generated artifacts:
None.

Checks run:
- `cargo test --locked -p xtask no_std` (NOSTD-TP-001 passes)
- `RUSTC_BOOTSTRAP=1 cargo build --locked -p bunny-num --no-default-features --target thumbv7em-none-eabihf -Zbuild-std=core` on the local 1.100.0 toolchain (RED on `main`'s crate: `can't find crate for std`, 68 errors). Local only: the toolchain has no rustup, so the gate's `rustup target add` cannot run here; CI's Dojo job runs the gate.
- `cargo run --locked -p xtask -- topic-docs`

Known risks:
- The gate installs `thumbv7em-none-eabihf` with `rustup target add` at gate time, as the full gate already does for `wasm32-unknown-unknown`; that needs network in CI.
- `cargo clippy` on the local 1.100.0 toolchain reports lints that exist only on that toolchain; the pinned 1.96.0 toolchain runs in CI.

Human reviewer:
@flyingrobots
