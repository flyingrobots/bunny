# Receipt: Bump `wesley-core` to `0.3.0-alpha.4`

Task:
Update `crates/bunny-wesley/Cargo.toml` from `wesley-core = "0.0.5"` to
`wesley-core = { version = "0.3.0-alpha.4", default-features = false }` so Bunny drops the
unused transitive `chrono` dependency flagged by `RUSTSEC-2026-0543` (`cargo deny check`)
and unused async resilience dependencies not needed by the synchronous `bunny-wesley` CLI.

Files read:
Cargo.toml
Cargo.lock
crates/bunny-wesley/Cargo.toml
crates/bunny-wesley/src/main.rs
crates/bunny-wesley/src/profile.rs
crates/bunny-wesley/src/render.rs

Files edited:
Cargo.lock
deny.toml
crates/bunny-wesley/Cargo.toml
crates/bunny-contract/src/generated/graphics.rs
generated/bunny-graphics.manifest.json
generated/typescript/bunny-graphics.ts
.repo-respect/receipts/2026-11-22-bump-wesley-core.md

Verification:
- `rg 'name = "chrono"' Cargo.lock` returns zero matches
- `cargo test --locked -p bunny-wesley` passes
- `cargo run --locked -p xtask -- code-dojo-generated` passes
- `cargo run --locked -p xtask -- code-dojo-rust --all` passes
- `cargo run --locked -p xtask -- repo-respect check --branch` passes
