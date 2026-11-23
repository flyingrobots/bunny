# Receipt: Transform Composition

Task:
flyingrobots/bunny#312: `FixedAffine3::from_trs(translation, rotation, scale)` (`T * R * S`,
linear part `R * diag(scale)`), `rotate_about_point(pivot, rotation)` (`T(p) * R * T(-p)`) and
`scale_about_point(pivot, scale)` under the contract identity `bunny-transform/1`
(`TRANSFORM_CONTRACT_ID`). Overflow is `None`; zero and negative scales are accepted. Column
vectors, row-major entries and right-to-left composition, as the matrix types, the coordinate
law and `bunny-rotation/1` document. Additive: no existing public item, output bit, golden,
digest or ID changes; `checked_mul_affine` gains a documented bound, not new behaviour.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*
.repo-respect/receipts/2026-11-23-rotation.md
crates/bunny-linalg/src/{lib.rs,fixed_affine3.rs,fixed_mat3.rs,fixed_mat3_rotation.rs,fixed_vec3.rs}
crates/bunny-linalg/tests/{rotation_tests.rs,affine_transform_tests.rs}
crates/bunny-num/src/fixed_q32_32.rs (checked_mul and its rounding)
xtask/src/topic_docs.rs (ledger rules)
docs/topics/{matrix-types,rotation}/README.md and test plans
flyingrobots/bunny#312, #311 (closed) and #108

Files edited:
crates/bunny-linalg/src/fixed_affine3_trs.rs (new: the constructors and TRANSFORM_CONTRACT_ID)
crates/bunny-linalg/src/lib.rs (module and export)
crates/bunny-linalg/tests/transform_tests.rs (new)
crates/bunny-linalg/README.md, CHANGELOG.md, docs/README.md (topic index)
docs/topics/transform/README.md (new chapter)
docs/topics/transform/test-plan.md (new: TRS-REQ-001 to -005, TRS-TP-001 to -009; mutations)
docs/topics/matrix-types/README.md (one cross-link; no matrix behaviour changes)
.repo-respect/receipts/2026-11-23-transform-composition.md

Topic docs:
docs/topics/transform/ is a new topic with its own TRS- prefix; no other topic uses it.

Generated artifacts:
None.

Checks run:
First commit (tests and planned cases only): the transform tests fail to compile (the
constructors do not exist); topic-docs and repo-respect check --branch clean.
Kernel commit, each its own command on the host (local Rust 1.100.0; CI uses the pinned 1.96.0):
cargo fmt --all --check; cargo test --locked --workspace --all-targets --all-features;
cargo run --locked -p xtask -- code-dojo-generated; code-dojo-rust --all;
code-dojo-determinism --enforce; code-dojo-perf-groups; topic-docs; repo-respect check
--branch. cargo clippy --locked -p bunny-linalg --lib --test transform_tests -D warnings clean.
Mutations: eight, each applied to the kernel, run with cargo test --locked -q -p bunny-linalg
--no-fail-fast --test transform_tests --test affine_transform_tests, and restored; each failed
at least one case (the test plan's table).
Python model (unbounded integers, Fraction bounds): the goldens and from_trs thresholds the
tests pin.
Not run locally: wasm-pack (not installed; CI's WebAssembly job runs it), cargo deny and
markdownlint-cli2 (not installed; CI's Dojo job runs them).

Known risks:
`rotate_about_point` inherits `checked_mul_vec3`'s refusal of an out-of-range partial sum, so
a few inputs whose finished translation would fit are `None`; that keeps the pivot exactly
fixed and matches the literal `T(p) * R * T(-p)` composition. The chapter says so.

Human reviewer:
James Ross (maintainer); review pending.
