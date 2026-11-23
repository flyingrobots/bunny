# Receipt: Inverse Trig Slice 2

Task:
flyingrobots/bunny#244, second slice (Refs, not Closes): `FixedAngle::atan` of a single
Q32.32 ratio, `FixedAngle::asin` and `FixedAngle::acos` with a typed refusal outside
`|x| <= 1.0`, and signed conversions out (`to_turns_signed`, `to_degrees_signed`,
`to_radians_signed`). Tangent, the `sin_cos` error sweep tool and rotation APIs are out of
scope. Additive: no existing public item, output bit, golden, digest or ID changes.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*
.repo-respect/receipts/2026-11-23-rotation.md
crates/bunny-num/src/{fixed_angle.rs,fixed_angle/atan2.rs,fixed_angle/conversions_out.rs,lib.rs,fixed_q32_32.rs}
crates/bunny-num/tests/{fixed_angle_inverse_tests.rs,angle_oracle/mod.rs}
docs/topics/angle-trigonometry/{README.md,test-plan.md}
xtask/src/code_dojo.rs (category limits), xtask/src/topic_docs.rs (case statuses)
flyingrobots/bunny#244 and the merged first slice, flyingrobots/bunny#448

Files edited:
crates/bunny-num/tests/fixed_angle_inverse_ratio_tests.rs (new)
crates/bunny-num/tests/inverse_oracle/mod.rs (new)
crates/bunny-num/tests/angle_oracle/mod.rs (visibility of four items only)
docs/topics/angle-trigonometry/test-plan.md (AT-REQ-020 to -026, AT-TP-030 to -039)
.repo-respect/receipts/2026-11-23-inverse-trig-slice-2.md

Topic docs:
docs/topics/angle-trigonometry/test-plan.md: planned cases AT-TP-030 to AT-TP-039.

Generated artifacts:
None.

Checks run:
First commit (tests and planned cases only): the new test target fails to compile because
the kernel functions do not exist yet; cargo run --locked -p xtask -- topic-docs clean.

Known risks:
`asin` and `acos` are not a single rounding of the exact angle: the cosine leg is a floor
square root, so the bound is 2/3 of an angle unit rather than 1/2 + 2^-9.

Human reviewer:
James Ross (maintainer); review pending.
