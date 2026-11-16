# Receipt: PLY Texture Coordinates

Task:
Make the ASCII PLY reader return per-vertex texture coordinates from `s`/`t` or
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
Files that declare `s`, `t`, `u` or `v` were accepted and their columns skipped; they are now
read, so such a file with a lone name, a non-float type or a non-decimal value is refused.
Whether that needs a new profile version is an open question on the pull request.

Human reviewer:
Pending.
