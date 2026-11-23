# Receipt: Smooth Dielectric Sample

Task:
flyingrobots/bunny#372: `bunny-dielectric/1`, the reflected or refracted continuation ray at a
smooth boundary between two refractive indices. `SurfaceHit::dielectric_scatter` and
`try_dielectric_scatter` return a `DielectricScatter` (`Reflected` or `Refracted`, each a
`SpecularSample`); `dielectric_reflectance` returns `Total` or the exact unpolarized Fresnel
reflectance, which is the probability of reflection, so each branch's throughput is exactly
1.0. Total internal reflection is one exact 256-bit comparison; the refracted direction is
the unit vector along an exact vector with one floored root; its ray starts at the nearest
point strictly on the far side of the exact plane. `DielectricBoundary` holds the two indices
(`1.0..=8.0`), ordered by the hit's facing (`SurfaceHit::front_face`, now public). Two new
`SecondaryRayError` variants. No existing output changes.

Review follow-up (the independent review on flyingrobots/bunny#406): `Partial` no longer
claims a refracted ray always exists, and OPTICS-TP-176 pins both reflectance-of-1.0 inputs;
the radiance factor is stated in camera-path terms, `(a / b)^2` from index `a` into `b`, in
the rustdoc, the chapter, the crate README and the changelog; OPTICS-TP-163 covers indices one
raw unit apart at both ends of the range; OPTICS-TP-162 and -164 count equal and near-equal
indices; the start-point derivation names the `TraceRay::from_ray` case. `origin/main` merged,
keeping both sides; no dielectric kernel line changed.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, docs/NUMERIC_CONSTITUTION.md
docs/topics/optics/README.md and test-plan.md
`crates/bunny-optics/src/shading/**` (specular, hit, refusal, surface, frame, light, wide)
crates/bunny-optics/tests/exact/mod.rs, specular_reflection_tests.rs
crates/bunny-query/src/trace.rs and trace/wide.rs, crates/bunny-linalg (FixedUnitVec3,
FixedVec3::normalize), crates/bunny-num (sqrt_u128)
The diff of flyingrobots/bunny#376 and its independent review comment

Files edited:
crates/bunny-optics/src/shading/dielectric.rs, snell.rs, wide256.rs (new)
crates/bunny-optics/src/shading/hit.rs (plane offset and facing kept; `front_face`,
`far_point`), refusal.rs (two variants), specular.rs (`SpecularSample::new`), light.rs
(`Albedo::WHITE`, crate-private)
crates/bunny-optics/src/shading.rs, crates/bunny-optics/src/lib.rs
crates/bunny-optics/tests/dielectric_tests.rs, tests/dielectric_oracle/mod.rs (new)
crates/bunny-optics/README.md
docs/topics/optics/README.md (Status, Smooth Dielectric, Inputs And Refusals)
docs/topics/optics/test-plan.md (OPTICS-REQ-080 to -086; OPTICS-TP-160 to -175; the oracle;
mutation checks; open gaps)
CHANGELOG.md (Unreleased)
Review follow-up: dielectric.rs and hit.rs (rustdoc only), tests/dielectric_tests.rs
(OPTICS-TP-176; OPTICS-TP-162, -163, -164), the chapter (the radiance factor, a reflectance
of 1.0, the `from_ray` clause), test-plan.md (OPTICS-TP-176; OPTICS-TP-162 to -164), the crate
README and the changelog entry
.repo-respect/receipts/2026-11-16-optics-dielectric.md

Topic docs:
docs/topics/optics/README.md (Status, Smooth Dielectric) and test-plan.md

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
cargo test -p bunny-optics: every case passes, 11 in dielectric_tests and 4 unit cases.
The grazing case of OPTICS-TP-170 was found by a seeded search (a temporary test, not
committed); the test asserts its premise exactly.
Golden values marked "model" come from a Python model with unbounded integers.
Mutations (test plan, Dielectric Mutation Checks): 33, each under `timeout 600`; 29 fail at
least one case, 4 rounding choices pass and are listed as an open gap; the restored source
passes.
cargo fmt, clippy -D warnings; code-dojo --all at the final commit (see the PR).
Review follow-up, on the host after merging origin/main: cargo fmt --check; cargo test
--workspace --all-targets --all-features (12 dielectric cases); code-dojo-generated,
code-dojo-rust --all, code-dojo-determinism --enforce, code-dojo-perf-groups, topic-docs and
repo-respect check --branch. The mutation table was not rerun: no kernel line changed after
it was recorded (only rustdoc and tests).

Known risks:
The refraction is about the rounded unit frame normal, not the exact plane normal; a
near-grazing refraction can therefore stay above the plane and is refused.
Four stated roundings (the floored root, the floored ratios, the truncated prescale, the tie
rule) are fixed by the code but by no test: their effect is below `2^-26` of a raw unit.
The slab bound is derived to first order with a stated margin for the higher terms.
`wide256.rs` repeats the private 256-bit type of `bunny-query`.
The near-critical case of OPTICS-TP-176 was found by a search outside the repository; the
test asserts its premise (short of the critical angle) with the exact oracle.

Human reviewer:
James Ross (maintainer); review pending.
