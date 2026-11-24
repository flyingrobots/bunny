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
- `crates/bunny-query/src/closest.rs`
- `crates/bunny-query/tests/closest_triangle_degenerate_tests.rs`
- `crates/bunny-query/README.md`
- `docs/topics/closest-point/README.md`
- `docs/topics/closest-point/test-plan.md`
- `docs/README.md`
- `CHANGELOG.md`
- `.repo-respect/receipts/2026-11-24-closest-point-degenerate-triangle.md`

Topic docs:
New chapter `docs/topics/closest-point/` (CLOSEST-REQ-001 to -004, CLOSEST-TP-001 to -009), with one row in `docs/README.md` in sorted position. No existing requirement or case ID changed.

Generated artifacts:
None.

Checks run:
- `cargo test --locked -p bunny-query --test closest_triangle_degenerate_tests`: five tests failed on the unfixed kernel (first commit), all eight pass with the fix.
- Eight mutations of the fix, recorded in `docs/topics/closest-point/test-plan.md#mutation-checks`; each fails at least one case.
- `cargo fmt --all --check`
- `cargo test --locked --workspace --all-targets --all-features`
- `cargo test --locked -p bunny-query --no-default-features`
- `cargo clippy --locked -p bunny-query --all-targets --all-features -- -D warnings`
- `cargo run --locked -p xtask -- code-dojo-generated`
- `cargo run --locked -p xtask -- code-dojo-rust --all`
- `cargo run --locked -p xtask -- code-dojo-determinism --enforce`
- `cargo run --locked -p xtask -- code-dojo-perf-groups`
- `cargo run --locked -p xtask -- topic-docs`
- `cargo run --locked -p xtask -- repo-respect check --branch`

Known risks:
- A behaviour change to a public function for degenerate inputs; a caller relying on the previous answer sees a different value.
- Collinear distinct vertices are included although flyingrobots/bunny#500 left their scope to flyingrobots/bunny#116; the previous answers were outside the bound on some of them.
- Slivers (one raw unit off a line) are not degenerate and keep their previous, sometimes wrong, answers by design.
- Saturated answers above `2^45` raw remain (decision D5 of flyingrobots/bunny#116).

Human reviewer:
@flyingrobots
