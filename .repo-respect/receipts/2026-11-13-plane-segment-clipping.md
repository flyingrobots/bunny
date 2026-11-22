# Receipt: Plane Classification And Segment Clipping

Task:
Implement flyingrobots/bunny#203: a fixed plane type, point classification,
and segment clipping in `bunny-geom`. No existing function changes.

Files read:

- `crates/bunny-geom/src/lib.rs`
- `crates/bunny-geom/tests/geom_tests.rs`
- `crates/bunny-num/src/fixed_q32_32.rs`
- `crates/bunny-linalg/src/fixed_vec3.rs`
- `docs/NUMERIC_CONSTITUTION.md`
- `docs/topics/matrix-types/README.md`
- `docs/topics/matrix-types/test-plan.md`
- `docs/topics/repo-respect/README.md`
- `CODE_STANDARDS.md`
- `CONTRIBUTING.md`
- `docs/CODE_DOJO.md`
- `clippy.toml`

Files edited:

- `.repo-respect/receipts/2026-11-13-plane-segment-clipping.md`
- `CHANGELOG.md`
- `crates/bunny-geom/README.md`
- `crates/bunny-geom/src/lib.rs`
- `crates/bunny-geom/src/plane.rs`
- `crates/bunny-geom/tests/plane_clip_tests.rs`
- `docs/MATH_GEOMETRY_CAPABILITY_MAP.md`
- `docs/README.md`
- `docs/topics/plane-clipping/README.md`
- `docs/topics/plane-clipping/test-plan.md`

Topic docs:

- `docs/topics/plane-clipping/README.md`
- `docs/topics/plane-clipping/test-plan.md`

Generated artifacts:

- None.

Checks run:

- RED: `cargo test --locked -p bunny-geom --test plane_clip_tests` failed with
  unresolved imports before implementation.
- GREEN: the same command passes 9 tests after implementation.
- Mutation: changing the keep rule `>= 0` to `> 0` made
  `pc_tp_004_touching_end_belongs_to_front` and
  `pc_tp_005_coplanar_segment_is_kept` fail; the rule was restored.
- `cargo clippy --locked -p bunny-geom --all-targets -- -D warnings`
- `cargo fmt --all`
- Final gates are recorded in the pull request.

Known risks:

- The clipped endpoint can lie behind the plane by rounding error; the bound is
  documented and tested, not repaired.
- Closed front half-space for on-plane points is a design choice the issue did
  not settle; the maintainer may overrule it.

Human reviewer:
Pending PR review.
