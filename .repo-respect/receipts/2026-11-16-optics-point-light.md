# Receipt: Point Light Sample

Task:
flyingrobots/bunny#365: `bunny-point-light/1`, a point light (position, intensity) whose
`LightSample` has an irradiance of intensity times cosine over distance squared and a shadow
ray that ends at the light, beside `DirectionalLight`. A hit at or nearer than the minimum
distance is refused with a typed error.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, docs/NUMERIC_CONSTITUTION.md
docs/topics/optics/README.md and test-plan.md
`crates/bunny-optics/src/shading/**` (hit, light, refusal, specular, wide)
crates/bunny-optics/tests/exact/mod.rs, specular_reflection_tests.rs
crates/bunny-query/src/trace.rs

Files edited:
crates/bunny-optics/src/shading/point_light.rs (new)
crates/bunny-optics/src/shading/light.rs (LightSample::shadow_t_max, bounded constructor)
crates/bunny-optics/src/shading/wide.rs (div_round's comment on its operand range)
crates/bunny-optics/src/shading.rs, crates/bunny-optics/src/lib.rs
crates/bunny-optics/tests/point_light_tests.rs (new)
crates/bunny-optics/tests/exact/mod.rs (a big-operand form of the square-root comparison)
crates/bunny-optics/README.md
docs/topics/optics/README.md (Status, Point Light)
docs/topics/optics/test-plan.md (scope; OPTICS-REQ-100 to -102; OPTICS-TP-200 to -206 with their
Test Cases rows; point-light mutation checks)
CHANGELOG.md (Unreleased)
.repo-respect/receipts/2026-11-16-optics-point-light.md

2026-11-22, the merge of main (GGX conductor, incidence-cosine rule, smooth dielectric,
texture bilinear, sRGB decode, ray disc, cylinder, box and sphere): conflicts in CHANGELOG.md,
crates/bunny-optics/README.md, src/lib.rs, src/shading.rs and the optics topic docs resolved
by keeping both sides. The point light's IDs moved from OPTICS-REQ-070 to -072 (also used by
the open thin-lens branch) and OPTICS-TP-120 to -126 (TP-120 to -123 landed on main for the
sRGB decode) to OPTICS-REQ-100 to -102 and OPTICS-TP-200 to -206, in the requirement table,
metadata, cases, mutation table and test names; the missing Test Cases rows were added and a
bare `|` in a table code span escaped.

Topic docs:
docs/topics/optics/README.md and test-plan.md

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
cargo test -p bunny-optics --test point_light_tests: 6 cases pass; the seeds are printed.
Golden values marked "model" come from Python integers written from the chapter's formulas.
Mutations (test plan, Point-Light Mutation Checks): twelve, each under `timeout 120`; each
fails at least one case; the restored source passes.
cargo fmt, clippy -D warnings; the full local gate at the final commit (see the PR).
2026-11-22, after the merge of main, on the macOS host (Rust 1.100.0): cargo fmt --all --check;
cargo test --locked --workspace --all-targets --all-features (107 test result lines, all ok;
point_light_tests 6 passed); xtask code-dojo-generated, code-dojo-rust --all,
code-dojo-determinism --enforce, code-dojo-perf-groups, topic-docs (zero duplicate IDs) and
repo-respect check --branch, each clean. No kernel line changed in the merge, so the twelve
mutations were not rerun.

Known risks:
The minimum distance (1/128 unit) and `shadow_t_max = m - 1` are choices this identity fixes;
the issue named neither value. The cosine bound of three raw units assumes a unit normal, as
the frame normal is. The shadow ray passes within `m / 2^33` raw units of the light, not
through it; the stop along the largest axis is exact.

Human reviewer:
James Ross (maintainer); review pending.
