# Receipt: Compressed Mesh Encoder

Task:
flyingrobots/bunny#124: an encoder for the existing Bunny compressed mesh profile v1
(`docs/goalposts/v0.4.0-gp3.md`, decoded by `decode_compressed_mesh`) in `bunny-codec`:
`encode_compressed_mesh`, `encode_compressed_mesh_into`, `encoded_compressed_mesh_len` and
`CompressedMeshEncodeError`. Additive: the profile, the decoder, the committed fixture and every
existing test, golden and ID are unchanged. The index width comes from the caller's
`IndexBufferLayout` variant, so the bytes are a function of the records alone.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*, Cargo.toml (workspace lints)
.repo-respect/receipts/2026-11-24-uniform-grid-broadphase.md
docs/goalposts/v0.4.0-gp3.md (the profile), docs/README.md, docs/topics/uniform-grid/*
crates/bunny-codec/{Cargo.toml,README.md,src/lib.rs,src/compressed.rs,src/compressed/*}
crates/bunny-codec/tests/{compressed_tests.rs,ply_allocation_tests.rs,fixtures/README.md}
crates/bunny-codec/tests/fixtures/canonical_compressed_triangle.bunny.hex
crates/bunny-mesh/src/lib.rs (QuantizedVertex, Triangle16, Triangle32, IndexBufferLayout)
crates/bunny-geom/src/lib.rs (FixedAabb3::try_new)
xtask/src/{code_dojo.rs,topic_docs.rs} (limits, case statuses)

Files edited:
crates/bunny-codec/src/compressed_encode.rs (new: the three functions)
crates/bunny-codec/src/compressed_encode/plan.rs (new: counts, capacity, lengths, bounds, indices)
crates/bunny-codec/src/compressed_encode/write.rs (new: the stream as one byte iterator)
crates/bunny-codec/src/compressed_encode/error.rs (new: CompressedMeshEncodeError)
crates/bunny-codec/src/lib.rs (module and exports, added lines only)
crates/bunny-codec/README.md (one table row), CHANGELOG.md, docs/README.md (topic row, sorted)
docs/topics/compressed-mesh/README.md (new chapter)
crates/bunny-codec/tests/compressed_encode_tests.rs (new)
crates/bunny-codec/tests/compressed_encode_allocation_tests.rs (new)
docs/topics/compressed-mesh/test-plan.md (new: CMESH-REQ-001 to -007, CMESH-TP-001 to -013)
.repo-respect/receipts/2026-11-24-compressed-mesh-encoder.md

Topic docs:
docs/topics/compressed-mesh/: a new topic with its own prefix CMESH-, starting at 001. No other
ledger uses CMESH-.

Generated artifacts:
None.

Checks run:
First commit (tests and planned cases only): the encoder tests fail to compile, because
`bunny_codec::encode_compressed_mesh` does not exist yet.
Implementation commit, each its own command on the host (local Rust 1.100.0; CI uses the pinned
1.96.0): cargo fmt --all --check; cargo test --locked --workspace --all-targets --all-features;
cargo run --locked -p xtask -- code-dojo-generated; code-dojo-rust --all; code-dojo-determinism
--enforce; code-dojo-perf-groups; topic-docs (zero duplicate IDs); repo-respect check --branch.
cargo clippy --locked -p bunny-codec --all-targets --all-features -- -D warnings clean.
Mutations: sixteen, each applied to the encoder, run with cargo test --no-fail-fast -p
bunny-codec --test compressed_encode_tests --test compressed_encode_allocation_tests, and
restored; each failed at least one case (the test plan's table).
The pinned hex and seeded digests were computed by a separate Python model (posted on the pull
request, not in the tree).
Not run locally: wasm-pack (CI's WebAssembly job), cargo deny and markdownlint-cli2 (CI's Dojo
job), the pinned 1.96.0 clippy.

Known risks:
`LengthOverflow` is not exercised: it needs a usize narrower than 25 bits. The canonical-bytes
property is tested on the constructed and seeded streams and argued for the rest. CMESH-TP-005
encodes and decodes an 18 MB stream, the largest the profile admits, on every target including
wasm32.

Human reviewer:
James Ross (maintainer); review pending.
