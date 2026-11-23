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
crates/bunny-optics/src/shading/light_direction.rs (new: the kernel, LightDirectionError and
LIGHT_DIRECTION_ID)
crates/bunny-optics/src/shading.rs and crates/bunny-optics/src/lib.rs (module and exports)
crates/bunny-optics/tests/light_direction_tests.rs (new)
crates/bunny-optics/tests/light_angle_oracle/mod.rs (new: the exact series oracle)
docs/topics/optics/README.md (new section "Directional Light From Angles" and one export bullet)
docs/topics/optics/test-plan.md (OPTICS-REQ-150 to -154, OPTICS-TP-290 to -298, the oracle and
the mutation section)
crates/bunny-optics/README.md, CHANGELOG.md
.repo-respect/receipts/2026-11-23-light-direction.md

Topic docs:
docs/topics/optics/README.md and test-plan.md: planned cases first, marked implemented with the
kernel. Edits are a new section and new ledger rows placed away from the insertion points of
flyingrobots/bunny#452 and #456, which edit the same files.

Generated artifacts:
None.

Checks run:
First commit (tests and planned cases only): the light-direction tests fail to compile (the
kernel does not exist); cargo run --locked -p xtask -- topic-docs clean;
cargo run --locked -p xtask -- repo-respect check --branch clean.

Kernel commit, each its own command on the host (local Rust 1.100.0; CI uses the pinned 1.96.0):
cargo fmt --all --check; cargo test --locked --workspace --all-targets --all-features (117 test
result lines, all ok); cargo run --locked -p xtask -- code-dojo-generated; code-dojo-rust --all;
code-dojo-determinism --enforce; code-dojo-perf-groups; topic-docs; repo-respect check --branch.
cargo clippy --locked -p bunny-optics --lib --test light_direction_tests --all-features -- -D
warnings clean (other optics test files report lints that exist only on 1.100.0).
Mutations: seventeen, each applied to light_direction.rs, run with cargo test --locked -q -p
bunny-optics --no-fail-fast --test light_direction_tests, and restored; each failed at least
one case (the test plan's table). The two product-tie mutants survived the first run, and two
tie goldens were added.
Python model (integer steps on sin_cos's raw outputs; Decimal at 60 digits for the exact
direction): every golden equals the model; component errors and angles within the bound; the
truncated division changes six of the ten goldens. It is in a pull-request comment, not the tree.
Not run locally: wasm-pack (not installed; CI's WebAssembly job runs it), cargo deny and
markdownlint-cli2 (not installed; CI's Dojo job runs them), clippy on Rust 1.96.0.

Known risks:
The bound is derived from sin_cos's documented one raw unit and is loose: measured 2.89 raw
units against 9.161. The environment chapter of the held draft flyingrobots/bunny#452 measures
its longitude from +z counter-clockwise seen from +y; this azimuth, fixed by the issue, starts at
+x and turns toward +z. Both are stated in their own chapters.

Human reviewer:
James Ross (maintainer); review pending.
