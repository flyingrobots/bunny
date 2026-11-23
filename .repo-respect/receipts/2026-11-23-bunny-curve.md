# Receipt: Bunny Curve

Task:
Implement `bunny-curve/1`, part 1 (flyingrobots/bunny#308): `CubicBezier3` in `bunny-linalg` with `at(t)` by de Casteljau over `FixedVec3::lerp` (`bunny-lerp/1`), `derivative_at(t)` as 3 times the de Casteljau quadratic of the checked differences, and `split(t)` into the de Casteljau halves, each `None` for `t` outside `[0, 1]`, with an independent rule port, an exact 384-bit Bernstein oracle, golden vectors and mutation checks. The first commit added the planned cases and failing tests against stub bodies; the second implements the kernel and the living docs.

Files read:
- `CONTRIBUTING.md`
- `CODE_STANDARDS.md`
- `crates/bunny-num/src/lerp.rs`
- `crates/bunny-linalg/src/lerp.rs`
- `crates/bunny-linalg/src/fixed_vec3.rs`
- `crates/bunny-linalg/src/lib.rs`
- `crates/bunny-linalg/tests/lerp_tests.rs`
- `crates/bunny-optics/tests/exact/mod.rs`
- `docs/topics/lerp/README.md`
- `docs/topics/lerp/test-plan.md`
- `.repo-respect/receipts/2026-11-23-bunny-lerp.md`
- `xtask/src/topic_docs.rs`

Files edited:
- `crates/bunny-linalg/src/cubic_bezier3.rs`
- `crates/bunny-linalg/src/lib.rs`
- `crates/bunny-linalg/tests/curve_tests.rs`
- `docs/topics/curve/test-plan.md`
- `docs/topics/curve/README.md`
- `docs/README.md`
- `crates/bunny-linalg/README.md`
- `CHANGELOG.md`
- `.repo-respect/receipts/2026-11-23-bunny-curve.md`

Topic docs:
- `docs/topics/curve/README.md`: new living reference for `bunny-curve/1`, part 1.
- `docs/topics/curve/test-plan.md`: CURVE-REQ-001 to -005 and CURVE-TP-001 to -007, implemented, with the mutation checks.
- `docs/README.md`: the topic's index row.

Generated artifacts:
None

Checks run:
- `cargo test --locked -p bunny-linalg --test curve_tests` (RED: all 6 fail against the stub bodies)
- After the implementation: `cargo test --locked -p bunny-linalg --test curve_tests` (6 passed), `cargo fmt --all --check`, `cargo test --locked --workspace --all-targets --all-features` (116 test binaries ok, none failed), `cargo run --locked -p xtask -- code-dojo-generated`, `code-dojo-rust --all`, `code-dojo-determinism --enforce`, `code-dojo-perf-groups`, `topic-docs`, `repo-respect check --branch`, `git diff --check`.
- `cargo clippy --locked -p bunny-linalg --all-targets --all-features -- -D warnings` clean.
- Mutations: ten, recorded in `docs/topics/curve/test-plan.md#mutation-checks`; each run with `cargo test --locked --no-fail-fast -p bunny-linalg --test curve_tests` and the source restored after.
- The golden vectors come from a Python model (integer rule port and `fractions` bounds) posted in the pull request, not kept in the tree.
- The wasm32 tests were not run locally (`wasm-pack` and the target are not installed); CI's WebAssembly job runs every `curve_tests` case.
- markdownlint and `cargo deny` are not installed locally; CI's Dojo job runs them.

Known risks:
- Local toolchain 1.100.0, CI pins 1.96.0.
- CURVE-TP-005 and -006 take about 8 s natively in a debug build, slower under the WebAssembly job.
- The pull request is about 950 changed lines against the issue's guide of about 400: about 190 are source, the rest tests and docs.
- No `bunny-perf` case is added: the performance chapter does not require one for a new API.
- flyingrobots/bunny#307 expects a `Curve3` trait that this pull request does not add.

Human reviewer:
@flyingrobots
