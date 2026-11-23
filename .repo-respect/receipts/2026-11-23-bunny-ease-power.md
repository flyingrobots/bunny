# Receipt: Bunny Ease Power

Task:
flyingrobots/bunny#304: `bunny-ease/1`, part 1, in `bunny-num`. `ease::power_in`, `power_out`
and `power_in_out` for `n` in `2..=5` evaluate the issue's polynomials as exact rationals and
round once to nearest, ties to even; `ease::steps(n, t)` is `round(floor(n t) / n)`. `t` outside
`[0, 1]` and `n` outside each function's range are `None`. Additive: no existing public item,
output bit, golden, digest or ID changes.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*
.repo-respect/receipts/2026-11-23-ray-footprint.md
crates/bunny-num/src/{lib.rs,lerp.rs}, crates/bunny-num/tests/lerp_tests.rs,
crates/bunny-num/Cargo.toml, crates/bunny-num/README.md
crates/bunny-linalg/src/fixed_mat4_look_at.rs (the wide product the issue cites)
docs/topics/lerp/README.md and test-plan.md, xtask/src/topic_docs.rs
flyingrobots/bunny#304, #305 and #306 (the issues that list #304 in `.blocked_by`)

Files edited:
crates/bunny-num/src/ease.rs (new), crates/bunny-num/src/lib.rs (module and export)
crates/bunny-num/tests/ease_tests.rs (new)
docs/topics/ease/test-plan.md (new: EASE-REQ-001 to -004, EASE-TP-001 to -008)
.repo-respect/receipts/2026-11-23-bunny-ease-power.md

Topic docs:
docs/topics/ease/ is a new topic with its own prefix, starting at 001.

Generated artifacts:
None. The two pinned digests and the golden raw values were printed by a Python
`fractions.Fraction` model that is posted on the pull request, not kept in the tree.

Checks run:
First commit (planned cases, failing tests, a stub kernel that returns `None`): cargo test
--locked -q -p bunny-num --test ease_tests runs 7 tests and all 7 fail on the kernel's
assertions (the oracle's own golden assertions pass); topic-docs clean.

Known risks:
The grid `k / 2^16` lands many powers exactly on the Q32.32 grid, so the seeded and boundary
full-resolution points carry most of the rounding evidence.

Human reviewer:
James Ross (maintainer); review pending.
