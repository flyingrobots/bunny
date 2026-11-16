# Receipt: PLY Texture Coordinates

Task:
Add the ASCII PLY profile `/2`, whose reader returns per-vertex texture coordinates from `s`/`t` or
`u`/`v` vertex properties, converted by `bunny-decimal-q32-32/1`, beside the
positions, with a file without them read as before (flyingrobots/bunny#369).

Files read:
CONTRIBUTING.md
CODE_STANDARDS.md
docs/NUMERIC_CONSTITUTION.md
docs/topics/ascii-ply/README.md
docs/topics/ascii-ply/test-plan.md
crates/bunny-codec/src/ply_ascii.rs and crates/bunny-codec/src/ply_ascii/*.rs
crates/bunny-codec/tests/ply_ascii_*.rs

Files edited:
crates/bunny-codec/src/lib.rs
crates/bunny-codec/src/ply_ascii.rs
crates/bunny-codec/src/ply_ascii/header.rs
crates/bunny-codec/src/ply_ascii/body.rs
crates/bunny-codec/src/ply_ascii/limits.rs
crates/bunny-codec/tests/ply_ascii_uv_tests.rs
docs/topics/ascii-ply/README.md
docs/topics/ascii-ply/test-plan.md
docs/README.md
CHANGELOG.md
.repo-respect/receipts/2026-11-15-ply-texture-coordinates.md

Topic docs:
docs/topics/ascii-ply/README.md and test-plan.md.

Generated artifacts:
None.

Checks run:
See the pull request description for the exact commands and results at the final commit.

Known risks:
Texture coordinates are a new profile, `bunny-ascii-ply-triangle-mesh/2` (coordinator decision
on the pull request), because it refuses inputs `/1` accepts; `/1` is unchanged.

Human reviewer:
Pending.
