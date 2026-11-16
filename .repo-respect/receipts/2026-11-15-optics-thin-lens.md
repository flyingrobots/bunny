# Receipt: Thin-Lens Camera

Task:
flyingrobots/bunny#364: `bunny-thin-lens/1`, a `ThinLensCamera` beside `PinholeCamera` that
takes an aperture radius and a focus distance and samples the lens with one sampler pair.
Aperture 0 gives the pinhole's rays bit for bit, by the same arithmetic path.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, docs/NUMERIC_CONSTITUTION.md
docs/topics/optics/README.md and test-plan.md
crates/bunny-optics/src/{camera,plane,basis,sampler,lib,error}.rs, tests/camera_tests.rs,
tests/exact/mod.rs
crates/bunny-query/src/trace.rs, crates/bunny-num/src/{fixed_q32_32,fixed_angle}.rs

Files edited:
(work in progress; completed below)

Topic docs:
docs/topics/optics/README.md and test-plan.md

Generated artifacts:
None.

Checks run:
(work in progress)

Known risks:
(work in progress)

Human reviewer:
James Ross (maintainer); review pending.
