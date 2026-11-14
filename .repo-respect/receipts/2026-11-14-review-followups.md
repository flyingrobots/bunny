# Receipt: Review Follow-Ups For Bounds, Ties And Ledger Wording

Task:
Implement flyingrobots/bunny#351: pin the compact searches' visit bound, state the
`smooth_normal` bound's shared-normal case with its derivation and a worst-case test,
correct the counting entry's "unchanged" and its x86-64 citation, fix the release-packaging
wording, and resolve whether `FixedAngle`'s tie rule is reachable. Also, from the same
review: compiling examples for the smooth-shading, counted-traversal and PLY-limit APIs, and
the regeneration check's fixed output list. No output bit of any function changes.

Files read:
crates/bunny-broadphase/src/compact_hit.rs, crates/bunny-broadphase/src/compact.rs,
crates/bunny-broadphase/tests/compact_tests.rs, crates/bunny-broadphase/tests/traversal_guard_tests.rs
crates/bunny-optics/src/shading/smooth_normal.rs, crates/bunny-optics/tests/smooth_normal_tests.rs,
crates/bunny-optics/tests/exact/mod.rs
crates/bunny-linalg/src/lib.rs (the `FixedUnitVec3` length check)
crates/bunny-num/src/fixed_angle.rs, crates/bunny-num/tests/fixed_angle_tests.rs
crates/bunny-codec/src/ply_ascii/limits.rs
xtask/src/generated_check.rs
docs/topics/optics/README.md, docs/topics/ray-tracing/README.md,
docs/topics/performance/README.md, docs/topics/release-packaging/README.md,
docs/topics/angle-trigonometry/README.md, docs/topics/contract-generation/README.md and the
test plans of optics, ray-tracing and angle-trigonometry
The reviewer's oracle (smooth_oracle.py, smooth.txt) and mutation script

Files edited:
crates/bunny-broadphase/src/compact_hit.rs (doc example only)
crates/bunny-broadphase/tests/compact_tests.rs
crates/bunny-optics/src/shading/smooth_normal.rs (documentation and doc example only)
crates/bunny-optics/tests/smooth_normal_tests.rs
crates/bunny-num/tests/fixed_angle_tests.rs
crates/bunny-codec/src/ply_ascii/limits.rs (doc example only)
docs/topics/optics/README.md, docs/topics/optics/test-plan.md
docs/topics/ray-tracing/README.md, docs/topics/ray-tracing/test-plan.md
docs/topics/performance/README.md
docs/topics/release-packaging/README.md
docs/topics/angle-trigonometry/README.md, docs/topics/angle-trigonometry/test-plan.md
docs/topics/contract-generation/README.md
CHANGELOG.md
.repo-respect/receipts/2026-11-14-review-followups.md

Topic docs:
docs/topics/optics/README.md#smooth-normal (the shared-normal bound, `None` after the shift)
docs/topics/ray-tracing/README.md#counting-traversals; the performance ledger entry
docs/topics/angle-trigonometry/README.md (ties occur); test plans: TRACE-TP-421,
OPTICS-TP-099, AT-REQ-009 and AT-TP-009
docs/topics/contract-generation/README.md#regeneration-check (what is compared)

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
Each claim of the issue was checked first. The compact bound is `visits > nodes.len()`
(`compact_hit.rs`), with no test at the boundary. The ledger's numbers are as the issue
quotes them; no x86-64 numbers are recorded on the pull request. The release table says
"Builds". The shared-normal step returns its input; the `FixedUnitVec3` check accepts a
floored length of `2^32 - 1` to `2^32 + 1`, so the length is below `2^32 + 2` and a
component's error below 2 raw units, which the reviewer's 1.86 respects. FixedAngle ties: a
bisection over a replica of `sin_quarter`'s interpolation (a temporary test, not committed)
found exact ties at both parities in every segment tried (segments 1 and 100 shown), so
the tie is reachable and the mutation is not equivalent.
`cargo test --locked` of `bunny-broadphase`, `bunny-optics`, `bunny-num` and `bunny-codec`,
sources touched: all pass, with the three new tests and the three new doc examples; Clippy
with `-D warnings` on the four crates, `code-dojo-rust --all` and `topic-docs`: clean. The
new tests describe the code as it is, so they pass on main; their evidence is the mutants
they kill.
Mutations, each under `timeout 120` after touching the sources, all caught: the compact bound
loosened by one (survived every test before; now TRACE-TP-421 fails) and tightened by one;
`q62_to_q32` rounding ties away from zero (survived every `bunny-num` test before; now
AT-TP-009 fails) and toward zero.
The full gate, `code-dojo --all` with `--history origin/main..HEAD --network`, at the head
commit: see the pull request.

Known risks:
The tie positions come from a replica of the interpolation in a throwaway probe; the
expected values follow from the rounding rule alone. The x86-64 cost of the counting change
is not recorded.

Human reviewer:
James Ross (maintainer); review pending.
