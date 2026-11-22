# Receipt: try_look_at Exact Parallel Up, Representable Translation, Test Range

Task:
Fix flyingrobots/bunny#273, three review findings on `FixedMat4::try_look_at`: refuse an exactly
parallel `up` (decided on the exact integer inputs), stop refusing a representable translation
entry, and make the seeded orthonormality test reach its stated 2^38 range. Declared behaviour
correction for two input classes; all other `Ok` results keep every bit.

Files read:
CODE_STANDARDS.md
docs/CODE_DOJO.md
clippy.toml
docs/topics/performance/README.md (Fidelity Rule)
docs/topics/coordinate-law/README.md
docs/topics/matrix-types/README.md
docs/topics/matrix-types/test-plan.md
crates/bunny-linalg/src/fixed_mat4_look_at.rs
crates/bunny-linalg/tests/look_at_tests.rs
crates/bunny-num/src/fixed_q32_32.rs (checked_mul rounding)

Files edited:
crates/bunny-linalg/src/fixed_mat4_look_at.rs
crates/bunny-linalg/tests/look_at_tests.rs
crates/bunny-linalg/tests/look_at_fidelity_tests.rs (new)
crates/bunny-linalg/tests/look_at_reference/mod.rs (new, 0.7.0 code verbatim)
docs/topics/matrix-types/README.md
docs/topics/matrix-types/test-plan.md (MT-REQ-014 text, MT-TP-030..032)
CHANGELOG.md (Unreleased, Changed (behaviour))
.repo-respect/receipts/2026-11-14-look-at-parallel-up.md

Topic docs:
docs/topics/matrix-types/README.md (Look-At View Matrix)

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
Red first: MT-TP-030, MT-TP-031 and the differential test fail on the unmodified implementation;
MT-TP-029 passes with 62-bit draws, so the documented bounds hold over the real range.
cargo fmt, clippy -D warnings and cargo test --locked --workspace in one command.
Mutations (see the PR): dropping the antiparallel case, and checking each term instead of the
finished entry, each fail a test.
code-dojo --all at the final commit (see the PR).

Known risks:
The change alters two input classes of a function released in 0.7.0 (not yet on crates.io);
declared in the changelog. The translation change also accepts entries whose individual terms
overflow but whose finished sum fits.

Human reviewer:
James Ross (maintainer); review pending.
