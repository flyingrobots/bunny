# Receipt: Ray, Box And Normal Transforms

Task:
flyingrobots/bunny#108, the remainder: ray and axis-aligned box transformation helpers under a
`FixedAffine3` (`FixedRay3::checked_transform`, giving an unnormalised `TransformedRay3`, and
`FixedAabb3::checked_transform`, the hull of the eight rounded corners) in `bunny-geom`, with the
typed `TransformError`; the inverse-transpose normal transform
`FixedAffine3::checked_transform_normal` in `bunny-linalg`; and the chapter text on what a
non-uniform scale does to normals and bounds. Additive: no existing public item, output bit,
golden, digest or ID changes; no dependency is added and no type moves.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*
.repo-respect/receipts/2026-11-23-transform-composition.md
crates/bunny-linalg/src/{lib.rs,fixed_affine3.rs,fixed_mat3.rs,fixed_affine3_trs.rs,fixed_vec3.rs}
crates/bunny-num/src/fixed_q32_32.rs (checked_mul, checked_div and their rounding)
crates/bunny-geom/src/lib.rs, crates/bunny-geom/Cargo.toml, scripts/publish-crates.sh
crates/bunny-query/src/{trace.rs,frame.rs} (ray types with unnormalised directions)
crates/bunny-linalg/tests/transform_tests.rs, crates/bunny-geom/tests/plane_clip_tests.rs
docs/topics/{transform,matrix-types}/README.md and test plans
flyingrobots/bunny#108, #119; open pull requests flyingrobots/bunny#469, #471, #475 (names only)

Files edited:
crates/bunny-geom/tests/ray_box_transform_tests.rs (new)
crates/bunny-linalg/tests/normal_transform_tests.rs (new)
docs/topics/transform/test-plan.md (TRS-REQ-010 to -014, TRS-TP-020 to -032)
.repo-respect/receipts/2026-11-23-ray-box-normal-transforms.md

Topic docs:
docs/topics/transform/ gains TRS-REQ-010 to -014 and TRS-TP-020 to -032, inside the range
assigned for this task (TRS-REQ-010 to -019, TRS-TP-020 to -049).

Generated artifacts:
None.

Checks run:
First commit (tests and planned cases only): the new tests fail to compile (the helpers do not
exist); topic-docs and repo-respect check --branch clean.

Known risks:
`checked_transform_normal` inherits `try_inverse`, whose distance from the exact inverse has no
bound in raw units; the bound is stated against the rounded inverse only.

Human reviewer:
James Ross (maintainer); review pending.
