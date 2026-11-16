# Receipt: GGX Rough-Conductor Sample

Task:
flyingrobots/bunny#371: `bunny-ggx-conductor/1`, one sample of a GGX rough conductor at a
`SurfaceHit`. `SurfaceHit::ggx_reflection` and `try_ggx_reflection` draw a microfacet normal
from the distribution of visible normals (Heitz 2018) from one sampler pair, reflect the
incoming ray about it and weight the sample by Schlick's Fresnel term times the
height-correlated Smith ratio `G2 / G1`. The result is a `GgxScatter`: `Reflected` with a
`SpecularSample`, or `Absorbed`. `GgxConductor` holds the roughness (`2^-16..=1.0`) and the
base reflectance. `reflect` becomes public and is the one reflection the mirror and the
sampler share. No existing output changes.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, docs/NUMERIC_CONSTITUTION.md, docs/CODE_DOJO.md
docs/topics/optics/README.md and test-plan.md
`crates/bunny-optics/src/shading/**` (specular, refusal, hit, frame, wide, light,
vertex_normals and its tests)
crates/bunny-optics/tests/specular_reflection_tests.rs, tests/exact/mod.rs
crates/bunny-linalg/src/lib.rs and fixed_vec3.rs (FixedUnitVec3, normalize)
crates/bunny-num/src/fixed_angle.rs and fixed_q32_32.rs (sin_cos, sqrt, checked_div)
crates/bunny-query/src/trace.rs (TraceRay::new)
xtask/src/code_dojo.rs (limits), xtask/src/topic_docs.rs
.repo-respect/receipts/2026-11-15-optics-specular-reflection.md
The independent review of flyingrobots/bunny#376 and flyingrobots/bunny#423

Files edited:
crates/bunny-optics/src/shading/ggx.rs, smith.rs, visible_normal.rs (new)
crates/bunny-optics/src/shading/ggx/tests.rs, smith/tests.rs, visible_normal/tests.rs (new)
crates/bunny-optics/src/shading/specular.rs (public `reflect`; `mirror` renamed
`reflect_about` and shared; a crate-private `SpecularSample::new`)
crates/bunny-optics/src/shading/frame.rs (`in_unit_interval` visible to the module)
crates/bunny-optics/src/shading/wide.rs (a comment on `div_round`'s widths)
crates/bunny-optics/src/shading.rs, crates/bunny-optics/src/lib.rs
crates/bunny-optics/tests/ggx_conductor_tests.rs, tests/ggx_reference/mod.rs (new)
crates/bunny-optics/README.md
docs/topics/optics/README.md (Status, Specular Reflection, GGX Rough Conductor)
docs/topics/optics/test-plan.md (OPTICS-REQ-080 to -088; OPTICS-TP-140 to -161; the GGX
oracles; mutation checks; open gaps)
CHANGELOG.md (Unreleased)
.repo-respect/receipts/2026-11-15-optics-ggx-conductor.md

Topic docs:
docs/topics/optics/README.md (Status, Specular Reflection, GGX Rough Conductor) and
test-plan.md

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host. The repository's Git
hooks are not installed in this clone, so the full gate was run by hand.
cargo test -p bunny-optics --lib --test ggx_conductor_tests: the 11 unit cases and the 10
integration cases pass.
The albedo table comes from an integer Python quadrature and a closed form, the "port" values
from an integer Python port of the steps; neither script is committed (the pull request
holds them).
Two scratch searches (temporary ignored tests, not committed) found the plane-only absorption
case of OPTICS-TP-143 and measured the furnace difference on finer grids.
Mutations (test plan, GGX Mutation Checks): each under `timeout 400`; the restored sources
pass.
cargo fmt, clippy -D warnings, code-dojo --all at the final commit (see the pull request).

Known risks:
The furnace tolerance (`2^-11`) and the agreement with the ideal map are measured, not derived.
Within about `2^-20` of the rim of the sample disk the direction loses relative precision and
can differ from the ideal map by more than `2^-8`; such samples are still unit reflections with
the weight of the direction returned.
The roughness is the GGX width `alpha` itself, and the Smith term is the height-correlated
form; the issue chooses neither.
The lobe is about the geometric frame normal, not a smooth-shading normal.

Human reviewer:
James Ross (maintainer); review pending.
