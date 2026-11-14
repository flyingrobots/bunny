# Receipt: Independent Near-Edge Oracle For hit_triangle

Task:
flyingrobots/bunny#294. Add an oracle for `TraceRay::hit_triangle` that is independent of
the kernel (signed volumes in 256-bit integers, its own floor division), run it on generated
cases concentrated on and next to edges and vertices with asserted minimum counts, and make
TRACE-TP-005 and TRACE-TP-203 count what they claim. Test-only: no library function changes.

Files read:
docs/topics/ray-tracing/README.md and test-plan.md
crates/bunny-query/src/trace.rs, src/barycentric.rs
crates/bunny-query/tests/trace_tests.rs, trace_equivalence_tests.rs, support/trace_reference.rs
flyingrobots/bunny#290 (the review that found the gap)

Files edited:
crates/bunny-query/tests/support/volume_oracle.rs (new)
crates/bunny-query/tests/trace_edge_oracle_tests.rs (new, TRACE-TP-315)
crates/bunny-query/tests/trace_tests.rs (TRACE-TP-005 compares with the oracle, counts hits)
crates/bunny-query/tests/trace_equivalence_tests.rs (TRACE-TP-203 counts crossings)
docs/topics/ray-tracing/test-plan.md (Fixtures, TRACE-TP-005, -203, -315, -316, mutation checks)
CHANGELOG.md (Unreleased)
.repo-respect/receipts/2026-11-14-trace-edge-oracle.md

Topic docs:
docs/topics/ray-tracing/test-plan.md. The chapter README describes kernel behaviour, which
is unchanged, so it is not edited.

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
Survival reproduced first at main: with the three edge comparisons relaxed by one numerator
unit, every test in bunny-query, bunny-broadphase and bunny-optics passed except TRACE-TP-201
(the frozen copy), under `timeout 120`, sources touched before each build.
After: each mutation in the test plan's edge-oracle table fails TRACE-TP-315; the restored
source passes. The generator and oracle were also written as an exact Python model, which
gives the same counts for every class.
cargo fmt, clippy -p bunny-query --all-targets -D warnings, the bunny-query tests.
code-dojo --all at the final commit (see the PR).

Known risks:
Numerator-unit near misses come from raw-unit geometry (translated to the domain corner or
not); in the scaled and long-direction families the near misses are raw units of the origin,
not numerator units. No shrinking: a failure prints the seed and case index.

Human reviewer:
James Ross (maintainer); review pending.
