# Receipt: Bump `wesley-core` to `0.3.0-alpha.4`

Task:
Update `crates/bunny-wesley/Cargo.toml` from `wesley-core = "0.0.5"` to
`wesley-core = { version = "=0.3.0-alpha.4", default-features = false }` so Bunny drops the
unused transitive `chrono` dependency flagged by `RUSTSEC-2026-0543` (`cargo deny check`)
and unused async resilience dependencies not needed by the synchronous `bunny-wesley` CLI.

Files read:
Cargo.toml
Cargo.lock
deny.toml
crates/bunny-wesley/Cargo.toml
crates/bunny-wesley/build.rs
crates/bunny-wesley/src/main.rs
crates/bunny-wesley/src/profile.rs
crates/bunny-wesley/src/render.rs

Files edited:
Cargo.lock
deny.toml
crates/bunny-wesley/Cargo.toml
crates/bunny-wesley/build.rs
crates/bunny-wesley/src/render.rs
crates/bunny-contract/src/generated/graphics.rs
generated/bunny-graphics.manifest.json
generated/typescript/bunny-graphics.ts
.repo-respect/receipts/2026-11-22-bump-wesley-core.md

Topic docs:
None changed; `docs/topics/contract-generation/` behavior is unchanged aside from bumping the
pinned `wesley-core` crate version witness from `0.0.5` to `0.3.0-alpha.4`.

Generated artifacts:
Regenerated `crates/bunny-contract/src/generated/graphics.rs`,
`generated/typescript/bunny-graphics.ts`, and `generated/bunny-graphics.manifest.json` via
`cargo run --locked -p xtask -- generate` (only the `wesleyCore` / `BUNNY_WESLEY_CORE_VERSION`
witness changed from `0.0.5` to `0.3.0-alpha.4`).

Checks run:
- `rg 'name = "chrono"' Cargo.lock` -> 0 matches
- `cargo test --locked --workspace --all-targets --all-features` -> pass
- `cargo run --locked -p xtask -- code-dojo-generated` -> clean
- `cargo run --locked -p xtask -- code-dojo-rust --all` -> clean
- `cargo run --locked -p xtask -- code-dojo-determinism --enforce` -> clean
- `cargo run --locked -p xtask -- repo-respect check --branch` -> clean
- `cargo test --locked -p bunny-wesley` with the exact pin and no operator stripping -> the
  witness test fails (`"=0.3.0-alpha.4"` against `"0.3.0-alpha.4"`); with stripping -> pass

Review follow-up:
The requirement is pinned exactly (`=0.3.0-alpha.4`) because a bare `0.3.0-alpha.4` admits any
later `0.3.x`, which would leave the generated `BUNNY_WESLEY_CORE_VERSION` witness stale after a
lockfile update. `build.rs` refuses a requirement that is not an exact pin and emits it without
the operator. `wesley_core_version_witness_matches_the_resolved_lockfile_version` in
`crates/bunny-wesley/src/render.rs` compares the witness with the version in `Cargo.lock`.

Known risks:
`wesley-core 0.3.0-alpha.4` is a pre-release version already published on crates.io and
consumes `foldhash v0.2.0` under the permissive `Zlib` license (added to `deny.toml`
`[licenses].allow`).

Human reviewer:
@flyingrobots
