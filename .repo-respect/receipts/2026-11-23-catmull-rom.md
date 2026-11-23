# Receipt: Catmull-Rom Splines

Task:
Implement `bunny-curve/1`, part 2 (flyingrobots/bunny#309): `CatmullRom3` in `bunny-linalg`, a uniform Catmull-Rom spline over a borrowed point slice, open (end points duplicated as phantom neighbours) or closed (wrapped), with typed refusals (`CatmullRomError`), `segment_count`, `segment(i)` and `to_beziers()` converting each segment to a `CubicBezier3` whose inner control points are `(6 P1 + P2 - P0) / 6` and `(6 P2 + P1 - P3) / 6` rounded once to nearest, ties to even, and `at(u)` and `derivative_at(u)` evaluating through `CubicBezier3`. Tested against an independent rule port and an exact 384-bit power-form oracle, golden vectors from a Python model posted in the pull request, thresholds on ties, and mutation checks. The first commit adds the planned cases and failing tests against stub bodies; the second implements the kernel and the living docs.

Files read:
- `CONTRIBUTING.md`
- `CODE_STANDARDS.md`
- `crates/bunny-linalg/src/cubic_bezier3.rs`
- `crates/bunny-linalg/src/fixed_vec3.rs`
- `crates/bunny-linalg/src/viewport.rs`
- `crates/bunny-linalg/src/lib.rs`
- `crates/bunny-linalg/tests/curve_tests.rs`
- `docs/topics/curve/README.md`
- `docs/topics/curve/test-plan.md`
- `.repo-respect/receipts/2026-11-23-bunny-curve.md`
- `xtask/src/topic_docs.rs`

Files edited:
- `crates/bunny-linalg/src/catmull_rom3.rs`
- `crates/bunny-linalg/src/lib.rs`
- `crates/bunny-linalg/tests/catmull_rom_tests.rs`
- `docs/topics/curve/test-plan.md`
- `docs/topics/curve/README.md`
- `docs/README.md`
- `crates/bunny-linalg/README.md`
- `CHANGELOG.md`
- `.repo-respect/receipts/2026-11-23-catmull-rom.md`

Topic docs:
- `docs/topics/curve/README.md`: the Catmull-Rom sections of `bunny-curve/1` (points and refusals, segments, parameter, evaluation, derivative, widths).
- `docs/topics/curve/test-plan.md`: CURVE-REQ-010 to -014 and CURVE-TP-020 to -025, implemented, with the Catmull-Rom oracles and mutation checks.
- `docs/README.md`: the curve topic's index row now covers part 2.

Generated artifacts:
None

Checks run:
- `cargo test --locked -p bunny-linalg --test catmull_rom_tests` (RED: all 5 fail against the stub bodies)
- `cargo run --locked -p xtask -- topic-docs` clean.
- After the implementation: `cargo test --locked -p bunny-linalg --test catmull_rom_tests` (5 passed), `cargo fmt --all --check`, `cargo test --locked --workspace --all-targets --all-features` (120 test result lines, all ok, none failed), `cargo run --locked -p xtask -- code-dojo-generated`, `code-dojo-rust --all`, `code-dojo-determinism --enforce`, `code-dojo-perf-groups`, `topic-docs`, `repo-respect check --branch`, `git diff --check`.
- `cargo clippy --locked -p bunny-linalg --all-targets --all-features -- -D warnings` clean.
- Mutations: ten, recorded in `docs/topics/curve/test-plan.md#mutation-checks`; each run with `cargo test --locked --no-fail-fast -p bunny-linalg --test catmull_rom_tests` and the source restored after (checked with `cmp`).
- The golden vectors come from a Python model (integer rule port and `fractions` bounds) posted in the pull request, not kept in the tree.
- The wasm32 tests were not run locally; CI's WebAssembly job runs every `catmull_rom_tests` case.
- markdownlint and `cargo deny` are not installed locally; CI's Dojo job runs them.

Known risks:
- Local toolchain 1.100.0, CI pins 1.96.0.
- The issue's collinear criterion (`at(u) = P0 + u d` at `u = k/8`) holds on interior segments only: the duplicated phantom end points the issue mandates make the end segments follow `d (t + 2 t^2 - t^3) / 2`; CURVE-TP-020 tests both.
- `ControlPointOutOfRange` is a third refusal the issue does not list; the alternative was `None` from `at` on a segment whose control points overflow.
- flyingrobots/bunny#307 may add a `Curve3` trait; `CatmullRom3` does not implement it here.
- No `bunny-perf` case is added.

Human reviewer:
@flyingrobots
