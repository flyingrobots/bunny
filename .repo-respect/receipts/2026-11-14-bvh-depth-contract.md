# Receipt: The Depth Contract Between The BVH Builders And The Traversal Stack

Task:
flyingrobots/bunny#293, with the decision the maintainer's delegate made: restate the contract
so that "equals a test of every primitive" holds for every tree whose depth the traversal
stack holds, with the bound stated exactly for each traversal and for the compact conversion;
hold `build_bvh_sah` to depth 60 with tests that do not compare against the constant under
test; give `build_bvh` no cap, document what a caller gets, and settle whether inputs inside
the trace domain can make it exceed the stack. No production behaviour changes.

Files read:
crates/bunny-broadphase/src/ (bvh.rs, bvh/build.rs, bvh/split.rs, bvh/sah.rs,
bvh/sah/tests/mod.rs, traversal.rs, closest_hit.rs, compact.rs, compact_hit.rs)
crates/bunny-broadphase/tests/ (common/mod.rs, bvh_sah_tests.rs, compact_tests.rs,
traversal_guard_tests.rs)
docs/topics/bvh-construction/README.md and test-plan.md
docs/topics/ray-tracing/README.md and test-plan.md
CONTRIBUTING.md, docs/PROCESS.md, docs/CODE_DOJO.md

Files edited:
crates/bunny-broadphase/tests/traversal_depth_tests.rs (new: TRACE-TP-412, TRACE-TP-413)
crates/bunny-broadphase/tests/bvh_depth_tests.rs (new: BVH-TP-022, BVH-TP-024, BVH-TP-026)
crates/bunny-broadphase/tests/common/mod.rs, bvh_sah_tests.rs (depth bounds as literals)
crates/bunny-broadphase/src/ (documentation comments only: bvh.rs, compact.rs,
closest_hit.rs, compact_hit.rs, traversal.rs)
docs/topics/bvh-construction/README.md and test-plan.md
docs/topics/ray-tracing/README.md and test-plan.md
CHANGELOG.md (Unreleased)
.repo-respect/receipts/2026-11-14-bvh-depth-contract.md

Topic docs:
docs/topics/bvh-construction/ and docs/topics/ray-tracing/ (README.md and test-plan.md)

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
Depth search: an exploratory test (not committed) measured builds of candidate inputs. For
build_bvh inside the trace domain the deepest tree found is 90 (94 collinear point boxes,
ratio 4/3); thin triangles give 70, two axes 83, three 84; earlier review constructions
peaked at 60. For build_bvh_sah: the review's 262 189-box witness takes 14.2 s in a debug build
(depth 60, 32 768 leaves of 8 at depth 60); 70 in-domain collinear points reach the cap in
about 3 ms; the build stack fills only on a left spine, which needs out-of-domain 17^k chains
plus 2^14 identical boxes (0.94 s).
Mutations, each under `timeout 120` with every source touched before the build:
SAH_MAX_DEPTH = 64 fails BVH-TP-022; the SAH build stack of SAH_MAX_DEPTH entries fails
BVH-TP-026; every traversal stack and the conversion queue moved by one each way fails
TRACE-TP-412 or -413; the box queries' `>=` capacity check made `>` is equivalent. The
restored source passes every bunny-broadphase test. One run's results were discarded: a
script bug mutated the wrong file for three mutants, and a test then built a 786 KB array on
the test thread's stack; both were fixed and the affected mutants rerun.
cargo fmt --all --check, cargo clippy -p bunny-broadphase --all-targets -D warnings, cargo
test -p bunny-broadphase; code-dojo --all at the final commit (see the PR).

Known risks:
The depth search is a targeted search, not a proof: deeper in-domain build_bvh trees may
exist. The triangles' depth 70 and BVH-TP-026's depth and left-turn count are recorded values
of the current builders, not hand derivations. BVH-TP-026 takes about 0.9 s in a debug build,
and its input is outside the trace domain (no in-domain input filling the build stack was
found). The traversal bound d + 1 is argued from the stack discipline and checked only at the
comb boundary.

Human reviewer:
James Ross (maintainer); review pending.
