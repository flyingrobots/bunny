# Receipt: No Std Audit

Task:
The remainder of flyingrobots/bunny#156: audit every `std` use in `bunny-linalg`, `bunny-geom` and `bunny-query`, decide whether `alloc` is needed, and adopt `no_std` only where it is purely additive, following flyingrobots/bunny#447 (`bunny-num`). The first commit adds the gate for the three crates and the planned cases; the gate's bare-metal build fails on `main`'s crates.

Files read:
- `CONTRIBUTING.md`
- `CODE_STANDARDS.md`
- every file under `crates/bunny-linalg/src`, `crates/bunny-geom/src` and `crates/bunny-query/src` (grep for `std`, collections, allocation and float methods; the matches read in context)
- `crates/bunny-linalg/Cargo.toml`, `crates/bunny-geom/Cargo.toml`, `crates/bunny-query/Cargo.toml`
- the change of flyingrobots/bunny#447, end to end
- `xtask/src/feature_configs.rs`
- `xtask/src/code_dojo.rs`
- `xtask/src/main.rs`
- `docs/topics/no-std/README.md`
- `docs/topics/no-std/test-plan.md`

Files edited:
- `xtask/src/code_dojo.rs`
- `xtask/src/feature_configs.rs`
- `xtask/src/main.rs`
- `crates/bunny-linalg/tests/std_feature_tests.rs`
- `crates/bunny-geom/tests/std_feature_tests.rs`
- `crates/bunny-query/tests/std_feature_tests.rs`
- `docs/topics/no-std/test-plan.md`
- `.repo-respect/receipts/2026-11-23-no-std-audit.md`

Topic docs:
- `docs/topics/no-std/test-plan.md` (NOSTD-REQ-010 to NOSTD-REQ-013, NOSTD-TP-010 to NOSTD-TP-017)

Generated artifacts:
None.

Checks run:
- `cargo test --locked -p xtask no_std` (NOSTD-TP-001 and NOSTD-TP-010 pass)
- `cargo test --locked -p bunny-linalg --test std_feature_tests`, and the same for `bunny-geom` and `bunny-query` (pass on the base code: they pin the default build's behaviour)
- `RUSTC_BOOTSTRAP=1 cargo build --locked -p bunny-linalg --no-default-features --target thumbv7em-none-eabihf -Zbuild-std=core` on the local 1.100.0 toolchain: RED on `main`'s crates (`bunny-num` is built with `std`, 149 errors). Local only: the toolchain has no rustup; CI's Dojo job runs the gate.
- `cargo run --locked -p xtask -- topic-docs`
- `cargo run --locked -p xtask -- repo-respect check --branch`

Known risks:
- The gate's bare-metal and wasm32 builds run only in CI (no rustup on this machine).
- `cargo clippy` on the local 1.100.0 toolchain reports lints that exist only on that toolchain; the pinned 1.96.0 toolchain runs in CI.

Human reviewer:
@flyingrobots
