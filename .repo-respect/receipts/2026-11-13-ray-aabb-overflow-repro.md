# Receipt: ray_intersects_aabb overflow reproduction and fix

Task:
Reproduce the suspected false miss in `ray_intersects_aabb` (flyingrobots/bunny#211), write a
proposal, and, after the maintainer chose option (a) as a declared contract change, fix it: a ray
whose slab distance or coordinate difference overflows is no longer reported as a miss. Every
input that returned `Some` before must keep its bits.

Files read:
CONTRIBUTING.md
CODE_STANDARDS.md
clippy.toml
docs/PROCESS.md
docs/CODE_DOJO.md
docs/TESTING.md
docs/README.md
docs/NUMERIC_CONSTITUTION.md
docs/topics/performance/README.md
docs/topics/performance/test-plan.md
docs/topics/repo-respect/README.md
docs/topics/ray-tracing/README.md
docs/topics/ray-tracing/test-plan.md
docs/design/0006-ray-aabb-slab-overflow.md
crates/bunny-query/src/ray.rs
crates/bunny-query/src/lib.rs
crates/bunny-query/README.md
crates/bunny-query/tests/query_tests.rs
crates/bunny-query/tests/determinism.rs
crates/bunny-query/tests/ray_determinism_tests.rs
crates/bunny-num/src/fixed_q32_32.rs
crates/bunny-linalg/src/lib.rs
crates/bunny-geom/src/lib.rs
crates/bunny-broadphase/src/traversal.rs
crates/bunny-broadphase/tests/bvh_tests.rs
crates/bunny-perf/src/cases.rs
xtask/src/code_dojo.rs
xtask/src/topic_docs.rs

Files edited:
crates/bunny-query/src/ray.rs
crates/bunny-query/src/ray/aabb_wide.rs
crates/bunny-query/README.md
crates/bunny-query/tests/ray_aabb_overflow_tests.rs
crates/bunny-query/tests/ray_aabb_oracle_tests.rs
crates/bunny-query/tests/query_tests.rs
crates/bunny-broadphase/tests/bvh_tests.rs
docs/design/0006-ray-aabb-slab-overflow.md
docs/topics/ray-tracing/README.md
docs/topics/ray-tracing/test-plan.md
docs/topics/performance/README.md
docs/NUMERIC_CONSTITUTION.md
CHANGELOG.md
.repo-respect/receipts/2026-11-13-ray-aabb-overflow-repro.md

Topic docs:
docs/topics/ray-tracing/ gains a "General-Purpose Ray-AABB Solver" section and requirements
TRACE-REQ-011 to TRACE-REQ-014 with cases TRACE-TP-013 to TRACE-TP-027. No topic covered the
general-purpose solvers in ray.rs; the ray-tracing chapter is the ray-related topic and already
uses an exact rational slab oracle, so the cases live there. The performance chapter records the
cost check of the common path. docs/NUMERIC_CONSTITUTION.md's overflow audit row is split.

Generated artifacts:
None changed. No golden vector, deterministic receipt test or generated manifest changed: the
golden ray/AABB vectors in crates/bunny-query/tests/determinism.rs and ray_determinism_tests.rs
are hits without overflow and pass unchanged, and code-dojo's manifest check passes with no diff.
One hand-written test expectation changed: the AABB assertion of
`query_tests.rs::ray_queries_return_none_when_checked_arithmetic_overflows` expected `None` for a
ray whose origin lies on the box's face; it is now its own test expecting `Some((MIN, 0))`.

Checks run:
RED: with the new tests and the old src/ (stashed), the flipped overflow tests, the oracle
corpora, the query_tests face case and the broadphase leaf test fail; the controls pass.
GREEN: cargo test --locked -p bunny-query and -p bunny-broadphase pass in the dev container.
Mutations of ray/aabb_wide.rs (truncated rounding, rounded hit decision, saturated difference,
dropped exit >= 0 check) each fail at least one case; see the ray-tracing test plan.
Perf: cargo run --locked -p xtask -- perf --base origin/main --rounds 11 --align-code
--filter query/, two runs against 2e92ce2748be: query/ray_aabb best p50 +0.2 % and -2.7 %,
inside noise (docs/topics/performance/README.md#contract-change-cost-checks).
Final: cargo test --locked --workspace and code-dojo --all (with --history origin/main..HEAD)
in the dev container at the final commit.

Known risks:
This is a declared contract change: outputs change from None to Some for the overflow inputs. A
caller that relied on None for those inputs (for example to detect far-away boxes) sees hits now.
The next release must use a version that signals the change. Ties to even versus ties away is not
observable through the public API (unit directions cannot produce an exact half), so that rule is
pinned only at the oracle level.

Human reviewer:
Pending: the maintainer reviews PR #236.
