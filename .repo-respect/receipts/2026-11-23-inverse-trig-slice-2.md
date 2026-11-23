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
crates/bunny-num/tests/angle_oracle/mod.rs (visibility only: FRAC, ONE, ratio, atan,
turn_units become pub(crate); no oracle arithmetic changes)
crates/bunny-num/src/fixed_angle/inverse_ratio.rs (new: atan, asin, acos)
crates/bunny-num/src/fixed_angle/conversions_out.rs (signed conversions out)
crates/bunny-num/src/fixed_angle/atan2.rs (refactor: the octant fold becomes
direction_angle, and first_octant takes u128 magnitudes up to 2^64; atan2's bits are
unchanged, pinned by AT-TP-013 to AT-TP-016)
crates/bunny-num/src/fixed_angle.rs (module declaration)
docs/topics/angle-trigonometry/test-plan.md (AT-REQ-020 to -026, AT-TP-030 to -039)
docs/topics/angle-trigonometry/README.md (Signed Conversions Out, Single-Ratio Atan,
Asin And Acos; Non-Goals)
docs/README.md (topic row), crates/bunny-num/README.md, CHANGELOG.md
.repo-respect/receipts/2026-11-23-inverse-trig-slice-2.md

Topic docs:
docs/topics/angle-trigonometry/: AT-REQ-020 to AT-REQ-026 and AT-TP-030 to AT-TP-039 in the
range assigned to this slice; topic-docs reports zero duplicate IDs.

Generated artifacts:
None. Golden vectors are inline in the test file, recorded from the kernel and checked
against a 60-digit Python decimal model posted on the pull request, not committed.

Checks run:
First commit (tests and planned cases only): the new test target fails to compile because
the kernel functions do not exist yet; cargo run --locked -p xtask -- topic-docs clean.
Kernel commit, each its own command on the host (local Rust 1.100.0; CI pins 1.96.0):
cargo fmt --all --check; cargo test --locked --workspace --all-targets --all-features
(122 test binaries, 888 passed, 0 failed); cargo run --locked -p xtask --
code-dojo-generated; code-dojo-rust --all; code-dojo-determinism --enforce;
code-dojo-perf-groups; topic-docs; repo-respect check --branch.
cargo clippy --locked -p bunny-num --all-targets --all-features -- -D warnings clean.
cargo build and cargo test --locked -p bunny-num --no-default-features natively (the
bare-metal code-dojo-no-std gate needs rustup and runs only in CI).
Mutations: nineteen, each applied to the kernel, run with cargo test --locked -q
--no-fail-fast -p bunny-num --test fixed_angle_inverse_ratio_tests --test
fixed_angle_inverse_tests, and restored; each failed at least one case (the test plan's
table).
Not run locally: wasm-pack (CI's WebAssembly job), cargo deny and markdownlint-cli2 (CI's
Dojo job).

Known risks:
`asin` and `acos` are not a single rounding of the exact angle: the cosine leg is a floor
square root, so the bound is 2/3 of an angle unit rather than 1/2 + 2^-9.

Human reviewer:
James Ross (maintainer); review pending.
