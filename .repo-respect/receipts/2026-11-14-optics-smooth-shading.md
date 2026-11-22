# Receipt: Smooth-Shaded Surface Hits

Task:
Part 3 of flyingrobots/bunny#289 (smooth shading normals): `SmoothHit`
(`bunny-smooth-shading/1`), a `SurfaceHit` with a shading frame around the interpolated vertex
normal. The frame is turned to the arrival side by the sign of an exact dot product with the
face's exact plane normal and falls back to the geometric frame when that product is zero, a
vertex normal is missing or the normals cancel; the light term keeps the geometric shadow ray and
side test and uses the shading cosine, clamped at zero. The bounce direction and its rejection
below the geometric plane, the wider oracle for `smooth_normal`, and the benchmark are later
parts. No existing output changes.

Files read:
CODE_STANDARDS.md
docs/topics/optics/README.md and test-plan.md
`crates/bunny-optics/src/shading/**` (hit, frame, light, smooth_normal, wide)
crates/bunny-query/src/barycentric.rs

Files edited:
crates/bunny-optics/src/shading/smooth_hit.rs (new)
crates/bunny-optics/src/shading/hit.rs (a crate-private accessor for the exact facing normal)
crates/bunny-optics/src/shading.rs, crates/bunny-optics/src/lib.rs
crates/bunny-optics/tests/smooth_hit_tests.rs (new)
crates/bunny-optics/README.md
docs/topics/optics/README.md (Smooth Shading; the shared-normal jump in Smooth Normal)
docs/topics/optics/test-plan.md (OPTICS-REQ-049, -050; OPTICS-TP-079 to -082)
CHANGELOG.md (Unreleased)
.repo-respect/receipts/2026-11-14-optics-smooth-shading.md

Topic docs:
docs/topics/optics/README.md (Status, Smooth Normal, Smooth Shading)

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
Red first: the new tests fail to build on the crate without `SmoothHit` and `SMOOTH_SHADING_ID`
(unresolved imports).
Green: the three new cases pass. The grazing case of OPTICS-TP-080 was found by a search in a
separate Python model (exact dot product negative, dot product with the rounded unit normal
positive); the test asserts both values.
Mutations (test plan, Smooth-Shading Mutation Checks): six, each with `timeout 120`; each fails
at least one case; the restored source passes.
cargo fmt, clippy -D warnings.
code-dojo --all at the final commit (see the PR).

Known risks:
A missing vertex normal falls back to the geometric frame even when its weight is zero. The
radiance expectations are compositions of existing public functions, not independent values.

Human reviewer:
James Ross (maintainer); review pending.
