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
- `.repo-respect/receipts/2026-11-23-catmull-rom.md`

Topic docs:
- `docs/topics/curve/test-plan.md`: CURVE-REQ-010 to -014 and CURVE-TP-020 to -025, planned.

Generated artifacts:
None

Checks run:
- `cargo test --locked -p bunny-linalg --test catmull_rom_tests` (RED: all 5 fail against the stub bodies)
- `cargo run --locked -p xtask -- topic-docs` clean.

Known risks:
- Local toolchain 1.100.0, CI pins 1.96.0.

Human reviewer:
@flyingrobots
