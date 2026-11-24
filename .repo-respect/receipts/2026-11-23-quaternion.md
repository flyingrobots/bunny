# Receipt: Quaternion

Task:
flyingrobots/bunny#109: deterministic quaternion rotations under the contract identity
`bunny-quaternion/1` (`QUATERNION_CONTRACT_ID`). `FixedUnitQuat` stores `(x, y, z, w)` with
`w` the scalar part and raw `x^2 + y^2 + z^2 + w^2` in `[(2^32 - 1)^2, (2^32 + 1)^2]`.
`try_from_unit` checks the band exactly; `try_normalize` rounds each exact `r / |p|` once to the
nearest raw unit and refuses only zero. The Hamilton product is normalised in the same single
rounding and composes right to left. `rotate_vec3` and `to_mat3` divide the exact `q v q*` by
`|q|^2` and round once per component. `from_axis_angle` halves the signed angle toward zero and
normalises the exact `(s u, c)`. `conjugate`, `Neg` and `canonical` are exact. Additive: no
existing public item, output bit, golden, digest or ID changes.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*
.repo-respect/receipts/2026-11-23-rotation.md
crates/bunny-linalg/src/{lib.rs,fixed_vec3.rs,fixed_mat3.rs,fixed_mat3_rotation.rs,lerp.rs,unproject.rs,catmull_rom3.rs}
crates/bunny-linalg/tests/{rotation_tests.rs,unproject_tests.rs,unproject_oracle/mod.rs}
crates/bunny-num/src/{fixed_q32_32.rs,fixed_angle.rs,lerp.rs}, crates/bunny-num/tests/fixed_angle_tests.rs
crates/bunny-optics/src/shading/wide256.rs (unit-test pattern only)
xtask/src/code_dojo.rs (category limits), xtask/src/topic_docs.rs (ledger rules), xtask/src/module_tree.rs
docs/topics/{rotation,look-rotation,lerp,angle-trigonometry}/README.md and test plans
flyingrobots/bunny#109; flyingrobots/bunny#482 (the oriented box basis check); flyingrobots/bunny#409 (title only)

Files edited:
crates/bunny-linalg/src/fixed_quat.rs (new: the type, the band, constructors, sign operations)
crates/bunny-linalg/src/fixed_quat/{normalize.rs,product.rs,rotate.rs,axis_angle.rs,wide.rs} (new)
crates/bunny-linalg/src/lib.rs (module and an export line at the end of the list)
crates/bunny-linalg/tests/quaternion_tests.rs (new)
crates/bunny-linalg/tests/quaternion_oracle/mod.rs (new: the independent oracle)
crates/bunny-linalg/README.md, CHANGELOG.md, docs/README.md (topic index)
docs/topics/quaternion/README.md (new chapter)
docs/topics/quaternion/test-plan.md (new: QUAT-REQ-001 to -010, QUAT-TP-001 to -017; mutations)
docs/topics/rotation/README.md and docs/topics/rotation/test-plan.md (one pointer each; the
test plan's "quaternions (flyingrobots/bunny#109)" out-of-scope note now points at the chapter)
.repo-respect/receipts/2026-11-23-quaternion.md

Topic docs:
docs/topics/quaternion/ is a new topic with its own QUAT- prefix; no other topic uses it.

Generated artifacts:
None.

Checks run:
First commit (tests and planned cases only): the quaternion tests fail to compile (the kernel
does not exist); topic-docs and repo-respect check --branch clean.
Kernel commit, each its own command on the host (local Rust 1.100.0; CI uses the pinned 1.96.0):
cargo fmt --all --check; cargo test --locked --workspace --all-targets --all-features;
cargo build --locked -p bunny-linalg --no-default-features;
cargo run --locked -p xtask -- code-dojo-generated; code-dojo-rust --all;
code-dojo-determinism --enforce; code-dojo-perf-groups; topic-docs; repo-respect check
--branch. cargo clippy --locked -p bunny-linalg --all-targets --all-features -- -D warnings clean.
Mutations: fifteen, each applied to the kernel, run with cargo test --locked -q -p bunny-linalg
--no-fail-fast --test quaternion_tests --lib, and restored; fourteen failed at least one case and
one is equivalent (the test plan's table).
Python model (sin_cos port checked against nine angle goldens; Fraction and Decimal): the digest
and goldens the tests pin; every derived bound held over 20 000 cases; no exact half.
Not run locally: wasm-pack (not installed; CI's WebAssembly job runs it), cargo deny and
markdownlint-cli2 (not installed; CI's Dojo job runs them), code-dojo-no-std (needs rustup;
CI runs it).

Known risks:
The type's band is narrower than FixedUnitVec3's, and the normalisation is not FixedUnitVec3's
rule; both are stated choices in the chapter and the pull request. `IDENTITY * q` renormalises
`q` and equals it only when `q` is exactly unit.

Human reviewer:
James Ross (maintainer); review pending.
