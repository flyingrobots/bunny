# Receipt: Convex Polygon Clipping Against A Plane

Task:
Implement flyingrobots/bunny#204: `FixedPlane3::clip_polygon`, Sutherland-Hodgman
against one plane into a caller-provided buffer. No existing function changes.

Files read:

- `crates/bunny-geom/src/plane.rs`
- `crates/bunny-geom/tests/plane_clip_tests.rs`
- `docs/topics/plane-clipping/README.md`
- `docs/topics/plane-clipping/test-plan.md`
- `docs/topics/repo-respect/README.md`
- `CODE_STANDARDS.md`
- `CONTRIBUTING.md`
- `docs/CODE_DOJO.md`
- `clippy.toml`

Files edited:

- `.repo-respect/receipts/2026-11-13-polygon-plane-clipping.md`
- `CHANGELOG.md`
- `crates/bunny-geom/README.md`
- `crates/bunny-geom/src/lib.rs`
- `crates/bunny-geom/src/plane.rs`
- `crates/bunny-geom/src/polygon.rs`
- `crates/bunny-geom/tests/polygon_clip_tests.rs`
- `docs/topics/plane-clipping/README.md`
- `docs/topics/plane-clipping/test-plan.md`

Topic docs:

- `docs/topics/plane-clipping/README.md`
- `docs/topics/plane-clipping/test-plan.md`

Generated artifacts:

- None.

Checks run:

- RED: `cargo test --locked -p bunny-geom --test polygon_clip_tests` failed with
  unresolved imports and missing `clip_polygon` before implementation.
- Final gates are recorded in the pull request.

Known risks:

- Cuts use the canonical edge direction (ascending raw x, y, z), so a cut equals
  `clip_segment` only for the canonically ordered segment; the maintainer may
  prefer another rule.
- A result with fewer than three vertices is `Outside`; the maintainer may
  prefer a distinct degenerate result.

Human reviewer:
Pending PR review.
