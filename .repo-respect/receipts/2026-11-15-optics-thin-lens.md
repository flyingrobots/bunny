# Receipt: Thin-Lens Camera

Task:
flyingrobots/bunny#364: `bunny-thin-lens/1`, a `ThinLensCamera` beside `PinholeCamera` that
takes an aperture radius and a focus distance and samples the lens with one sampler pair.
Aperture 0 gives the pinhole's rays bit for bit, by the same arithmetic path.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, docs/NUMERIC_CONSTITUTION.md
docs/topics/optics/README.md and test-plan.md
crates/bunny-optics/src/{camera,plane,basis,sampler,lib,error}.rs, tests/camera_tests.rs,
tests/exact/mod.rs
crates/bunny-query/src/trace.rs, crates/bunny-num/src/{fixed_q32_32,fixed_angle}.rs

Files edited:
crates/bunny-optics/src/thin_lens.rs (new)
crates/bunny-optics/src/camera.rs (shared sample_direction, parts, narrow)
crates/bunny-optics/src/plane.rs (Margin, fit_with; fit unchanged in output)
crates/bunny-optics/src/lib.rs (module, exports, crate docs)
crates/bunny-optics/tests/thin_lens_tests.rs (new; the radius-scaled octant exclusion)
crates/bunny-optics/tests/exact/mod.rs (Signed::sign)
crates/bunny-optics/README.md
docs/topics/optics/README.md (Status, Thin-Lens Camera)
docs/topics/optics/test-plan.md (OPTICS-REQ-070 to -075; OPTICS-TP-130 to -137; the
focus-plane oracle; mutation checks; two open gaps)
CHANGELOG.md (Unreleased)
.repo-respect/receipts/2026-11-15-optics-thin-lens.md

Topic docs:
docs/topics/optics/README.md and test-plan.md

Generated artifacts:
None.

Checks run:
All inside the bunny container through devrun; nothing on the host.
cargo test -p bunny-optics: every case passes, thin_lens_tests 7 of 7.
Mutations (test plan, Thin-Lens Mutation Checks): eleven; ten fail at least one case; the
defensive forward margin is not reached (an open gap).
cargo fmt, clippy -D warnings; code-dojo --all at the final commit (see the PR).
2026-11-22, after merging origin/main (GGX conductor, cosine rounding, dielectric, texture,
ray-disc and ray-cylinder): cargo fmt --all --check; cargo test --locked --workspace
--all-targets --all-features (107 test binaries, 0 failed; thin_lens_tests 7 of 7);
code-dojo-generated; code-dojo-rust --all; code-dojo-determinism --enforce;
code-dojo-perf-groups; topic-docs (0 duplicate IDs); repo-respect check --branch. The merge
changed no line of thin_lens.rs, camera.rs or plane.rs, so the mutations were not rerun.
CodeRabbit review on a193459: the octant check of OPTICS-TP-132 excluded a fixed 2^-24 turn,
which rejects valid small-radius points (u = 1 raw, v = 2^29 - 256 gives (46340, 46340)). The
new constructed points failed the old exclusion first; the exclusion is now 2^31 / r raw turns.
After the change the four mutants that cite OPTICS-TP-132 (disc factor removed, angle in half
turns, radius u, cosine and sine swapped) still fail it.

Known risks:
The forward margin is defensive and no test reaches it. A large aperture for its focus
distance raises the scale shift, shortening directions. The disc is shrunk by sqrt(1 - 2^-28)
so that its rounded points stay inside the unit disc. IDs OPTICS-REQ-070.. and
OPTICS-TP-130.. leave room for parallel optics work (#366, #367); after the merge none
collides with main.
The GGX sampler on main has its own disk_point (radius sqrt(u), floored); the lens keeps
lens_disc_point because its radius sqrt(u (1 - 2^-28)) is what keeps every rounded point
strictly inside the disc, and switching would change pinned output bits.

Human reviewer:
James Ross (maintainer); review pending.
