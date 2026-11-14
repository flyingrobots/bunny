# Receipt: Deterministic Diffuse Shading For A One-Bounce Path Tracer

Task:
Implement the diffuse-shading slice of flyingrobots/bunny#160 in `bunny-optics`: a
triangle's geometric normal and frame, the facing rule, the start rule for secondary rays,
the cosine-weighted hemisphere direction, the directional light's term and shadow ray, the
sky, the diffuse reflection, a stated radiometric convention and a versioned identity
(`bunny-diffuse-shading/1`) with a reference order of operations, all integer-only, with
golden vectors, stated bounds and oracle tests. Reflection, refraction and other material
helpers in the issue's original scope are not part of this slice; no existing function
changes.

Files read:
CODE_STANDARDS.md
CONTRIBUTING.md
CHANGELOG.md
clippy.toml
docs/CODE_DOJO.md
docs/NUMERIC_CONSTITUTION.md
docs/topics/optics/README.md
docs/topics/optics/test-plan.md
docs/topics/ray-tracing/README.md
docs/topics/performance/README.md
crates/bunny-optics/src/*.rs
crates/bunny-optics/src/pixel/*.rs
crates/bunny-optics/tests/*.rs
crates/bunny-linalg/src/lib.rs
crates/bunny-linalg/src/fixed_vec3.rs
crates/bunny-num/src/fixed_q32_32.rs
crates/bunny-num/src/fixed_angle.rs
crates/bunny-num/tests/fixed_angle_tests.rs
crates/bunny-query/src/trace.rs
crates/bunny-broadphase/src/closest_hit.rs
crates/bunny-perf/src/*.rs
xtask/src/code_dojo.rs
xtask/src/topic_docs.rs
Consumer design (read only, for context): Geordi's fixed-point and ray-traced scene
profiles, sections B2, B4, B5 and the Decisions.

Files edited:
crates/bunny-optics/src/shading.rs
crates/bunny-optics/src/shading/frame.rs
crates/bunny-optics/src/shading/hit.rs
crates/bunny-optics/src/shading/light.rs
crates/bunny-optics/src/shading/surface.rs
crates/bunny-optics/src/shading/wide.rs
crates/bunny-optics/src/lib.rs
crates/bunny-optics/tests/shading_tests.rs
crates/bunny-optics/tests/shading_property_tests.rs
crates/bunny-optics/Cargo.toml
crates/bunny-optics/README.md
crates/bunny-perf/Cargo.toml
crates/bunny-perf/src/cases.rs
crates/bunny-perf/src/main.rs
crates/bunny-perf/src/shade.rs
docs/topics/optics/README.md
docs/topics/optics/test-plan.md
docs/topics/performance/README.md
docs/MATH_GEOMETRY_CAPABILITY_MAP.md
docs/TESTING.md
README.md
CHANGELOG.md
.repo-respect/receipts/2026-11-13-optics-shading.md

Topic docs:
docs/topics/optics/README.md (Diffuse Shading: identity, ownership, geometric normal,
orthonormal frame, facing and secondary rays, cosine-weighted hemisphere, radiometric
convention, directional light, sky, diffuse reflection, order of operations, inputs and
refusals, accuracy and cost)
docs/topics/optics/test-plan.md (OPTICS-REQ-030..040, OPTICS-TP-040..056, the shading
reference model, shading mutations)
docs/topics/performance/README.md (the `shade` harness group)

Generated artifacts:
None.

Checks run:
Recorded in the final commit of this receipt.

Known risks:
The contract choices (unit normals per triangle; the step-off-the-plane start rule instead
of primitive exclusion; the polar cosine-weighted map; the irradiance convention with
1 / pi; the order of operations) are the maintainer's to overrule before any artifact
records `bunny-diffuse-shading/1`. The distribution test is a regression guard, not a
statistical test.

Human reviewer:
James Ross (maintainer); review pending.
