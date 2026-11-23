# Receipt: Optics Incidence Cosine Rounding

Task:
flyingrobots/bunny#423: pin the rounding of the incidence cosine of
`bunny-specular-reflection/1`. The docs say `-(u . n)` is rounded once to nearest raw unit,
ties to even; flooring it instead passed every existing case. Five constructed kernel hits on
two planes with non-axis normals put the exact cosine one unit of `2^-64` under a tie, on three
ties (two even floors, one odd) and one unit over a tie, at cosines below 0.38 where one raw
unit of cosine changes the throughput. The kernel already follows the documented rule; no
output changes and the contract id stays `bunny-specular-reflection/1`.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*
.repo-respect/receipts/2026-11-15-optics-specular-reflection.md
crates/bunny-optics/src/shading/specular.rs, crates/bunny-optics/src/shading/wide.rs
crates/bunny-optics/tests/specular_reflection_tests.rs, crates/bunny-optics/tests/exact/mod.rs
crates/bunny-linalg/src/lib.rs, crates/bunny-linalg/src/fixed_vec3.rs (FixedUnitVec3::new)
docs/topics/optics/README.md and test-plan.md (also as they stand on the open branches
`feat/dielectric`, `feat/ggx-conductor`, `feat/point-light`, `feat/thin-lens-camera`,
`fix/ray-sphere-unit-normal` and `fix/272-perf-single-group`, to choose ids none of them uses)

Files edited:
crates/bunny-optics/tests/specular_cosine_rounding_tests.rs (new)
docs/topics/optics/README.md (Specular Reflection: the Cosine row and the cosine's rounding)
docs/topics/optics/test-plan.md (OPTICS-REQ-090; OPTICS-TP-180 to -182; Incidence-Cosine
Mutation Checks; an open gap)
CHANGELOG.md (Unreleased, Fixed)
.repo-respect/receipts/2026-11-23-optics-incidence-cosine-rounding.md

Topic docs:
docs/topics/optics/README.md (Specular Reflection) and test-plan.md. The ids were chosen so
that no open branch uses them. crates/bunny-optics/README.md does not state the cosine's rule,
so it is unchanged.

Generated artifacts:
None.

Checks run:
cargo test --locked -p bunny-optics --test specular_cosine_rounding_tests: 2 cases pass.
The cases were found by a local search (a temporary test, not committed) and their
throughputs checked against a separate Python model with unbounded integers.
Mutations of `incidence_cosine` in specular.rs, each run with
`cargo test --locked -p bunny-optics --no-fail-fast --test specular_cosine_rounding_tests
--test specular_reflection_tests` and the source restored after each: floor, ceiling, ties
away from zero, ties toward zero, ties to odd, per-component rounding and a double rounding
each fail OPTICS-TP-180 and no older case; normalizing `d` without the prescale fails none.

Full local gate at the docs commit, each its own command on the host (local Rust 1.100.0; CI
uses the pinned 1.96.0): cargo fmt --all --check; cargo test --locked --workspace
--all-targets --all-features; cargo run --locked -p xtask -- code-dojo-generated;
code-dojo-rust --all; code-dojo-determinism --enforce; topic-docs; repo-respect check
--branch. All clean. markdownlint-cli2 and cargo deny are not installed locally; CI runs them.

Known risks:
The unit vector `u` along `d` is a premise pinned from `FixedUnitVec3::new` (and checked
within one raw unit of the exact unit vector), not derived by hand.

Human reviewer:
James Ross (maintainer); review pending.
