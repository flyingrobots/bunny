# Receipt: No Std Audit

Task:
The remainder of flyingrobots/bunny#156: audit every `std` use in `bunny-linalg`, `bunny-geom` and `bunny-query`, decide whether `alloc` is needed, and adopt `no_std` only where it is purely additive, following flyingrobots/bunny#447 (`bunny-num`). The first commit adds the gate for the three crates and the planned cases; the gate's bare-metal build fails on `main`'s crates. The second adopts `no_std` in all three (every `std` use was a `core` item under the same name), with a default `std` feature that turns on the Bunny dependencies' `std` features, and writes the audit into the chapter.

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
- `docs/topics/no-std/README.md`
- `docs/CODE_DOJO.md`
- `docs/README.md`
- `CHANGELOG.md`
- `crates/bunny-linalg/Cargo.toml`, `README.md`, `src/lib.rs`, `src/catmull_rom3.rs`, `src/fixed_mat4_look_at.rs`, `src/fixed_mat4_projection.rs`, `src/fixed_vec2.rs`, `src/fixed_vec3.rs`, `src/viewport.rs`
- `crates/bunny-geom/Cargo.toml`, `README.md`, `src/lib.rs`, `src/plane/polygon.rs`
- `crates/bunny-query/Cargo.toml`, `README.md`, `src/lib.rs`, `src/frame.rs`, `src/ray.rs`, `src/ray/aabb_wide.rs`, `src/trace/box_hit.rs`, `src/trace/cylinder.rs`, `src/trace/cylinder/span.rs`, `src/trace/disc.rs`, `src/trace/sphere.rs`
- `.repo-respect/receipts/2026-11-23-no-std-audit.md`

Topic docs:
- `docs/topics/no-std/README.md` (status, features, the audit tables per crate, the gate)
- `docs/topics/no-std/test-plan.md` (NOSTD-REQ-010 to NOSTD-REQ-013, NOSTD-TP-010 to NOSTD-TP-017)

Generated artifacts:
None.

Checks run:
- `cargo test --locked -p xtask no_std` (NOSTD-TP-001 and NOSTD-TP-010 pass)
- `cargo test --locked -p bunny-linalg --test std_feature_tests`, and the same for `bunny-geom` and `bunny-query` (pass on the base code: they pin the default build's behaviour)
- `RUSTC_BOOTSTRAP=1 cargo build --locked -p bunny-linalg --no-default-features --target thumbv7em-none-eabihf -Zbuild-std=core` on the local 1.100.0 toolchain: RED on `main`'s crates (`bunny-num` is built with `std`, 149 errors). Local only: the toolchain has no rustup; CI's Dojo job runs the gate.
- `cargo run --locked -p xtask -- topic-docs`
- `cargo run --locked -p xtask -- repo-respect check --branch`
- After the change, with the same `-Zbuild-std=core` workaround: each of `bunny-linalg`, `bunny-geom` and `bunny-query` with `--no-default-features` builds for `thumbv7em-none-eabihf` and `wasm32-unknown-unknown` (GREEN); natively `cargo build`, `cargo clippy --all-targets -- -D warnings` and `cargo test` without default features pass for `bunny-linalg` (91 tests) and `bunny-query` (118 tests); `bunny-geom` builds and tests (47 tests), and its Clippy reports only `clippy::assert_is_empty` in the untouched `tests/polygon_clip_tests.rs`, a lint of the local 1.100.0 toolchain that the same command with default features also reports.
- Seeded `std` uses (NOSTD-TP-012): one per crate, each fails the native build without default features; a Bunny dependency line without `default-features = false` fails only the bare-metal build. Each seed restored.
- Default API audit (NOSTD-TP-013): rustdoc HTML of the default builds at `b306708` and at the change differ only by `lib.rs` source anchors and the new crate-level paragraph.
- `cargo tree --locked -p bunny-query -e features -i bunny-num`, with and without default features, and over `--workspace` (NOSTD-TP-016).
- `cargo fmt --all --check`
- `cargo test --locked --workspace --all-targets --all-features` (944 passed, 0 failed, 133 test binaries)
- `cargo run --locked -p xtask -- code-dojo-generated`
- `cargo run --locked -p xtask -- code-dojo-rust --all`
- `cargo run --locked -p xtask -- code-dojo-determinism --enforce`
- `cargo run --locked -p xtask -- code-dojo-perf-groups`
- `cargo clippy --locked -p <crate> --all-targets --all-features -- -D warnings` for `xtask`, `bunny-linalg`, `bunny-geom` and `bunny-query`: clean for `bunny-linalg` and `bunny-query`; `xtask` and `bunny-geom` report only `clippy::assert_is_empty` (1.100.0 only) in files this change does not touch.
- Not run locally: `code-dojo-no-std` and `code-dojo --all` as written (they call `rustup`, absent here), `cargo deny`, markdownlint, the headless wasm tests. CI's jobs run them.

Known risks:
- The gate's bare-metal and wasm32 builds run only in CI (no rustup on this machine).
- Open pull requests that add non-test code with `std::` paths to these crates (flyingrobots/bunny#475, #478, #479 at the time of writing) will fail this gate after they merge until those paths become `core::`.
- `cargo clippy` on the local 1.100.0 toolchain reports lints that exist only on that toolchain; the pinned 1.96.0 toolchain runs in CI.

Human reviewer:
@flyingrobots
