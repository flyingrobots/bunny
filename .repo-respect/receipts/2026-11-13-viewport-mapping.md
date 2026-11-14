# Receipt: Viewport Mapping From NDC

Task:
Implement flyingrobots/bunny#202: `Viewport`, `ViewportError`, and the checked Q32.32
mapping from NDC to viewport coordinates and back, with the contract decisions written
into the coordinate-law chapter. Pixel snapping (Geordi) is out of scope; no existing
function changes.

Files read:
CODE_STANDARDS.md
CONTRIBUTING.md
CHANGELOG.md
clippy.toml
docs/CODE_DOJO.md
docs/COORDINATE_LAW.md
docs/topics/coordinate-law/README.md
docs/topics/coordinate-law/test-plan.md
docs/topics/matrix-types/README.md
docs/topics/repo-respect/README.md
crates/bunny-linalg/src/fixed_mat4_projection.rs
crates/bunny-linalg/tests/projection_tests.rs

Files edited:
crates/bunny-linalg/src/viewport.rs
crates/bunny-linalg/src/lib.rs
crates/bunny-linalg/tests/viewport_tests.rs
crates/bunny-linalg/README.md
docs/topics/coordinate-law/README.md
docs/topics/coordinate-law/test-plan.md
docs/topics/matrix-types/README.md
CHANGELOG.md
.repo-respect/receipts/2026-11-13-viewport-mapping.md

Topic docs:
docs/topics/coordinate-law/README.md (Viewport conversion section: origin, y direction,
pixel corners, floor rounding, depth, errors, round-trip bound)
docs/topics/coordinate-law/test-plan.md (CL-REQ-014..018, CL-TP-006..012)

Generated artifacts:
None.

Checks run:
All inside flyingrobots/rust-dev:1.96.0 through devrun; nothing on the host.
cargo test --locked -p bunny-linalg --test viewport_tests at the test-only commit (does not compile: Viewport missing)
cargo fmt --all -- --check
cargo clippy --locked -p bunny-linalg --all-targets -- -D warnings
cargo test --locked -p bunny-linalg
Deliberate mutation: y mapped as x_ndc + 1 instead of 1 - y_ndc (y direction flipped) made
CL-TP-006..010 and CL-TP-012 fail.
cargo test --locked --workspace
cargo run --locked -p xtask -- code-dojo --all

Known risks:
The contract choices (pixel corners, floor rounding, unsigned sizes) are the maintainer's to
overrule; they are listed in the coordinate-law chapter.

Human reviewer:
James Ross (maintainer); review pending.
