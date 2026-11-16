# Receipt: Texture Coordinates And Bilinear Texel Weights

Task:
flyingrobots/bunny#367: `bunny-texture-bilinear/1`, the texture coordinates at a hit
interpolated from the hit's barycentric numerators, and the four texels and weights of a
bilinear lookup with repeat and clamp wrapping. `interpolate_uv` rounds each component of the
exact convex combination once (exact u128 long division); `bilinear(uv, [width, height], wrap)`
returns a `BilinearTaps` whose weights sum to exactly 1.0. No existing output changes.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, docs/NUMERIC_CONSTITUTION.md
docs/topics/optics/README.md and test-plan.md
crates/bunny-query/src/barycentric.rs
crates/bunny-optics/src/shading/smooth_normal.rs, smooth_hit.rs, specular.rs
crates/bunny-optics/tests/exact/mod.rs, smooth_normal_tests.rs, specular_reflection_tests.rs

Files edited:
crates/bunny-optics/src/texture.rs (new), crates/bunny-optics/src/lib.rs
crates/bunny-optics/tests/texture_bilinear_tests.rs (new)
crates/bunny-optics/README.md
docs/topics/optics/README.md (Status, Texture Coordinates)
docs/topics/optics/test-plan.md (Scope; OPTICS-REQ-056 to -059; OPTICS-TP-110 to -115;
mutation checks)
CHANGELOG.md (Unreleased)
.repo-respect/receipts/2026-11-15-optics-texture-bilinear.md

Topic docs:
docs/topics/optics/README.md and test-plan.md (Status, Texture Coordinates) and test-plan.md

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
cargo test -p bunny-optics --test texture_bilinear_tests: 5 cases pass; doctests pass.
Mutations (test plan, Texture-Bilinear Mutation Checks): nine, each fails at least one case;
the restored source passes.
cargo fmt, clippy -D warnings; code-dojo --all steps at the final commit (see the PR).

Known risks:
Row 0 is at v = 0 (no flip); a caller with top-row-first images must orient them itself. The
issue sketched a five-argument `bilinear`; the workspace's four-argument clippy limit made it
`bilinear(uv, [width, height], wrap)`. Test-plan IDs OPTICS-REQ-056.. and OPTICS-TP-110.. may
collide with parallel optics work.

Human reviewer:
James Ross (maintainer); review pending.
