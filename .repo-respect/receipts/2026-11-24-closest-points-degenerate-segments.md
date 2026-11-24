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
- `crates/bunny-query/src/closest.rs` (`closest_points_segments`, its new private helpers
  `segment_pair`, `exactly_parallel`, `raw_difference`, `same_product`, `exact_product`, and
  one argument added to the private `SegmentParams::new`)
- `crates/bunny-query/tests/closest_segments_degenerate_tests.rs` (new)
- `crates/bunny-query/README.md`
- `docs/topics/closest-point/README.md` (new chapter)
- `docs/topics/closest-point/test-plan.md` (new: CLOSEST-REQ-010 to -014, CLOSEST-TP-020 to -028)
- `docs/README.md` (one row, sorted position)
- `CHANGELOG.md` (Fixed, with the behaviour change)
- `.repo-respect/receipts/2026-11-24-closest-points-degenerate-segments.md`

Topic docs:
New chapter `docs/topics/closest-point/`, created because no topic covered this function on
`main`. The IDs start at CLOSEST-REQ-010 and CLOSEST-TP-020 so as not to collide with
CLOSEST-REQ-001 to -004 and CLOSEST-TP-001 to -009 of draft flyingrobots/bunny#504, which
creates the same chapter for `closest_point_triangle`; whichever merges second combines the
two chapters.

Generated artifacts:
None.

Checks run:
- First commit (tests and planned cases only): `cargo test --locked -p bunny-query --test
  closest_segments_degenerate_tests` fails five of eight tests on the unfixed kernel
  (CLOSEST-TP-020, -021, -025, -026, -027).
- Zero-length fix: CLOSEST-TP-020, -021, -027 pass; -025 and -026 still fail on the parallel
  pairs.
- Parallel fix: all eight tests pass.
- Nine mutations of the fix, recorded in `docs/topics/closest-point/test-plan.md#mutation-checks`;
  eight fail at least one case, and the one that survives is recorded with the reason.
- Each as its own command (local Rust 1.100.0; CI pins 1.96.0): `cargo fmt --all --check`;
  `cargo test --locked --workspace --all-targets --all-features`; `cargo test --locked -p
  bunny-query --no-default-features`; `cargo clippy --locked -p bunny-query --all-targets
  --all-features -- -D warnings`; `RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps -p
  bunny-query`; `cargo run --locked -p xtask --` `code-dojo-generated`, `code-dojo-rust
  --all`, `code-dojo-determinism --enforce`, `code-dojo-perf-groups`, `topic-docs`,
  `repo-respect check --branch`.
- Not run locally: wasm-pack (CI's WebAssembly job), `code-dojo-no-std` (needs `rustup`),
  cargo deny, markdownlint-cli2 and the pinned 1.96.0 clippy.

Known risks:
- A behaviour change to a public function for a zero-length second segment and for exactly
  parallel segments with a nonzero rounded denominator. A caller relying on the previous
  answer sees a different value.
- The parallel reroute also moves answers that were already a closest pair (162 parallel and
  516 collinear of the sweep), because the routine cannot tell them apart from the wrong ones
  without an exact oracle. It is its own commit, so it can be dropped on its own.
- Non-parallel segments keep their previous answers by contract, including one shared-end
  pair 99 716 raw units off in the sweep.
- Saturated answers above `2^45` raw remain (decision D5 of flyingrobots/bunny#116).
- The exact parallel test adds six 128-bit products to every call whose rounded denominator
  is not zero. No performance claim is made.

Human reviewer:
James Ross (maintainer); review pending.
