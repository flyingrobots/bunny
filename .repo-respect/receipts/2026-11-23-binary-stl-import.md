# Receipt: Binary STL Import

Task:
flyingrobots/bunny#127: a deterministic binary STL import profile,
`bunny-binary-stl-triangle-mesh/1`, read by a new `parse_binary_stl` in `bunny-codec`. Exact
length rule (`84 + 50 n` in `u64`), typed refusals naming the triangle, vertex and axis of a
non-finite or out-of-range coordinate, coordinates converted by `FixedQ32_32::try_from_f32`
unchanged, stored vertex order kept with no welding, stored normal and attribute ignored.
Additive: no existing decoder, error type, fixture, golden, digest or ID changes.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*, .repo-respect/receipts/2026-11-23-shape-library.md
crates/bunny-codec/src/{lib.rs,ply.rs,ply/header.rs,ply_ascii.rs,ply_ascii/{body,error}.rs}
crates/bunny-codec/tests/{ply_ascii_limits_tests,ply_ascii_allocation_tests}.rs (conventions)
crates/bunny-codec/{Cargo.toml,README.md,tests/fixtures/README.md,tests/fixtures/stanford-bunny-ascii/README.md}
crates/bunny-num/src/fixed_q32_32.rs and fixed_q32_32/conversions.rs (try_from_f32)
docs/topics/{ascii-ply,fixed-point-rounding,release-packaging}/README.md, docs/NUMERIC_CONSTITUTION.md
xtask/src/{topic_docs.rs,code_dojo.rs} (ledger and lint rules)
flyingrobots/bunny#127

Files edited:
crates/bunny-codec/src/stl.rs, src/stl/{error,layout}.rs and src/stl/layout/tests.rs (new)
crates/bunny-codec/src/lib.rs (one module line and one export block, at the end)
crates/bunny-codec/README.md (one table row and one section, at the end)
crates/bunny-codec/tests/fixtures/README.md (one provenance section, at the end)
crates/bunny-codec/tests/stl_support/mod.rs (new: byte builders and seeded generator)
crates/bunny-codec/tests/{stl_tests,stl_oracle_tests,stl_allocation_tests}.rs (new)
docs/topics/binary-stl/README.md and test-plan.md (new)
docs/topics/ascii-ply/README.md (parse_binary_stl added to the list of readers available
without default features; no requirement or case changed)
docs/README.md (one topic row, sorted), CHANGELOG.md (one Unreleased entry)
.repo-respect/receipts/2026-11-23-binary-stl-import.md

Topic docs:
docs/topics/binary-stl/ is a new topic with its own STL- prefix (STL-REQ-001 to -007,
STL-TP-001 to -022 and -030); no other topic uses it.

Generated artifacts:
None.

Checks run:
First commit (tests and planned cases only): the STL tests failed to compile because
`parse_binary_stl` and its types did not exist. topic-docs and repo-respect check --branch.
Implementation commit, each its own command (local Rust 1.100.0; CI pins 1.96.0):
cargo fmt --all --check; cargo test --locked --workspace --all-targets --all-features;
code-dojo-generated; code-dojo-rust --all; code-dojo-determinism --enforce;
code-dojo-perf-groups; topic-docs; repo-respect check --branch; cargo clippy --locked -p
bunny-codec --all-targets --all-features -D warnings (clean).
Mutations: fourteen applied to stl.rs and stl/layout.rs with the STL tests run and the
sources restored; every one fails at least one case (test-plan table).
Not run locally: wasm-pack and any 32-bit target (no wasm32 or i686 standard library on this
machine; CI's WebAssembly job runs the wasm_bindgen_test cases on wasm32), cargo deny and
markdownlint-cli2 (not installed; CI's Dojo job runs them).

After merging origin/main (flyingrobots/bunny#485 landed, CHANGELOG conflict resolved
keeping both entries): the chapter names `bunny_mesh::validate_topology` in place of the
open pull request; workspace tests and topic-docs rerun clean.

Known risks:
The rules the issue did not fix are choices listed in the pull request for the owner:
trailing bytes refused, empty mesh accepted, normals ignored rather than exposed, no
welding, iterator items as Result like the ASCII PLY reader, StlError non_exhaustive.
The triangle limit is reachable only with inputs of at least 71 582 788 384 bytes, so it is
tested on the private layout check, not through parse_binary_stl.

Human reviewer:
James Ross (maintainer); review pending.
