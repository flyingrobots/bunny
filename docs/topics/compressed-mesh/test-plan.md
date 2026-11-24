# Compressed Mesh Encoder Test Plan

This document defines how Bunny verifies the encoder for the Bunny compressed
mesh profile v1 in `bunny-codec`: `encode_compressed_mesh`,
`encode_compressed_mesh_into` and `encoded_compressed_mesh_len`. The decoder,
`decode_compressed_mesh`, and its profile are unchanged; their evidence is in
`crates/bunny-codec/tests/compressed_tests.rs` and
`docs/goalposts/v0.4.0-gp3.md`.

The current topic chapter is [`README.md`](README.md). This test plan is a
living verification design: it records requirements, executable evidence and
known gaps.

## Scope

- the bytes the encoder writes, field by field, against the profile v1 layout;
- the round trip through the real decoder, record by record;
- canonical bytes: one stream per mesh, and every accepted stream tested
  re-encodes to itself;
- the domain: counts, the width-16 vertex capacity, bounds and indices, each
  refused with a typed error in a stated order;
- the caller-owned output buffer and the allocation contract.

## Test Goals

- Compare every stream against two oracles that share no code with the
  encoder: a test-local writer that pushes each field onto a `Vec`, and a
  separate Python model (posted on the pull request, not in the tree) built on
  `struct.pack`, which pins the hex of constructed meshes and the length and
  FNV-1a-64 digest of each seeded mesh.
- Decode every encoded stream with `decode_compressed_mesh` and compare the
  bounds, counts, index width and every vertex and triangle record.
- Put constructed counts and indices on each limit and one unit either side:
  0, 1, 1 000 000 and 1 000 001 records; 65 536 and 65 537 vertices at width 16;
  indices `vertex_count - 1` and `vertex_count`, and `u32::MAX`.
- Generate eight seeded meshes from a 64-bit linear congruential generator;
  print each seed with its counts and width.
- Break each rule on purpose and record which case fails.

## Requirements

The table is the human-readable view. The fenced TOML block immediately after
it is the contract graph consumed by `cargo run --locked -p xtask --
topic-docs`.

| ID | Requirement | Current Source |
| --- | --- | --- |
| CMESH-REQ-001 | The encoder writes exactly the profile v1 layout: magic `BUNNYQZ!`, version 1, the index width of the caller's layout (16 or 32), zero flags, the counts, the exact payload length, the six raw bound values, then the vertex and triangle records, all little-endian. | `README.md#byte-layout` |
| CMESH-REQ-002 | Decoding an encoded stream with `decode_compressed_mesh` returns the same bounds, counts, index width and every vertex and triangle record. | `README.md#round-trip-and-canonical-bytes` |
| CMESH-REQ-003 | The bytes are a function of the bounds, the vertices and the triangle layout alone, so a mesh has one stream; re-encoding the records decoded from an accepted stream reproduces it, and the committed fixture's records encode to its committed bytes. | `README.md#round-trip-and-canonical-bytes` |
| CMESH-REQ-004 | The domain is that of the decoder: 1 to 1 000 000 vertices and triangles, at most 65 536 vertices at width 16, bounds not inverted on any axis, every index below the vertex count. Anything else is refused with a typed error naming the value, checked in the order vertex count, triangle count, width-16 capacity, bounds, indices (first triangle, then first corner), output size. | `README.md#domain-and-refusals` |
| CMESH-REQ-005 | `encode_compressed_mesh_into` writes the stream to the front of the caller's buffer and returns its length; a buffer shorter than the stream is refused naming both lengths; a refusal writes nothing and bytes past the stream are untouched. | `README.md#output-buffer` |
| CMESH-REQ-006 | `encode_compressed_mesh_into` and `encoded_compressed_mesh_len` allocate nothing; `encode_compressed_mesh` allocates one block of exactly the stream's length on success and nothing on a refusal. | `README.md#allocation-contract` |
| CMESH-REQ-007 | Every length is computed without truncation: the largest stream, 18 000 076 bytes, fits a 32-bit `usize`, and a length that does not fit is refused rather than wrapped. | `README.md#lengths-and-targets` |

```toml
[[requirement]]
id = "CMESH-REQ-001"
summary = "The encoder writes exactly the profile v1 layout: magic, version 1, the caller's index width, zero flags, the counts, the exact payload length, the six raw bound values, then the vertex and triangle records, all little-endian."
status = "active"

[[requirement]]
id = "CMESH-REQ-002"
summary = "Decoding an encoded stream with decode_compressed_mesh returns the same bounds, counts, index width and every vertex and triangle record."
status = "active"

[[requirement]]
id = "CMESH-REQ-003"
summary = "The bytes are a function of the bounds, vertices and triangle layout alone; re-encoding the records decoded from an accepted stream reproduces it, and the committed fixture's records encode to its committed bytes."
status = "active"

[[requirement]]
id = "CMESH-REQ-004"
summary = "The domain is the decoder's: 1 to 1 000 000 vertices and triangles, at most 65 536 vertices at width 16, bounds not inverted, indices below the vertex count; anything else is a typed error naming the value, in the documented order."
status = "active"

[[requirement]]
id = "CMESH-REQ-005"
summary = "encode_compressed_mesh_into writes the stream to the front of the buffer and returns its length; a short buffer is refused naming both lengths; a refusal writes nothing and bytes past the stream are untouched."
status = "active"

[[requirement]]
id = "CMESH-REQ-006"
summary = "encode_compressed_mesh_into and encoded_compressed_mesh_len allocate nothing; encode_compressed_mesh allocates one block of exactly the stream's length on success and nothing on a refusal."
status = "active"

[[requirement]]
id = "CMESH-REQ-007"
summary = "Every length is computed without truncation: the largest stream, 18 000 076 bytes, fits a 32-bit usize, and a length that does not fit is refused rather than wrapped."
status = "active"
```

## Fixtures

The committed fixture `crates/bunny-codec/tests/fixtures/canonical_compressed_triangle.bunny.hex`
(unchanged). Every other input is constructed or seeded in
`crates/bunny-codec/tests/compressed_encode_tests.rs`.

## Test Cases

| ID | Category | Requirements | Case | Oracle | Test |
| --- | --- | --- | --- | --- | --- |
| CMESH-TP-001 | Fixture | CMESH-REQ-001, CMESH-REQ-002, CMESH-REQ-003 | The committed fixture's records encode to its committed bytes and round trip; its width-32 form, as `compressed_tests.rs` derives it, likewise; both re-encode to themselves after decoding. | The committed hex. | `crates/bunny-codec/tests/compressed_encode_tests.rs::cmesh_tp_001_encoding_the_committed_fixture_records_reproduces_its_bytes` |
| CMESH-TP-002 | Extremes | CMESH-REQ-001, CMESH-REQ-002, CMESH-REQ-003 | A width-32 mesh with bounds at `i64::MIN`, `i64::MAX`, -1 and 0 raw and coordinates 0, 1, 0x8000 and `u16::MAX`; a width-16 mesh with flat axes and a negative bound: pinned hex. | The Python model's hex and the test-local writer. | `crates/bunny-codec/tests/compressed_encode_tests.rs::cmesh_tp_002_constructed_extremes_match_the_independent_model_byte_for_byte` |
| CMESH-TP-003 | Seeded | CMESH-REQ-001, CMESH-REQ-002 | Eight seeded meshes (1 to 300 vertices and triangles, random bounds, both widths) round trip record by record and equal the pinned length and FNV-1a-64 digest. | The Python model's pins and the test-local writer. | `crates/bunny-codec/tests/compressed_encode_tests.rs::cmesh_tp_003_seeded_meshes_round_trip_and_match_the_pinned_model` |
| CMESH-TP-004 | Canonical | CMESH-REQ-003 | Both widths of each seeded mesh, written by the test-local writer, decode and re-encode to the same bytes. | The accepted stream itself. | `crates/bunny-codec/tests/compressed_encode_tests.rs::cmesh_tp_004_every_accepted_stream_tested_reencodes_to_itself` |
| CMESH-TP-005 | Counts | CMESH-REQ-004, CMESH-REQ-007 | 0 and 1 000 001 vertices or triangles are refused naming the count; the largest stream, 1 000 000 vertices and width-32 triangles, is 18 000 076 bytes and round trips. | Hand-derived errors and length; the test-local writer. | `crates/bunny-codec/tests/compressed_encode_tests.rs::cmesh_tp_005_counts_from_one_to_one_million_are_accepted_and_others_refused` |
| CMESH-TP-006 | Width 16 | CMESH-REQ-004 | 65 536 vertices with index 65 535 round trip at width 16; 65 537 are refused at width 16 and accepted at width 32. | Hand-derived errors. | `crates/bunny-codec/tests/compressed_encode_tests.rs::cmesh_tp_006_width16_holds_at_most_65536_vertices` |
| CMESH-TP-007 | Indices | CMESH-REQ-004 | Index `vertex_count - 1` is accepted and `vertex_count` refused at each corner and both widths, naming the triangle and index; the first offending triangle and corner win; `u32::MAX` is refused. | Hand-derived errors. | `crates/bunny-codec/tests/compressed_encode_tests.rs::cmesh_tp_007_indices_below_the_vertex_count_are_accepted_and_the_first_other_refused` |
| CMESH-TP-008 | Bounds | CMESH-REQ-004 | A min one raw unit above the max on each axis is refused; flat bounds and the whole raw range round trip. | Hand-derived errors. | `crates/bunny-codec/tests/compressed_encode_tests.rs::cmesh_tp_008_bounds_inverted_on_any_axis_are_refused_and_flat_or_extreme_accepted` |
| CMESH-TP-009 | Order | CMESH-REQ-004 | Meshes with several faults report the first in the documented order. | Hand-derived errors. | `crates/bunny-codec/tests/compressed_encode_tests.rs::cmesh_tp_009_refusals_are_checked_in_the_documented_order` |
| CMESH-TP-010 | Output buffer | CMESH-REQ-005 | Buffers of 0, 1, 75, 76 and required - 1 bytes are refused naming both lengths and left untouched; required, + 1 and + 64 bytes return the length, hold the fixture and leave the tail untouched; earlier contents do not matter. | The committed hex; sentinels. | `crates/bunny-codec/tests/compressed_encode_tests.rs::cmesh_tp_010_the_output_buffer_must_hold_the_stream_and_its_tail_is_untouched` |
| CMESH-TP-011 | Errors | CMESH-REQ-004, CMESH-REQ-005 | Every error variant displays a message naming its values. | Hand-listed substrings. | `crates/bunny-codec/tests/compressed_encode_tests.rs::cmesh_tp_011_errors_display_their_values` |
| CMESH-TP-012 | Allocation | CMESH-REQ-006 | A counting allocator sees 0 allocations for the length and the buffer encoder, 1 for the allocating encoder (capacity equal to length) and 0 for a refusal, on a 500-vertex, 400-triangle mesh. | Thread-scoped allocation count. | `crates/bunny-codec/tests/compressed_encode_allocation_tests.rs::cmesh_tp_012_the_encoder_allocates_only_the_returned_stream` |
| CMESH-TP-013 | Mutations | CMESH-REQ-001, CMESH-REQ-003, CMESH-REQ-004, CMESH-REQ-005 | Each deliberate mutation of the encoder makes at least one case fail. | Each mutant's test run. | [Mutation checks](#mutation-checks) |

```toml
[[case]]
id = "CMESH-TP-001"
requirements = ["CMESH-REQ-001", "CMESH-REQ-002", "CMESH-REQ-003"]
evidence = "test"
test = "crates/bunny-codec/tests/compressed_encode_tests.rs::cmesh_tp_001_encoding_the_committed_fixture_records_reproduces_its_bytes"
oracle = "The committed hex."
tier = "fast"
status = "implemented"

[[case]]
id = "CMESH-TP-002"
requirements = ["CMESH-REQ-001", "CMESH-REQ-002", "CMESH-REQ-003"]
evidence = "test"
test = "crates/bunny-codec/tests/compressed_encode_tests.rs::cmesh_tp_002_constructed_extremes_match_the_independent_model_byte_for_byte"
oracle = "The Python model's hex and the test-local writer."
tier = "fast"
status = "implemented"

[[case]]
id = "CMESH-TP-003"
requirements = ["CMESH-REQ-001", "CMESH-REQ-002"]
evidence = "test"
test = "crates/bunny-codec/tests/compressed_encode_tests.rs::cmesh_tp_003_seeded_meshes_round_trip_and_match_the_pinned_model"
oracle = "The Python model's pins and the test-local writer."
tier = "fast"
status = "implemented"

[[case]]
id = "CMESH-TP-004"
requirements = ["CMESH-REQ-003"]
evidence = "test"
test = "crates/bunny-codec/tests/compressed_encode_tests.rs::cmesh_tp_004_every_accepted_stream_tested_reencodes_to_itself"
oracle = "The accepted stream itself."
tier = "fast"
status = "implemented"

[[case]]
id = "CMESH-TP-005"
requirements = ["CMESH-REQ-004", "CMESH-REQ-007"]
evidence = "test"
test = "crates/bunny-codec/tests/compressed_encode_tests.rs::cmesh_tp_005_counts_from_one_to_one_million_are_accepted_and_others_refused"
oracle = "Hand-derived errors and length; the test-local writer."
tier = "fast"
status = "implemented"

[[case]]
id = "CMESH-TP-006"
requirements = ["CMESH-REQ-004"]
evidence = "test"
test = "crates/bunny-codec/tests/compressed_encode_tests.rs::cmesh_tp_006_width16_holds_at_most_65536_vertices"
oracle = "Hand-derived errors."
tier = "fast"
status = "implemented"

[[case]]
id = "CMESH-TP-007"
requirements = ["CMESH-REQ-004"]
evidence = "test"
test = "crates/bunny-codec/tests/compressed_encode_tests.rs::cmesh_tp_007_indices_below_the_vertex_count_are_accepted_and_the_first_other_refused"
oracle = "Hand-derived errors."
tier = "fast"
status = "implemented"

[[case]]
id = "CMESH-TP-008"
requirements = ["CMESH-REQ-004"]
evidence = "test"
test = "crates/bunny-codec/tests/compressed_encode_tests.rs::cmesh_tp_008_bounds_inverted_on_any_axis_are_refused_and_flat_or_extreme_accepted"
oracle = "Hand-derived errors."
tier = "fast"
status = "implemented"

[[case]]
id = "CMESH-TP-009"
requirements = ["CMESH-REQ-004"]
evidence = "test"
test = "crates/bunny-codec/tests/compressed_encode_tests.rs::cmesh_tp_009_refusals_are_checked_in_the_documented_order"
oracle = "Hand-derived errors."
tier = "fast"
status = "implemented"

[[case]]
id = "CMESH-TP-010"
requirements = ["CMESH-REQ-005"]
evidence = "test"
test = "crates/bunny-codec/tests/compressed_encode_tests.rs::cmesh_tp_010_the_output_buffer_must_hold_the_stream_and_its_tail_is_untouched"
oracle = "The committed hex; sentinels."
tier = "fast"
status = "implemented"

[[case]]
id = "CMESH-TP-011"
requirements = ["CMESH-REQ-004", "CMESH-REQ-005"]
evidence = "test"
test = "crates/bunny-codec/tests/compressed_encode_tests.rs::cmesh_tp_011_errors_display_their_values"
oracle = "Hand-listed substrings."
tier = "fast"
status = "implemented"

[[case]]
id = "CMESH-TP-012"
requirements = ["CMESH-REQ-006"]
evidence = "test"
test = "crates/bunny-codec/tests/compressed_encode_allocation_tests.rs::cmesh_tp_012_the_encoder_allocates_only_the_returned_stream"
oracle = "Thread-scoped allocation count."
tier = "fast"
status = "implemented"

[[case]]
id = "CMESH-TP-013"
requirements = ["CMESH-REQ-001", "CMESH-REQ-003", "CMESH-REQ-004", "CMESH-REQ-005"]
evidence = "manual-audit"
artifact = "docs/topics/compressed-mesh/test-plan.md#mutation-checks"
oracle = "Each deliberate mutation makes at least one case fail."
tier = "manual"
status = "implemented"
```

## Mutation Checks

Each mutation below was applied to the encoder on the host (then restored),
`cargo test --locked --no-fail-fast -p bunny-codec --test
compressed_encode_tests --test compressed_encode_allocation_tests` was run, and
the cases that failed were recorded.

| Mutation | Cases that fail |
| --- | --- |
| Version byte 2 | CMESH-TP-001, 002, 003, 004, 005, 006, 007, 008, 010 |
| Non-zero flags | CMESH-TP-001, 002, 003, 004, 005, 006, 007, 008, 010 |
| Vertex `x` and `y` swapped | CMESH-TP-001, 002, 003, 004, 005, 006, 007, 010 |
| Vertex count written big-endian | CMESH-TP-001, 002, 003, 004, 005, 006, 007, 008, 010 |
| Bounds max written before min | CMESH-TP-001, 002, 003, 004, 005, 006, 007, 008, 010 |
| Width-32 stride taken as 6 in the payload length | CMESH-TP-001, 002, 003, 004, 005, 006, 007 |
| Width-16 corners `v1` and `v2` swapped | CMESH-TP-001, 003, 004, 006, 010 |
| Zero counts accepted | CMESH-TP-005, 009 |
| Count limit 1 000 001 | CMESH-TP-005 |
| Width-16 capacity 65 535 | CMESH-TP-006 |
| Index equal to the vertex count accepted | CMESH-TP-007, 012 |
| The last offending corner reported, not the first | CMESH-TP-007 |
| Bounds check removed | CMESH-TP-008, 009 |
| Bounds checked after the indices | CMESH-TP-009 |
| The whole output buffer written (tail zeroed) | CMESH-TP-010 |
| The allocating encoder allocates before validating | CMESH-TP-012 |

## Determinism Obligations And Evidence

The encoder uses only integer conversions to little-endian bytes and checked
integer arithmetic on `usize` and `u64`; there is no floating point, no map,
no host state and no target-specific path. The round-trip, pinned and seeded
cases run in `cargo test --workspace` on Linux, macOS and Windows in the
`determinism-matrix` CI job and under WebAssembly (`wasm32`, a 32-bit `usize`)
in `wasm-headless`, where they must equal the same pins. The allocation case is
not built for `wasm32`, as the crate's other allocation tests are not.

## Gaps

- `CompressedMeshEncodeError::LengthOverflow` is not exercised: the largest
  stream is 18 000 076 bytes, so it can occur only where `usize` is narrower
  than 25 bits, and `bunny-codec` needs `std`, which no such target has.
- The canonical-bytes property (CMESH-REQ-003) is proved for the streams the
  tests construct, not for every accepted stream; the argument for all of them
  is in the chapter.
