# Receipt: Light Direction

Task:
flyingrobots/bunny#313: `DirectionalLight::from_angles(azimuth, elevation, irradiance)` and
`DirectionalLight::from_direction(v, irradiance)` under the identity `bunny-light-direction/1`.
The direction `(cos e cos a, sin e, cos e sin a)` from `FixedAngle::sin_cos`, each product one
Q32.32 multiplication rounded to nearest, ties to even, then every component times `2^32` over
the exact largest magnitude, rounded to nearest, ties to even, so the largest is exactly
`+-1.0`. The zero vector is refused with a new `LightDirectionError`. Additive: no existing
public item, output bit, golden, digest or ID changes; `DirectionalLight::new` is unchanged.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*
.repo-respect/receipts/2026-11-23-rotation.md
crates/bunny-optics/src/shading.rs, crates/bunny-optics/src/shading/{light.rs,point_light.rs,wide.rs}
crates/bunny-optics/src/lib.rs, crates/bunny-optics/Cargo.toml
crates/bunny-optics/tests/{point_light_tests.rs,exact/mod.rs}
crates/bunny-num/src/{fixed_angle.rs,fixed_q32_32.rs,lib.rs}, crates/bunny-num/tests/angle_oracle/mod.rs
docs/topics/{angle-trigonometry,coordinate-law}/README.md
docs/topics/optics/README.md and test-plan.md
xtask/src/topic_docs.rs (ledger rules), xtask/src/code_dojo.rs (gates)
flyingrobots/bunny#313; the optics-doc edits of flyingrobots/bunny#452 and #456 (insertion points only)

Files edited:
crates/bunny-optics/tests/light_direction_tests.rs (new)
crates/bunny-optics/tests/light_angle_oracle/mod.rs (new: the exact series oracle)
docs/topics/optics/test-plan.md (OPTICS-REQ-150 to -154, OPTICS-TP-290 to -298, the oracle and
the mutation section)
.repo-respect/receipts/2026-11-23-light-direction.md

Topic docs:
docs/topics/optics/test-plan.md (planned cases first).

Generated artifacts:
None.

Checks run:
First commit (tests and planned cases only): the light-direction tests fail to compile (the
kernel does not exist); cargo run --locked -p xtask -- topic-docs clean;
cargo run --locked -p xtask -- repo-respect check --branch clean.

Known risks:
The test file does not compile until the kernel lands in the next commit.

Human reviewer:
James Ross (maintainer); review pending.
