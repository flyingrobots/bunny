# Receipt: Pin The Three Rounding Rules Of bunny-diffuse-shading/1

Task:
flyingrobots/bunny#298 (from the independent review in flyingrobots/bunny#290, batch 2). Pin
the light cosine's rounding to nearest, `round_shift`'s ties to even and the frame division's
ties to even with exact goldens, make OPTICS-TP-042 assert the exact tie result, and show by
mutation that each wrong rule fails a test that does not use the retained 0.7.1 copy. Test and
documentation only: no library function changes.

Files read:
crates/bunny-optics/src/shading/ (wide.rs, frame.rs, light.rs, surface.rs, hit.rs)
crates/bunny-linalg/src/lib.rs, fixed_vec3.rs (FixedUnitVec3::new, normalize, length)
crates/bunny-num/src/fixed_q32_32.rs (division rounding)
crates/bunny-num/tests/tie_rounding_tests.rs and docs/topics/fixed-point-rounding/ (the
pattern of flyingrobots/bunny#287)
crates/bunny-optics/tests/shading_tests.rs, shading_property_tests.rs, smooth_normal_tests.rs
docs/topics/optics/README.md, docs/topics/optics/test-plan.md

Files edited:
crates/bunny-optics/tests/shading_rounding_tests.rs (new: OPTICS-TP-092 to -094)
crates/bunny-optics/tests/shading_property_tests.rs (OPTICS-TP-042: exact ties-to-even oracle
and five tie normals)
docs/topics/optics/README.md (the frame's exact ties and which unit normals reach them)
docs/topics/optics/test-plan.md (OPTICS-TP-042 text; OPTICS-TP-092 to -095; shading rounding
mutation checks; the rounding sites of the shading path)
CHANGELOG.md (Unreleased)
.repo-respect/receipts/2026-11-14-optics-shading-rounding-pins.md

Topic docs:
docs/topics/optics/README.md and test-plan.md

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
Survival reproduced first, on the unchanged tests, each mutant under `timeout 120` with every
source touched before the build: the cosine floored, `round_shift` with ties away from zero
and the frame division with ties away from zero each passed every bunny-optics test.
Every expected value re-derived with exact rationals in a separate Python model of
`wide::unit`, `FixedUnitVec3::new`, the frame and the cosine; the frame-division tie on a real
triangle was settled by search (four triangles and a sliver reach ties; the review's own
`(2^31, 3 719 550 789, 0)` is not reachable from a triangle).
After: 14 mutants (six wrong rules for each of `round_shift` and `div_round`, and each
`round_shift` call site floored) each fail OPTICS-TP-042, -092, -093 or -094 (table in the
test plan); the restored source passes every bunny-optics test.
cargo fmt --check, cargo clippy -p bunny-optics --all-targets -D warnings, cargo test -p
bunny-optics; code-dojo --all at the final commit (see the PR).

Known risks:
The Python model and the hand derivations share one author's reading of the chapter. The
combination site has exact ties but no input one raw unit either side of one. The first
product of the light term (`cosine * FRAC_1_PI`) has no tie case in the optics tests; its
rule is the shared Q32.32 multiplication, pinned in bunny-num.

Human reviewer:
James Ross (maintainer); review pending.
