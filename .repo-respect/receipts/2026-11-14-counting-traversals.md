# Receipt: Counting Traversals

Task:
Implement flyingrobots/bunny#335: counting forms of `compact_closest_hit` and
`compact_any_hit` that return the same result, error and primitive calls as the uncounted
functions together with a record of the work the search did (box tests, boxes entered,
skipped pops, interior and leaf visits, primitive tests, the deepest stack), for Lumen's
statistics report (flyingrobots/lumen#29) and BVH-cost view (flyingrobots/lumen#16). The
uncounted functions keep every output bit and their measured cost.

Files read:
crates/bunny-broadphase/src/compact_hit.rs
crates/bunny-broadphase/src/closest_hit.rs
crates/bunny-broadphase/src/traversal.rs
crates/bunny-broadphase/src/compact.rs
crates/bunny-broadphase/tests/compact_tests.rs
crates/bunny-broadphase/tests/traversal_guard_tests.rs
crates/bunny-perf/src/profile.rs
crates/bunny-perf/src/compact.rs
docs/topics/ray-tracing/README.md
docs/topics/ray-tracing/test-plan.md
docs/topics/performance/README.md
CODE_STANDARDS.md
CONTRIBUTING.md
docs/PROCESS.md

Files edited:
crates/bunny-broadphase/src/compact_hit.rs
crates/bunny-broadphase/src/counts.rs
crates/bunny-broadphase/src/lib.rs
crates/bunny-broadphase/tests/counted_traversal_tests.rs
crates/bunny-broadphase/README.md
docs/topics/ray-tracing/README.md
docs/topics/ray-tracing/test-plan.md
docs/topics/performance/README.md
CHANGELOG.md
.repo-respect/receipts/2026-11-14-counting-traversals.md

Topic docs:
docs/topics/ray-tracing/README.md (Counting Traversals: functions, counter definitions,
identities, sums, overflow argument, errors, which to call); docs/topics/ray-tracing/test-plan.md
(TRACE-REQ-033, TRACE-TP-412 to TRACE-TP-417, counting mutation checks; TRACE-TP-415 planned for
the follow-up that adds the bunny-perf group); docs/topics/performance/README.md (contract change
cost check, and the shared-loop design's readings).

Generated artifacts:
None.

Checks run:
The new tests against a stub whose counting tally records nothing (TRACE-TP-412 and 413 fail),
then against the implementation; the broadphase and bunny-perf tests; fmt and workspace Clippy;
the mutation runs under `timeout 120`; an A/A perf reading and the perf runs with `--exclusive`;
the full Code Dojo gate. Results are in the pull request.

Known risks:
The uncounted compact functions now run a loop generic over a zero-sized tally; their output is
unchanged by construction and their cost was measured, but a later compiler could inline the
shared loop differently. The counted forms are slower (measured in the follow-up). Only the
compact layout has counted forms.

Human reviewer:
James Ross (maintainer); review pending.
