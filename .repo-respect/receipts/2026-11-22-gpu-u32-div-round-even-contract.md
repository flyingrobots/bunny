# Receipt: Gpu U32 Div Round Even Contract

Task:
Amend `docs/design/0005-gpu-compute-backend.md` (tracked in `#427`) with Amendment 1 defining the narrow shared unsigned division slice (`bunny-u32-div-round-even/1`), explicit Q32.32 non-equivalence boundary, four-layer verification taxonomy, content-addressed implementation manifest, kernel conformance receipt schema, and WebGL2 `RGBA32UI` integer readback with transport canary.

Files read:
- `docs/design/0005-gpu-compute-backend.md`
- `docs/PROCESS.md`
- `docs/README.md`
- `docs/topics/repo-respect/README.md`
- `CODE_STANDARDS.md`
- `Cargo.toml`

Files edited:
- `docs/design/0005-gpu-compute-backend.md`
- `.repo-respect/receipts/2026-11-22-gpu-u32-div-round-even-contract.md`

Topic docs:
None - this is a design-proposal amendment under `docs/design/`; nothing in `crates/bunny-gpu` exists in `HEAD` yet (implementation is tracked separately in `#428`).

Generated artifacts:
None.

Checks run:
- `git diff --check` (passed)
- `cargo run --locked -p xtask -- topic-docs` (passed: 236 active requirements, 0 errors)
- `cargo run --locked -p xtask -- code-dojo-rust` (passed)
- `cargo run --locked -p xtask -- code-dojo-generated` (passed)
- `cargo run --locked -p xtask -- code-dojo-commit-msg` (passed)
- `cargo run --locked -p xtask -- repo-respect check --staged` and `--branch` (passed)
- `cargo fmt --all -- --check` (passed)
- Fresh-context Baker Street mathematical & contract verification (15/15 core golden vectors verified against Python `fractions.Fraction`, 100,444 bounds-checked `u32` pairs with 0 overflows/underflows/mismatches).
- Note: local `cargo clippy` ran under Homebrew `rustc 1.100.0` (rather than `rust-toolchain.toml` `1.96.0`) and reported an unrelated `clippy::chunks_exact_to_as_chunks` lint on untouched `crates/bunny-perf/src/memory.rs`; CI runs on pinned `1.96.0`.

Known risks:
- Design contract only; `crates/bunny-gpu`, WGSL shader execution, and downstream WebGL2 `RGBA32UI` execution remain unimplemented until `#428` and `flyingrobots/geordi#170` land.
- Downstream rounded-arc geometry (`Q12` vs exact integer ellipse predicate) remains a separate open decision tracked outside this amendment.

Human reviewer:
@flyingrobots
