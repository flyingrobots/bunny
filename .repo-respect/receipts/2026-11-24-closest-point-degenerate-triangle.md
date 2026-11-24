# Receipt: Closest Point On A Degenerate Triangle

Task:
Fix `closest_point_triangle` in `bunny-query` for degenerate triangles (flyingrobots/bunny#500): a triangle with a repeated vertex, three equal vertices, or collinear distinct vertices is the segment between its two farthest vertices, or a point, and the answer is the closest point of that segment, computed by the existing `closest_points_segments`. Every non-degenerate triangle keeps its previous answer bit for bit. The degeneracy policy of flyingrobots/bunny#116 is not decided here.

Files read:
- `crates/bunny-query/src/closest.rs`
- `crates/bunny-query/tests/closest_raw_tests.rs`
- `crates/bunny-query/tests/query_tests.rs`
- `crates/bunny-query/tests/trace_edge_oracle_tests.rs`
- `crates/bunny-geom/src/shapes/triangle.rs`
- `crates/bunny-linalg/src/fixed_vec3.rs`
- `crates/bunny-num/src/fixed_q32_32.rs`
- `docs/NUMERIC_CONSTITUTION.md`
- `docs/TECHNICAL_TEARDOWN.md`
- `docs/topics/lerp/test-plan.md`

Files edited:
- `crates/bunny-query/tests/closest_triangle_degenerate_tests.rs`
- `.repo-respect/receipts/2026-11-24-closest-point-degenerate-triangle.md`

Topic docs:
None yet: the first commit adds the failing tests only.

Generated artifacts:
None.

Checks run:
- `cargo test --locked -p bunny-query --test closest_triangle_degenerate_tests`: five tests fail on the unfixed kernel, as intended.
- `cargo run --locked -p xtask -- repo-respect check --branch`

Known risks:
- A behaviour change to a public function for degenerate inputs; a caller relying on the previous answer sees a different value.

Human reviewer:
@flyingrobots
