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
cargo run --locked -p xtask -- topic-docs; cargo run --locked -p xtask -- repo-respect check
--branch.

Known risks:
None yet; see the implementation commit.

Human reviewer:
James Ross (maintainer); review pending.
