# Receipt: Bunny Curve

Task:
Implement `bunny-curve/1`, part 1 (flyingrobots/bunny#308): `CubicBezier3` in `bunny-linalg` with `at(t)` by de Casteljau over `FixedVec3::lerp` (`bunny-lerp/1`), `derivative_at(t)` as 3 times the de Casteljau quadratic of the checked differences, and `split(t)` into the de Casteljau halves, each `None` for `t` outside `[0, 1]`, with an independent rule port, an exact 384-bit Bernstein oracle, golden vectors and mutation checks. The first commit adds the planned cases and failing tests against stub bodies.

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
- `.repo-respect/receipts/2026-11-23-bunny-curve.md`

Topic docs:
- `docs/topics/curve/test-plan.md`: CURVE-REQ-001 to -005 and CURVE-TP-001 to -007, planned.

Generated artifacts:
None

Checks run:
- `cargo test --locked -p bunny-linalg --test curve_tests` (RED: all 6 fail against the stub bodies)
- `cargo run --locked -p xtask -- topic-docs`

Known risks:
- Local toolchain 1.100.0, CI pins 1.96.0.

Human reviewer:
@flyingrobots
