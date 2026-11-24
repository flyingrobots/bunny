# Receipt: Closest Points Between Degenerate Segments

Task:
Fix `closest_points_segments` in `bunny-query` for degenerate and parallel inputs
(flyingrobots/bunny#506): a zero-length second segment returned `p1` whatever the point. A
zero-length segment is now a point answered by the existing point-to-segment path, and exactly
parallel segments take the existing parallel rule. Every pair of non-degenerate, non-parallel
segments keeps its previous answer bit for bit. The degeneracy policy of
flyingrobots/bunny#116 is not decided here.

Files read:
- `CONTRIBUTING.md`, `CODE_STANDARDS.md`, `.githooks/*`
- `.repo-respect/receipts/2026-11-24-uniform-grid-broadphase.md`
- `crates/bunny-query/src/closest.rs`, `crates/bunny-query/src/lib.rs`
- `crates/bunny-query/tests/closest_raw_tests.rs`, `query_tests.rs`, `property_tests.rs`
- `crates/bunny-geom/src/shapes/triangle.rs` (the exact cross-product pattern)
- `crates/bunny-num/src/fixed_q32_32.rs`, `crates/bunny-linalg/src/fixed_vec3.rs`
- `docs/topics/planar-geometry/README.md`, `docs/topics/lerp/test-plan.md`
- `xtask/src/topic_docs.rs` (case statuses)
- flyingrobots/bunny#116, draft flyingrobots/bunny#504 and draft flyingrobots/bunny#499

Files edited:
- `crates/bunny-query/src/closest.rs`
- `crates/bunny-query/tests/closest_segments_degenerate_tests.rs` (new)
- `docs/topics/closest-point/test-plan.md` (new: CLOSEST-REQ-010 to -014, CLOSEST-TP-020 to -028)
- `.repo-respect/receipts/2026-11-24-closest-points-degenerate-segments.md`

Topic docs:
New chapter `docs/topics/closest-point/`, created because no topic covered this function on
`main`. The IDs start at CLOSEST-REQ-010 and CLOSEST-TP-020 so as not to collide with
CLOSEST-REQ-001 to -004 and CLOSEST-TP-001 to -009 of draft flyingrobots/bunny#504.

Generated artifacts:
None.

Checks run:
- First commit (tests and planned cases only): `cargo test --locked -p bunny-query --test
  closest_segments_degenerate_tests` fails five of eight tests on the unfixed kernel
  (CLOSEST-TP-020, -021, -025, -026, -027).
- Zero-length fix: CLOSEST-TP-020, -021, -027 pass; -025 and -026 still fail on the parallel
  pairs.
- Parallel fix: all eight tests pass.

Known risks:
- A behaviour change to a public function for degenerate and exactly parallel inputs.

Human reviewer:
James Ross (maintainer); review pending.
