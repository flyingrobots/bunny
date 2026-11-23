# Receipt: Fixed Angle Atan2 Conversions

Task:
First slice of flyingrobots/bunny#244: conversions out of `FixedAngle` (`to_turns`, `to_degrees`, `to_radians`) and `FixedAngle::atan2`, each with a stated rounding rule and bound in raw units, an independent high-precision oracle, golden vectors from a 60-digit decimal model and mutation checks. The first commit adds the planned cases and failing tests against stub bodies; the second implements the kernels and the living docs. Tangent, `asin`, `acos`, single-ratio `atan`, rotation APIs and the `sin_cos` error-sweep tool remain for later pull requests.

Files read:
- `CONTRIBUTING.md`
- `CODE_STANDARDS.md`
- `crates/bunny-num/src/fixed_angle.rs`
- `crates/bunny-num/src/fixed_q32_32.rs`
- `crates/bunny-num/src/lerp.rs`
- `crates/bunny-num/src/lib.rs`
- `crates/bunny-num/tests/fixed_angle_tests.rs`
- `crates/bunny-num/tests/lerp_tests.rs`
- `crates/bunny-num/README.md`
- `docs/topics/angle-trigonometry/README.md`
- `docs/topics/angle-trigonometry/test-plan.md`
- `docs/topics/lerp/test-plan.md`
- `docs/topics/look-rotation/README.md`
- `docs/topics/fixed-point-rounding/README.md`
- `.repo-respect/receipts/2026-11-23-bunny-lerp.md`

Files edited:
- `crates/bunny-num/src/fixed_angle/atan2.rs`
- `crates/bunny-num/src/fixed_angle/conversions_out.rs`
- `crates/bunny-num/src/fixed_angle.rs`
- `crates/bunny-num/tests/angle_oracle/mod.rs`
- `crates/bunny-num/tests/fixed_angle_inverse_tests.rs`
- `docs/topics/angle-trigonometry/test-plan.md`
- `.repo-respect/receipts/2026-11-23-fixed-angle-atan2-conversions.md`

Topic docs:
- `docs/topics/angle-trigonometry/test-plan.md`: AT-REQ-010 to -014 and AT-TP-010 to -017, planned.

Generated artifacts:
None

Checks run:
- `cargo test --locked -p bunny-num --test fixed_angle_inverse_tests` (RED: 6 of 7 fail against the stubs; AT-TP-015 passes vacuously while every result is `None`)
- `cargo run --locked -p xtask -- topic-docs`
- `cargo run --locked -p xtask -- repo-respect check --branch`

Known risks:
- Local toolchain 1.100.0, CI pins 1.96.0.

Human reviewer:
@flyingrobots
