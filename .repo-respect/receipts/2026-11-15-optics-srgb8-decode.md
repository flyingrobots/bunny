# Receipt: sRGB 8-bit Decode

Task:
flyingrobots/bunny#366: `bunny-srgb8-decode/1`, `srgb8_to_linear`, a fixed 256-entry table
from an 8-bit sRGB code to linear light in raw Q32.32, each entry the exact sRGB decode rounded
to nearest. Adds a public function and a constant; no existing output changes.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, docs/NUMERIC_CONSTITUTION.md
docs/topics/optics/README.md and test-plan.md
crates/bunny-optics/src/pixel.rs, pixel/srgb.rs, pixel/srgb_knots.rs, pixel/tone.rs
crates/bunny-optics/tests/pixel_tests.rs

Files edited:
crates/bunny-optics/src/pixel/decode.rs, pixel/srgb8_table.rs (new)
crates/bunny-optics/src/pixel.rs, crates/bunny-optics/src/lib.rs
crates/bunny-optics/tests/srgb8_decode_tests.rs (new)
docs/topics/optics/README.md and test-plan.md
CHANGELOG.md (Unreleased)
.repo-respect/receipts/2026-11-15-optics-srgb8-decode.md

Topic docs:
docs/topics/optics/README.md (Status, Pixel Values) and test-plan.md

Generated artifacts:
The table in srgb8_table.rs, generated once with 80-digit decimal arithmetic (rule in its
header and in the optics chapter).

Checks run:
In progress; see the pull request.

Known risks:
None known yet.

Human reviewer:
James Ross (maintainer); review pending.
