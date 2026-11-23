# Receipt: Environment Equirect

Task:
flyingrobots/bunny#394: equirectangular environment coordinates, `bunny-environment-equirect/1`.
`equirect_from_direction` maps a nonzero direction of any length to `(u, v)`: `u` the longitude
`atan2(-x, z)` in turns (`+z` the seam at 0, `-z` the image centre at 1/2, `[0, 1)`), `v` the
polar angle from `+y` in half turns (`+y` at 0, `[0, 1]`), both from `FixedAngle::atan2` and
one rounding to nearest, ties to even; `u = 0` at the poles; the zero vector is refused.
`direction_from_equirect` maps `(u, v)` back with `FixedAngle::sin_cos` and two rounded
products; `v` outside `[0, 1]` is refused. Bounds: `u` within `1/2 + 2^-32` and `v` within
`1/2 + 2^-29` raw units; the inverse within `5/2` raw units per horizontal component and one
for `y`; round trips within `42/5` raw units (direction) and one raw unit of `v` plus a stated
`u` bound away from the poles (coordinates). Additive: no existing public item, output bit,
golden, digest or ID changes.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*, .repo-respect/receipts/2026-11-23-ray-footprint.md
docs/topics/angle-trigonometry/README.md and test-plan.md, docs/topics/coordinate-law/README.md
docs/topics/optics/README.md and test-plan.md
crates/bunny-num/src/{lib.rs,fixed_angle.rs,fixed_q32_32.rs}, crates/bunny-num/tests/angle_oracle/mod.rs
crates/bunny-optics/src/{lib.rs,error.rs,texture.rs,footprint.rs}
crates/bunny-optics/tests/{ray_footprint_tests.rs,texture_bilinear_tests.rs,exact/mod.rs}
crates/bunny-optics/README.md, CHANGELOG.md, xtask/src/topic_docs.rs
flyingrobots/bunny#394, #395 and #414 (the issues that list #394 in `.blocked_by`)

Files edited:
crates/bunny-optics/src/environment.rs (new), crates/bunny-optics/src/lib.rs (module, exports)
crates/bunny-optics/tests/environment_equirect_tests.rs (new)
crates/bunny-optics/tests/equirect_oracle/mod.rs (new)
crates/bunny-optics/README.md, CHANGELOG.md
docs/topics/optics/README.md (Status; Environment Coordinates)
docs/topics/optics/test-plan.md (OPTICS-REQ-120 to -126; OPTICS-TP-230 to -244; the
equirectangular oracle; Equirectangular Mutation Checks; two open gaps)
.repo-respect/receipts/2026-11-23-environment-equirect.md

Topic docs:
docs/topics/optics/README.md (Environment Coordinates) and test-plan.md, with ids only from the
range assigned to this task (OPTICS-REQ-120 to -129, OPTICS-TP-230 to -249).

Generated artifacts:
None. The 60-digit decimal model of the forward goldens is posted on the pull request, not
committed.

Checks run:
First commit (tests and planned cases only): the test target fails to compile because the
kernel does not exist; topic-docs clean. Later commits list their checks in the pull request.

Known risks:
The axis convention, the pole value `u = 0`, `v = 0` at the top and the closed range of `v`
were chosen here: the issue does not fix them. Bounds are proven by analysis and checked on
seeded and constructed samples, not exhaustively.

Human reviewer:
James Ross (maintainer); review pending.
