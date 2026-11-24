# Receipt: Runnable Examples

Task:
Add runnable examples for downstream consumers (flyingrobots/bunny#131): decode a compressed mesh and read its vertices and triangles; parse OBJ and PLY and compute the mesh hash; build a BVH and ray-query it; validate float boundary geometry into fixed primitives; and have the gate compile them. Additive only: no library code, public API, existing test, fixture, golden, digest or ID changes.

Files read:
- `CONTRIBUTING.md`
- `CODE_STANDARDS.md`
- `docs/CODE_DOJO.md`
- `docs/README.md`
- `docs/TESTING.md`
- `docs/topics/code-dojo-determinism/README.md`
- `docs/topics/release-packaging/README.md`
- `xtask/src/code_dojo.rs`
- `xtask/src/feature_configs.rs`
- `xtask/src/module_tree.rs`
- `xtask/src/waivers.rs`
- `.github/workflows/code-dojo.yml`
- `crates/bunny-codec/Cargo.toml`, `README.md`, `src/lib.rs`, `src/compressed.rs`, `src/compressed/read.rs`, `src/compressed/view.rs`, `src/obj.rs`, `src/obj/iter.rs`, `src/ply.rs`, `src/ply_ascii.rs`
- `crates/bunny-codec/tests/compressed_tests.rs`, `tests/obj_tests.rs`, `tests/ply_ascii_tests.rs`, `tests/stanford_fixture_tests.rs`, `tests/fixtures/canonical_compressed_triangle.bunny.hex`
- `crates/bunny-mesh/Cargo.toml`, `README.md`, `src/lib.rs`, `src/hash.rs`, `src/bounds.rs`
- `crates/bunny-broadphase/Cargo.toml`, `README.md`, `src/lib.rs`, `src/bvh.rs`, `src/closest_hit.rs`, `src/traversal.rs`
- `crates/bunny-query/Cargo.toml`, `README.md`, `src/trace.rs`
- `crates/bunny-geom/Cargo.toml`, `README.md`, `src/lib.rs`, `src/conversions.rs`
- `crates/bunny-linalg/src/lib.rs`, `src/fixed_vec3.rs`
- `crates/bunny-num/src/fixed_q32_32.rs`, `src/fixed_q32_32/conversions.rs`

Files edited:
- `CHANGELOG.md`
- `crates/bunny-broadphase/README.md`
- `crates/bunny-broadphase/examples/bvh_ray_query.rs`
- `crates/bunny-codec/Cargo.toml`
- `crates/bunny-codec/README.md`
- `crates/bunny-codec/examples/decode_compressed_mesh.rs`
- `crates/bunny-codec/examples/mesh_hash.rs`
- `crates/bunny-geom/README.md`
- `crates/bunny-geom/examples/float_boundary.rs`
- `docs/CODE_DOJO.md`
- `docs/TESTING.md`
- `.repo-respect/receipts/2026-11-24-runnable-examples.md`

Topic docs:
None - the examples add no behaviour and no ledger entry. `docs/CODE_DOJO.md` gains an Examples section on how the existing `--all-targets` commands build and check them; `docs/TESTING.md` gains a layout row.

Generated artifacts:
None.

Checks run:
- `cargo fmt --all --check`
- `cargo test --locked --workspace --all-targets --all-features` (152 `test result: ok` lines, none failed; the four example test modules ran)
- `cargo run --locked -p xtask -- code-dojo-generated`
- `cargo run --locked -p xtask -- code-dojo-rust --all`
- `cargo run --locked -p xtask -- code-dojo-determinism --enforce`
- `cargo run --locked -p xtask -- code-dojo-perf-groups`
- `cargo run --locked -p xtask -- topic-docs` (0 duplicate IDs)
- `cargo clippy --locked -p bunny-codec -p bunny-geom -p bunny-broadphase --all-targets --all-features -- -D warnings` (no finding in the new files)
- `cargo clippy --locked -p <crate> --no-default-features --all-targets -- -D warnings` and `cargo test --locked -p <crate> --no-default-features` for `bunny-geom`, `bunny-codec`, `bunny-mesh`, `bunny-query`
- Gate proof: a type error in `bvh_ray_query.rs` made `cargo test --locked --workspace --all-targets --all-features` exit 101; a wrong pinned primitive index made the example's test fail; an `unwrap()` in `float_boundary.rs` made `code-dojo-rust --all` report `panic-path`. Each was restored.
- The expected mesh hash was computed outside Rust with Python `hashlib` over the framing `compute_mesh_hash` documents.

Known risks:
- Local Rust is 1.100.0; CI pins 1.96.0. Clippy on 1.100 reports lints in files this change does not touch.
- `wasm-pack test` builds the examples for `wasm32-unknown-unknown`; that target is not installed locally, so only CI checks it.

Human reviewer:
@flyingrobots
