# Receipt: Deterministic Contract Profile

Task:
Implement the first Bunny-owned deterministic contract profile slice for
generated wire-codec metadata. The change extends `bunny-wesley` scalar
profiles with wire-profile names and fixed byte widths, emits those witnesses
into generated Rust, TypeScript, and manifest artifacts, and documents the
topic under `docs/topics/`.

Files read:
- `README.md`
- `CHANGELOG.md`
- `CONTRIBUTING.md`
- `CODE_STANDARDS.md`
- `crates/bunny-wesley/src/main.rs`
- `crates/bunny-wesley/src/render.rs`
- `crates/bunny-wesley/README.md`
- `crates/bunny-contract/README.md`
- `crates/bunny-contract/tests/generated_version_tests.rs`
- `crates/bunny-contract/src/generated/graphics.rs`
- `generated/typescript/bunny-graphics.ts`
- `generated/bunny-graphics.manifest.json`
- `docs/README.md`
- `docs/topics/coordinate-law/README.md`
- `docs/topics/coordinate-law/test-plan.md`
- `.repo-respect/README.md`
- `xtask/src/topic_docs.rs`
- `xtask/src/main.rs`

Files edited:
- `README.md`
- `CHANGELOG.md`
- `crates/bunny-wesley/src/main.rs`
- `crates/bunny-wesley/src/render.rs`
- `crates/bunny-wesley/src/profile.rs`
- `crates/bunny-wesley/README.md`
- `crates/bunny-contract/README.md`
- `crates/bunny-contract/tests/generated_version_tests.rs`
- `docs/README.md`
- `docs/topics/deterministic-contract-profile/README.md`
- `docs/topics/deterministic-contract-profile/test-plan.md`

Topic docs:
- Added `docs/topics/deterministic-contract-profile/README.md`.
- Added `docs/topics/deterministic-contract-profile/test-plan.md`.
- Updated `docs/README.md` with the new topic shelf.

Generated artifacts:
- `crates/bunny-contract/src/generated/graphics.rs`
- `generated/typescript/bunny-graphics.ts`
- `generated/bunny-graphics.manifest.json`

Checks run:
- `cargo test --locked -p bunny-wesley`
- `cargo run --locked -p xtask -- generate`
- `cargo fmt --all`
- `cargo test --locked -p bunny-wesley -p bunny-contract`
- `cargo run --locked -p xtask -- topic-docs`
- `cargo run --locked -p xtask -- code-dojo-rust --all`

Known risks:
The current slice emits deterministic profile witnesses but not byte reader or
writer functions. The topic docs record codec emitters, explicit maximum bounds,
canonical map profiles, and cross-language byte vectors as open gaps.

Human reviewer:
James Ross requested and approved the Bunny deterministic contract profile
slice in chat on 2026-06-26.
