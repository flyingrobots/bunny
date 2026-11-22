# Receipt: bunny-codec Readers Without crates.io Dependencies

Task:
Implement flyingrobots/bunny#291. Make the `bunny-codec` readers usable without
`sha2` and its crates.io tree. Only `bunny_mesh::compute_mesh_hash` uses `sha2`, so
`sha2` becomes optional behind a default-on `mesh-hash` feature of `bunny-mesh`, and
`bunny-codec` depends on `bunny-mesh` with `default-features = false` and forwards the
feature as its own default-on `mesh-hash`. The full gate checks the no-default
configuration of both crates.

Moving the plain record types to a new crate was rejected: it adds a crate and a
publish step for the same result. Re-implementing SHA-256 locally was rejected: it
changes the default dependency graph and replaces an audited hash for no gain here.
Gating the compressed profile alone would not help, because every reader uses
`bunny_mesh::Triangle32`. Without default features nothing in `bunny-codec` is lost;
only `bunny_mesh::compute_mesh_hash` is.

Files read:
Cargo.toml (workspace)
`crates/bunny-mesh/**` (Cargo.toml, README.md, src/lib.rs, tests)
`crates/bunny-codec/**` (Cargo.toml, README.md, src/lib.rs, every `use bunny_mesh`)
crates/bunny-geom/Cargo.toml, crates/bunny-linalg/Cargo.toml, crates/bunny-num/Cargo.toml
xtask/src/code_dojo.rs, xtask/src/nondeterminism.rs (dependency check),
xtask/src/topic_docs.rs (case evidence rules), xtask/src/repo_respect.rs (trailer check)
scripts/publish-crates.sh, .github/workflows/release.yml, .github/workflows/code-dojo.yml
docs/CODE_DOJO.md, docs/PROCESS.md, docs/topics/repo-respect/README.md
docs/topics/ascii-ply/README.md, docs/topics/ascii-ply/test-plan.md
docs/topics/code-dojo-determinism/README.md, CODE_STANDARDS.md, CHANGELOG.md

Files edited:
crates/bunny-mesh/Cargo.toml (`sha2` optional; `mesh-hash` feature, default on)
crates/bunny-mesh/src/lib.rs, crates/bunny-mesh/src/hash.rs (new; moved unchanged, then gated)
crates/bunny-mesh/tests/determinism.rs, crates/bunny-mesh/tests/mesh_tests.rs (hash tests gated)
crates/bunny-mesh/README.md
crates/bunny-codec/Cargo.toml (`bunny-mesh` without default features; `mesh-hash` forwarded, default on)
crates/bunny-codec/README.md
xtask/src/feature_configs.rs (new), xtask/src/code_dojo.rs, xtask/src/main.rs
.github/workflows/code-dojo.yml (wasm-pack runs of both crates without default features)
docs/CODE_DOJO.md (No-Default-Features Check)
docs/topics/ascii-ply/README.md (Dependencies And Features)
docs/topics/ascii-ply/test-plan.md (APLY-REQ-012, -013; APLY-TP-052 to -054; Feature Configurations)
CHANGELOG.md (Unreleased)
.repo-respect/receipts/2026-11-14-codec-without-mesh-hash.md

Topic docs:
docs/topics/ascii-ply/README.md (Dependencies And Features)
docs/topics/ascii-ply/test-plan.md (APLY-REQ-012, APLY-REQ-013; APLY-TP-052 to -054)

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
Red first: the full gate at 28aefed (the check, before the feature) fails with
"bunny-codec --no-default-features reaches `sha2 v0.10.9`" and nine more crates
(`cfg-if`, `cpufeatures`, `libc`, `digest`, `block-buffer`, `generic-array`,
`typenum`, `version_check`, `crypto-common`), and the same ten for bunny-mesh.
`cargo tree -e normal` for both crates with default features: identical to
`origin/main` 3c395f9. `cargo tree -p bunny-codec --no-default-features -e normal`
(and `-e normal,build --target all`): only the five Bunny path crates.
`cargo clippy`, `cargo test` and a wasm32 `cargo check` of `-p bunny-mesh
-p bunny-codec --no-default-features`: clean, every codec test passes (66 tests;
the four mesh hash tests are compiled out).
Rustdoc item lists (APLY-TP-054): identical to 3c395f9 with default features;
without them only `fn.compute_mesh_hash.html` is absent.
Seeded violation: dropping `default-features = false` from the bunny-codec
dependency on bunny-mesh makes the full gate fail with the ten crates above.
Mutations of `xtask/src/feature_configs.rs`, each under `timeout 120`: dropping the
Windows-path rule fails APLY-TP-053; dropping deduplication fails APLY-TP-052;
accepting every parenthesized source except `(*)` fails APLY-TP-052.
code-dojo --all at the final commit (see the PR); cargo package --list for both
crates; scripts/publish-crates.sh verify.

Known risks:
The wasm-pack runs without default features are new CI lines; the container has no
wasm-pack, so they were first run by CI, not locally (the wasm32 `cargo check` in
the gate did run). Publishing: `bunny-codec` now names the `bunny-mesh/mesh-hash`
feature, so a future release must publish `bunny-mesh` before `bunny-codec`, which
is already the order in scripts/publish-crates.sh. Commits 641e877 and 9f16290
have no blank line between subject and body, and 641e877's scope
`docs(ascii-ply)` is not a valid Code Dojo scope; commits cannot be amended, so
the record is corrected here.

Human reviewer:
James Ross (maintainer); review pending.
