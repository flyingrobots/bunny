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
All inside flyingrobots/rust-dev:1.96.0 through devrun; nothing on the host except the
Python reference model that produced the model golden vectors and the composed paths.
cargo test --locked -p bunny-optics --test shading_tests --test shading_property_tests at
the test-only commit e58aca2: does not compile (E0432, the API is missing).
cargo fmt --all --check; cargo clippy --locked --workspace --all-targets --all-features
-- -D warnings; cargo test --locked -p bunny-optics -p bunny-perf; xtask topic-docs
(contract graph clean), after merging origin/main (7fdd2e4, with the pixel values).
A container-only measurement over 200 000 seeded normals (not committed): frame residuals
at most 4.3 (dot), 3.9 (squared length), 4.4 (cross) raw units; bounce direction length
within 3 raw units, deviation from the ideal map at most 2.1 raw units; light cosine
within 1.2 raw units. The committed bounds are 8, 8, 8, 8, 4 and 3.
Deliberate mutations (seven, listed in the test plan's Shading Mutation Checks), each
run with timeout 120: every one made at least one case fail; the restored source passed.
cargo run --locked -p xtask -- perf --base origin/main --rounds 11 --align-code
--filter shade/ (--exclusive), Apple M5 Pro in rust-dev:1.96.0, base 7fdd2e4: all three
cases are new (p50 84.11 ns surface_new, 26.90 ns bounce_direction, 73.73 ns
hit_shading); xtask reports that no case exists on both sides. In the same exclusive run,
bunny-perf --filter gave p50 19.65 ns for sample/sampler_point and 226.77 ns for
trace/closest_hit. A reading, not a speed claim.
code-dojo --all (history origin/main..HEAD) at 2ade1a1: full gate clean.
cargo test --locked --workspace and code-dojo --all at the final commit.
Not run locally: the headless WebAssembly tests (wasm-pack); CI runs them.

Known risks:
The contract choices (unit normals per triangle; the step-off-the-plane start rule instead
of primitive exclusion; the polar cosine-weighted map; the irradiance convention with
1 / pi; the order of operations) are the maintainer's to overrule before any artifact
records `bunny-diffuse-shading/1`. The distribution test is a regression guard, not a
statistical test.

Human reviewer:
James Ross (maintainer); review pending.
