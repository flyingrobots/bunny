# Receipt: Transform-Aware Bounds

Task:
flyingrobots/bunny#119, the remainder after flyingrobots/bunny#478: transform-aware bounds that
are conservative against the exact image, for boxes, point sets (meshes), triangles and spheres.
`FixedAabb3::transformed_bounds`, `from_transformed_points`, `from_transformed_triangle` and
`from_transformed_sphere` in `bunny-geom` return the smallest raw box containing the exact image
under a `FixedAffine3`, refusing with the existing `BoundsError`. Additive: no existing public
item, output bit, golden, digest or ID changes; no dependency is added and no type moves.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*
.repo-respect/receipts/2026-11-23-ray-box-normal-transforms.md
crates/bunny-geom/src/{lib.rs,transform.rs,bounds.rs,frustum/sphere.rs}, crates/bunny-geom/Cargo.toml
crates/bunny-linalg/src/{fixed_affine3.rs,fixed_mat3.rs}, crates/bunny-num/src/fixed_angle.rs
crates/bunny-geom/tests/ray_box_transform_tests.rs, crates/bunny-geom/tests/shape_oracle/mod.rs
crates/bunny-geom/README.md, docs/README.md, docs/topics/transform/README.md and test-plan.md
flyingrobots/bunny#119 and flyingrobots/bunny#478 (description); open pull requests
flyingrobots/bunny#492, #495, #496, #469, #466 (changed files only)

Files edited:
crates/bunny-geom/src/transform_bounds.rs (new: the four helpers)
crates/bunny-geom/src/transform_bounds/wide.rs (new: the sphere reach in 256-bit integers)
crates/bunny-geom/src/lib.rs (one module line)
crates/bunny-geom/tests/transform_bounds_tests.rs (new)
crates/bunny-geom/tests/transform_bounds_oracle/mod.rs (new: a big-integer oracle)
crates/bunny-geom/README.md, CHANGELOG.md, docs/README.md (the transform row)
docs/topics/transform/README.md (transform-aware bounds; limitations)
docs/topics/transform/test-plan.md (TRS-REQ-020 to -023, TRS-TP-050 to -060; mutations)
.repo-respect/receipts/2026-11-24-transform-aware-bounds.md

Topic docs:
docs/topics/transform/ gains TRS-REQ-020 to -023 and TRS-TP-050 to -060, numbers no other open
pull request uses (the earlier task used TRS-REQ-010 to -014 and TRS-TP-020 to -032).

Generated artifacts:
None.

Checks run:
First commit (tests and planned cases only): the new tests fail to compile (the helpers do not
exist); topic-docs and repo-respect check --branch clean.

Known risks:
The bounds contain the exact image, not the rounded `checked_transform_point` image of every
point, which can lie up to 3/2 raw outside; the chapter says so and how to widen.

Human reviewer:
James Ross (maintainer); review pending.
