# Receipt: The Start-Point Rule's Two-Step Case And Tie Axis

Task:
flyingrobots/bunny#288 (with the witnesses of the independent review in flyingrobots/bunny#290,
batch 2). Pin the start-point rule's two-step case and its "lowest axis on a tie" choice with
tests that do not depend on the retained 0.7.1 copy, and correct the chapter's claims that two
steps were never needed. Test and documentation only: no library function changes.

Files read:
crates/bunny-optics/src/shading/ (hit.rs, wide.rs, surface.rs, frame.rs, light.rs)
crates/bunny-optics/tests/shading_tests.rs, shading_property_tests.rs, shading_refusal_tests.rs,
shading_reference/mod.rs
crates/bunny-query/src/trace.rs (hit_triangle's floored distance)
docs/topics/optics/README.md, docs/topics/optics/test-plan.md
CONTRIBUTING.md, docs/PROCESS.md, docs/CODE_DOJO.md

Files edited:
crates/bunny-optics/tests/start_point_tests.rs (new: OPTICS-TP-087 to -090)
docs/topics/optics/README.md (start point: two steps occur, with measured counts; Accuracy table)
docs/topics/optics/test-plan.md (OPTICS-REQ-033 names the tie rule; OPTICS-TP-087 to -091;
start-point mutation checks; the two-step open gap removed)
CHANGELOG.md (Unreleased)
.repo-respect/receipts/2026-11-14-optics-start-point-two-steps.md

Topic docs:
docs/topics/optics/README.md and test-plan.md

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
Survival reproduced first, on the unchanged tests, each mutant under `timeout 120` with every
source touched before the build: `MAX_STEPS = 1` failed only OPTICS-TP-062 and -064 (the
comparison with the retained 0.7.1 code); the highest axis on a tie failed nothing.
Both witnesses checked with exact integers in a separate Python model (exact distance floored,
point inside the triangle, point_at rounding, plane values, steps under both tie rules), then
asserted on the hits hit_triangle reports.
After: the four start-point mutants (one step, highest tie axis, wrong step sign, inclusive
plane test) each fail OPTICS-TP-087 to -090 or a subset (table in the test plan); the restored
source passes every bunny-optics test.
cargo fmt --check, cargo clippy -p bunny-optics --all-targets -D warnings, cargo test -p
bunny-optics; code-dojo --all at the final commit (see the PR).

Known risks:
The rule check of OPTICS-TP-089 restates the chapter's rule in the test; a misreading shared by
the chapter, the code and the test would pass. The golden cases are hand derived and checked by
a Python model written by the same author. The step counts depend on the generator's
population (ray directions from 0.5 to 1.0 in their largest component); other scenes give other
rates.

Human reviewer:
James Ross (maintainer); review pending.
