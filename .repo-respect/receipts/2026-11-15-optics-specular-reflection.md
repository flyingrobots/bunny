# Receipt: Specular Reflection Sample

Task:
flyingrobots/bunny#370: `bunny-specular-reflection/1`, the continuation ray and throughput of a
perfect mirror at a `SurfaceHit`. `SurfaceHit::specular_reflection` and
`try_specular_reflection` return a `SpecularSample` (ray from the start point, throughput per
channel); the direction is the exact `i128` reflection about the frame normal, normalized once;
the throughput is `schlick_fresnel(F0_c, cos)`, with the exact `(1 - cos)^5` and the sum each
rounded once. A ray arriving from below the face and a grazing reflection that stays under the
exact plane are refused with two new `SecondaryRayError` variants. No existing output changes.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, docs/NUMERIC_CONSTITUTION.md
docs/topics/optics/README.md and test-plan.md
`crates/bunny-optics/src/shading/**` (hit, frame, light, refusal, surface, smooth_hit, wide)
crates/bunny-optics/tests/exact/mod.rs, shading_property_tests.rs, shading_refusal_tests.rs,
smooth_normal_tests.rs
crates/bunny-query/src/trace.rs, crates/bunny-linalg/src/lib.rs (FixedUnitVec3)

Files edited:
crates/bunny-optics/src/shading/specular.rs (new)
crates/bunny-optics/src/shading/refusal.rs (two variants, doc of BounceRayRefused)
crates/bunny-optics/src/shading.rs, crates/bunny-optics/src/lib.rs
crates/bunny-optics/tests/specular_reflection_tests.rs (new)
crates/bunny-optics/README.md
docs/topics/optics/README.md (Status, Specular Reflection)
docs/topics/optics/test-plan.md (OPTICS-REQ-052 to -055; OPTICS-TP-100 to -108; mutation
checks; an open gap)
CHANGELOG.md (Unreleased)
.repo-respect/receipts/2026-11-15-optics-specular-reflection.md

Topic docs:
docs/topics/optics/README.md (Status, Specular Reflection) and test-plan.md

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
cargo test -p bunny-optics --test specular_reflection_tests: 8 cases pass.
The grazing case of OPTICS-TP-107 was found by a seeded search (a temporary ignored test, not
committed); the test asserts its premise exactly.
Golden values marked "model" come from a Python model with unbounded integers.
Mutations (test plan, Specular-Reflection Mutation Checks): eight, each under `timeout 120`;
each fails at least one case; the restored source passes.
cargo fmt, clippy -D warnings; code-dojo --all at the final commit (see the PR).

Known risks:
The reflection is about the rounded unit frame normal, not the exact plane normal; a grazing
ray can therefore reflect under the plane and is refused (the caller treats it as dark).
Reflection about a smooth-shading normal is not part of this identity.

Human reviewer:
James Ross (maintainer); review pending.
