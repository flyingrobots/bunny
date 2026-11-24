# Binary STL Test Plan

This document defines how Bunny proves the binary STL reader. The contract is
[`README.md`](README.md).

## Scope

`parse_binary_stl` in `bunny-codec`: the profile `bunny-binary-stl-triangle-mesh/1`, its
length rule, the coordinate conversion it inherits from `FixedQ32_32::try_from_f32`, the
winding, normal and attribute rules, the output form and the error contract.

## Test Goals

- Give every refusal its own typed error, with the triangle, vertex and axis of a bad
  coordinate.
- Prove the length rule at and around every overflow boundary without a large buffer.
- Prove the reader against an independent reader written differently, bit for bit, on
  seeded generated files and on random and mutated bytes.
- Keep every test native and WASM-compatible (`wasm_bindgen_test`), except the native
  allocation witness. Use no float in a fixture: coordinates are written as bit patterns.

## Non-Goals

- ASCII STL, welding of equal positions, normal computation or repair, and topology
  validation.

## Requirements

| ID | Requirement | Current Source |
| --- | --- | --- |
| STL-REQ-001 | The profile is binary STL only: an 80-byte header that is not interpreted, a little-endian `u32` triangle count and 50-byte triangle records; a header beginning with `solid` is read as binary and ASCII STL is not detected. | `README.md#the-profile` |
| STL-REQ-002 | An input shorter than 84 bytes is `TooShort`; otherwise the length must be exactly `84 + 50 n`, computed in `u64` without overflow on any target, and any other length (truncated, a count claiming more or fewer triangles, trailing bytes, a big-endian count) is `LengthMismatch` with the count, the expected and the actual length; the count never sizes memory. | `README.md#the-profile` |
| STL-REQ-003 | With the length right, a count above `u32::MAX / 3` is `TooManyTriangles`, after the length rule. | `README.md#the-profile` |
| STL-REQ-004 | Each vertex coordinate converts by `FixedQ32_32::try_from_f32` unchanged (round half to even of the exact value times 2^32); a NaN or infinity is `NonFiniteVertex` and a finite value outside Q32.32 is `VertexOutOfRange`, each naming the triangle, vertex and axis of the first bad coordinate in file order. | `README.md#the-conversion` |
| STL-REQ-005 | The stored vertex order is kept, triangle `i` is `(3i, 3i+1, 3i+2)` with no welding; the stored normal and the attribute byte count are never read, so a non-finite normal is accepted; degenerate triangles are kept; zero triangles is an empty mesh. | `README.md#winding-normals-and-attributes` |
| STL-REQ-006 | The reader validates everything up front and returns a borrowed view whose iterators restart, yield exactly `3n` vertices and `n` triangles, all `Ok`, and allocate nothing. | `README.md#output` |
| STL-REQ-007 | Errors are typed, `Copy` and `Eq`, display their position and implement `core::error::Error`. | `README.md#errors` |

```toml
[[requirement]]
id = "STL-REQ-001"
summary = "The profile is binary STL only: an 80-byte header that is not interpreted, a little-endian u32 triangle count and 50-byte triangle records; a header beginning with solid is read as binary and ASCII STL is not detected."
status = "active"

[[requirement]]
id = "STL-REQ-002"
summary = "An input shorter than 84 bytes is TooShort; otherwise the length must be exactly 84 + 50 n, computed in u64 without overflow on any target, and any other length (truncated, a count claiming more or fewer triangles, trailing bytes, a big-endian count) is LengthMismatch with the count, the expected and the actual length; the count never sizes memory."
status = "active"

[[requirement]]
id = "STL-REQ-003"
summary = "With the length right, a count above u32::MAX / 3 is TooManyTriangles, after the length rule."
status = "active"

[[requirement]]
id = "STL-REQ-004"
summary = "Each vertex coordinate converts by FixedQ32_32::try_from_f32 unchanged (round half to even of the exact value times 2^32); a NaN or infinity is NonFiniteVertex and a finite value outside Q32.32 is VertexOutOfRange, each naming the triangle, vertex and axis of the first bad coordinate in file order."
status = "active"

[[requirement]]
id = "STL-REQ-005"
summary = "The stored vertex order is kept, triangle i is (3i, 3i+1, 3i+2) with no welding; the stored normal and the attribute byte count are never read, so a non-finite normal is accepted; degenerate triangles are kept; zero triangles is an empty mesh."
status = "active"

[[requirement]]
id = "STL-REQ-006"
summary = "The reader validates everything up front and returns a borrowed view whose iterators restart, yield exactly 3n vertices and n triangles, all Ok, and allocate nothing."
status = "active"

[[requirement]]
id = "STL-REQ-007"
summary = "Errors are typed, Copy and Eq, display their position and implement core::error::Error."
status = "active"
```

## Test Cases

| ID | Category | Requirements | Oracle | Test |
| --- | --- | --- | --- | --- |
| STL-TP-001 | Identity | STL-REQ-001 | The profile identifier string and `BINARY_STL_MAX_TRIANGLES * 3 == u32::MAX`. | `crates/bunny-codec/tests/stl_tests.rs::stl_tp_001_profile_has_a_stable_identity` |
| STL-TP-002 | Golden path | STL-REQ-001, STL-REQ-005 | A unit tetrahedron: 284 bytes, count bytes `04 00 00 00`, twelve hand-derived vertices in stored order and triangles `(3i, 3i+1, 3i+2)`. | `crates/bunny-codec/tests/stl_tests.rs::stl_tp_002_tetrahedron_reads_in_file_order_without_welding` |
| STL-TP-003 | Conversion | STL-REQ-004 | Hand-derived raw values of 0.5, -2.25, 0.125 and 1. | `crates/bunny-codec/tests/stl_tests.rs::stl_tp_003_coordinates_convert_exactly` |
| STL-TP-004 | Empty mesh | STL-REQ-005 | An 84-byte file with count 0 is a mesh of zero triangles and vertices. | `crates/bunny-codec/tests/stl_tests.rs::stl_tp_004_zero_triangles_is_an_empty_mesh` |
| STL-TP-005 | Header | STL-REQ-001 | Headers `solid tetrahedron`, `solid`, all zero and all `0xFF` read to the same mesh; an ASCII STL text is `LengthMismatch` with the count its bytes 80 to 83 spell. | `crates/bunny-codec/tests/stl_tests.rs::stl_tp_005_the_header_is_not_interpreted` |
| STL-TP-006 | Normals, attributes | STL-REQ-005 | Four normals (zero, wrong, NaN and infinities, all-ones NaN) times four attribute values read to the same mesh as the reference. | `crates/bunny-codec/tests/stl_tests.rs::stl_tp_006_stored_normals_and_attributes_are_ignored` |
| STL-TP-007 | Winding, degenerates | STL-REQ-005 | A facet wound against its normal keeps its order; a point and a collinear triangle are kept with hand-derived vertices. | `crates/bunny-codec/tests/stl_tests.rs::stl_tp_007_winding_is_kept_and_degenerate_triangles_are_kept` |
| STL-TP-008 | Too short | STL-REQ-002 | Lengths 0, 1, 79, 80 and 83 are `TooShort` with that length; 84 zero bytes are accepted. | `crates/bunny-codec/tests/stl_tests.rs::stl_tp_008_input_shorter_than_the_header_and_count_is_too_short` |
| STL-TP-009 | Length rule | STL-REQ-002 | Seven truncations (mid-triangle and whole triangles), five trailing-byte lengths, an empty mesh plus one byte, and five counts over four facets: exact `LengthMismatch` fields. | `crates/bunny-codec/tests/stl_tests.rs::stl_tp_009_the_length_must_be_exactly_84_plus_50_per_triangle` |
| STL-TP-010 | Overflow boundaries | STL-REQ-002, STL-REQ-003 | Counts 85 899 344 and 85 899 345 (either side of a `u32` length), `u32::MAX / 3` and one either side, `u32::MAX - 1` and `u32::MAX` over 0, 1 and 4 facets are `LengthMismatch` with the exact `u64` expected length. | `crates/bunny-codec/tests/stl_tests.rs::stl_tp_010_counts_at_the_overflow_boundaries_are_length_mismatches` |
| STL-TP-011 | Byte order | STL-REQ-002 | A count of 1 written big-endian is 16 777 216 triangles and `LengthMismatch` over 134 bytes. | `crates/bunny-codec/tests/stl_tests.rs::stl_tp_011_a_big_endian_count_is_read_little_endian` |
| STL-TP-012 | Non-finite | STL-REQ-004 | Quiet, signalling and negative NaN and both infinities in each of the nine coordinate positions of triangle 1 are `NonFiniteVertex` naming it; of two bad coordinates the first in file order is reported. | `crates/bunny-codec/tests/stl_tests.rs::stl_tp_012_every_non_finite_vertex_coordinate_is_refused_where_it_is` |
| STL-TP-013 | Out of range | STL-REQ-004 | `2^31`, `-2^31 - 256`, `f32::MAX`, `-f32::MAX` and `2^63` in each of the nine positions of triangle 3 are `VertexOutOfRange`; out of range before non-finite in file order wins. | `crates/bunny-codec/tests/stl_tests.rs::stl_tp_013_coordinates_outside_q32_32_are_refused_where_they_are` |
| STL-TP-014 | Range ends, ties | STL-REQ-004 | Twelve bit patterns (the last values in range at both ends, ties at 0.5, 1.5, 2.5 and 3.5 raw units, subnormal, -0, just above a tie) in each axis give the hand-derived raw value, and `try_from_f32` gives the same. | `crates/bunny-codec/tests/stl_tests.rs::stl_tp_014_the_range_ends_and_ties_follow_try_from_f32` |
| STL-TP-015 | Errors | STL-REQ-007 | Exact `Display` text of each variant and use as `core::error::Error` with no source. | `crates/bunny-codec/tests/stl_tests.rs::stl_tp_015_errors_display_their_position_and_are_errors` |
| STL-TP-016 | Iterators | STL-REQ-006 | Iterators restart, a copied view reads the same, and the counts match `vertex_count` and `triangle_count`. | `crates/bunny-codec/tests/stl_tests.rs::stl_tp_016_iterators_restart_and_yield_exactly_the_counts` |
| STL-TP-017 | Triangle limit | STL-REQ-002, STL-REQ-003 | The private layout check with exact lengths: `u32::MAX / 3` accepted, one more and `u32::MAX` `TooManyTriangles`; a wrong length at those counts is `LengthMismatch` first. | `crates/bunny-codec/src/stl/layout/tests.rs::stl_tp_017_the_triangle_limit_applies_after_the_length_rule` |
| STL-TP-020 | Oracle, seeded | STL-REQ-002, STL-REQ-004, STL-REQ-005 | 3 000 seeded files (seed `0x0571127`, up to 11 random facets each) and the 18 boundary patterns: the independent reader and `parse_binary_stl` agree bit for bit on every vertex or on the exact refusal; 1 269 meshes, 748 non-finite and 983 out-of-range refusals in the run that set the minimums of 1 000, 500 and 700. | `crates/bunny-codec/tests/stl_oracle_tests.rs::stl_tp_020_seeded_files_match_the_independent_reader_bit_for_bit` |
| STL-TP-021 | Fuzz | STL-REQ-002, STL-REQ-004, STL-REQ-006 | 20 000 random, mutated, recounted and valid byte strings (seed `0x0571127 ^ 0xF0`) never panic, every mesh obeys the length rule and the index rule, and every outcome equals the independent reader's; 3 801 meshes, 1 971 too short, 12 105 length mismatches, 855 non-finite and 1 268 out-of-range refusals in the run that set the minimums of 3 000, 1 500, 10 000, 600 and 1 000. | `crates/bunny-codec/tests/stl_oracle_tests.rs::stl_tp_021_random_and_mutated_bytes_never_panic_and_match_the_oracle` |
| STL-TP-022 | Oracle check | STL-REQ-004 | The oracle against fourteen hand-derived conversions and the tetrahedron. | `crates/bunny-codec/tests/stl_oracle_tests.rs::stl_tp_022_the_oracle_agrees_with_hand_derived_values` |
| STL-TP-030 | Allocation | STL-REQ-006 | Zero allocation blocks on the measuring thread for parse and iteration after warm-up, and for a `u32::MAX` count (native only). | `crates/bunny-codec/tests/stl_allocation_tests.rs::stl_tp_030_binary_stl_reader_allocates_zero_times_after_warm_up` |

```toml
[[case]]
id = "STL-TP-001"
requirements = ["STL-REQ-001"]
evidence = "test"
test = "crates/bunny-codec/tests/stl_tests.rs::stl_tp_001_profile_has_a_stable_identity"
oracle = "The profile identifier string and BINARY_STL_MAX_TRIANGLES * 3 == u32::MAX."
tier = "fast"
status = "implemented"

[[case]]
id = "STL-TP-002"
requirements = ["STL-REQ-001", "STL-REQ-005"]
evidence = "test"
test = "crates/bunny-codec/tests/stl_tests.rs::stl_tp_002_tetrahedron_reads_in_file_order_without_welding"
oracle = "A unit tetrahedron: 284 bytes, count bytes 04 00 00 00, twelve hand-derived vertices in stored order and triangles (3i, 3i+1, 3i+2)."
tier = "fast"
status = "implemented"

[[case]]
id = "STL-TP-003"
requirements = ["STL-REQ-004"]
evidence = "test"
test = "crates/bunny-codec/tests/stl_tests.rs::stl_tp_003_coordinates_convert_exactly"
oracle = "Hand-derived raw values of 0.5, -2.25, 0.125 and 1."
tier = "fast"
status = "implemented"

[[case]]
id = "STL-TP-004"
requirements = ["STL-REQ-005"]
evidence = "test"
test = "crates/bunny-codec/tests/stl_tests.rs::stl_tp_004_zero_triangles_is_an_empty_mesh"
oracle = "An 84-byte file with count 0 is a mesh of zero triangles and vertices."
tier = "fast"
status = "implemented"

[[case]]
id = "STL-TP-005"
requirements = ["STL-REQ-001"]
evidence = "test"
test = "crates/bunny-codec/tests/stl_tests.rs::stl_tp_005_the_header_is_not_interpreted"
oracle = "Headers solid tetrahedron, solid, all zero and all 0xFF read to the same mesh; an ASCII STL text is LengthMismatch with the count its bytes 80 to 83 spell."
tier = "fast"
status = "implemented"

[[case]]
id = "STL-TP-006"
requirements = ["STL-REQ-005"]
evidence = "test"
test = "crates/bunny-codec/tests/stl_tests.rs::stl_tp_006_stored_normals_and_attributes_are_ignored"
oracle = "Four normals (zero, wrong, NaN and infinities, all-ones NaN) times four attribute values read to the same mesh as the reference."
tier = "fast"
status = "implemented"

[[case]]
id = "STL-TP-007"
requirements = ["STL-REQ-005"]
evidence = "test"
test = "crates/bunny-codec/tests/stl_tests.rs::stl_tp_007_winding_is_kept_and_degenerate_triangles_are_kept"
oracle = "A facet wound against its normal keeps its order; a point and a collinear triangle are kept with hand-derived vertices."
tier = "fast"
status = "implemented"

[[case]]
id = "STL-TP-008"
requirements = ["STL-REQ-002"]
evidence = "test"
test = "crates/bunny-codec/tests/stl_tests.rs::stl_tp_008_input_shorter_than_the_header_and_count_is_too_short"
oracle = "Lengths 0, 1, 79, 80 and 83 are TooShort with that length; 84 zero bytes are accepted."
tier = "fast"
status = "implemented"

[[case]]
id = "STL-TP-009"
requirements = ["STL-REQ-002"]
evidence = "test"
test = "crates/bunny-codec/tests/stl_tests.rs::stl_tp_009_the_length_must_be_exactly_84_plus_50_per_triangle"
oracle = "Seven truncations (mid-triangle and whole triangles), five trailing-byte lengths, an empty mesh plus one byte, and five counts over four facets: exact LengthMismatch fields."
tier = "fast"
status = "implemented"

[[case]]
id = "STL-TP-010"
requirements = ["STL-REQ-002", "STL-REQ-003"]
evidence = "test"
test = "crates/bunny-codec/tests/stl_tests.rs::stl_tp_010_counts_at_the_overflow_boundaries_are_length_mismatches"
oracle = "Counts 85 899 344 and 85 899 345 (either side of a u32 length), u32::MAX / 3 and one either side, u32::MAX - 1 and u32::MAX over 0, 1 and 4 facets are LengthMismatch with the exact u64 expected length."
tier = "fast"
status = "implemented"

[[case]]
id = "STL-TP-011"
requirements = ["STL-REQ-002"]
evidence = "test"
test = "crates/bunny-codec/tests/stl_tests.rs::stl_tp_011_a_big_endian_count_is_read_little_endian"
oracle = "A count of 1 written big-endian is 16 777 216 triangles and LengthMismatch over 134 bytes."
tier = "fast"
status = "implemented"

[[case]]
id = "STL-TP-012"
requirements = ["STL-REQ-004"]
evidence = "test"
test = "crates/bunny-codec/tests/stl_tests.rs::stl_tp_012_every_non_finite_vertex_coordinate_is_refused_where_it_is"
oracle = "Quiet, signalling and negative NaN and both infinities in each of the nine coordinate positions of triangle 1 are NonFiniteVertex naming it; of two bad coordinates the first in file order is reported."
tier = "fast"
status = "implemented"

[[case]]
id = "STL-TP-013"
requirements = ["STL-REQ-004"]
evidence = "test"
test = "crates/bunny-codec/tests/stl_tests.rs::stl_tp_013_coordinates_outside_q32_32_are_refused_where_they_are"
oracle = "2^31, -2^31 - 256, f32::MAX, -f32::MAX and 2^63 in each of the nine positions of triangle 3 are VertexOutOfRange; out of range before non-finite in file order wins."
tier = "fast"
status = "implemented"

[[case]]
id = "STL-TP-014"
requirements = ["STL-REQ-004"]
evidence = "test"
test = "crates/bunny-codec/tests/stl_tests.rs::stl_tp_014_the_range_ends_and_ties_follow_try_from_f32"
oracle = "Twelve bit patterns (the last values in range at both ends, ties at 0.5, 1.5, 2.5 and 3.5 raw units, subnormal, -0, just above a tie) in each axis give the hand-derived raw value, and try_from_f32 gives the same."
tier = "fast"
status = "implemented"

[[case]]
id = "STL-TP-015"
requirements = ["STL-REQ-007"]
evidence = "test"
test = "crates/bunny-codec/tests/stl_tests.rs::stl_tp_015_errors_display_their_position_and_are_errors"
oracle = "Exact Display text of each variant and use as core::error::Error with no source."
tier = "fast"
status = "implemented"

[[case]]
id = "STL-TP-016"
requirements = ["STL-REQ-006"]
evidence = "test"
test = "crates/bunny-codec/tests/stl_tests.rs::stl_tp_016_iterators_restart_and_yield_exactly_the_counts"
oracle = "Iterators restart, a copied view reads the same, and the counts match vertex_count and triangle_count."
tier = "fast"
status = "implemented"

[[case]]
id = "STL-TP-017"
requirements = ["STL-REQ-002", "STL-REQ-003"]
evidence = "test"
test = "crates/bunny-codec/src/stl/layout/tests.rs::stl_tp_017_the_triangle_limit_applies_after_the_length_rule"
oracle = "The private layout check with exact lengths: u32::MAX / 3 accepted, one more and u32::MAX TooManyTriangles; a wrong length at those counts is LengthMismatch first."
tier = "fast"
status = "implemented"

[[case]]
id = "STL-TP-020"
requirements = ["STL-REQ-002", "STL-REQ-004", "STL-REQ-005"]
evidence = "test"
test = "crates/bunny-codec/tests/stl_oracle_tests.rs::stl_tp_020_seeded_files_match_the_independent_reader_bit_for_bit"
oracle = "3 000 seeded files (seed 0x0571127, up to 11 random facets each) and the 18 boundary patterns: the independent reader and parse_binary_stl agree bit for bit on every vertex or on the exact refusal; 1 269 meshes, 748 non-finite and 983 out-of-range refusals in the run that set the minimums of 1 000, 500 and 700."
tier = "fast"
status = "implemented"

[[case]]
id = "STL-TP-021"
requirements = ["STL-REQ-002", "STL-REQ-004", "STL-REQ-006"]
evidence = "test"
test = "crates/bunny-codec/tests/stl_oracle_tests.rs::stl_tp_021_random_and_mutated_bytes_never_panic_and_match_the_oracle"
oracle = "20 000 random, mutated, recounted and valid byte strings (seed 0x0571127 ^ 0xF0) never panic, every mesh obeys the length rule and the index rule, and every outcome equals the independent reader's; 3 801 meshes, 1 971 too short, 12 105 length mismatches, 855 non-finite and 1 268 out-of-range refusals in the run that set the minimums of 3 000, 1 500, 10 000, 600 and 1 000."
tier = "fast"
status = "implemented"

[[case]]
id = "STL-TP-022"
requirements = ["STL-REQ-004"]
evidence = "test"
test = "crates/bunny-codec/tests/stl_oracle_tests.rs::stl_tp_022_the_oracle_agrees_with_hand_derived_values"
oracle = "The oracle against fourteen hand-derived conversions and the tetrahedron."
tier = "fast"
status = "implemented"

[[case]]
id = "STL-TP-030"
requirements = ["STL-REQ-006"]
evidence = "test"
test = "crates/bunny-codec/tests/stl_allocation_tests.rs::stl_tp_030_binary_stl_reader_allocates_zero_times_after_warm_up"
oracle = "Zero allocation blocks on the measuring thread for parse and iteration after warm-up, and for a u32::MAX count (native only)."
tier = "fast"
status = "implemented"
```

## Mutation Check

Each rule was broken on purpose in `crates/bunny-codec/src/stl.rs` or `stl/layout.rs`, the
STL tests were run (`--lib` and the three `stl_*` test files, `--no-fail-fast`, under a
300-second timeout) and the source was restored. Every mutant fails at least one case:

| Mutant | Fails |
| --- | --- |
| length rule accepts trailing bytes (`!=` made `<`) | STL-TP-009, 017, 021 |
| expected length formed in wrapping `u32` | STL-TP-005, 010, 017, 021 |
| triangle limit removed | STL-TP-017 |
| triangle limit off by one (`>` made `>=`) | STL-TP-017 |
| count read big-endian | STL-TP-002, 003, 005 to 007, 009 to 014, 016, 020, 021, 030 |
| stored normal read as vertex 0 | STL-TP-002, 003, 006, 007, 012 to 014, 020, 021 |
| `y` and `z` swapped | STL-TP-002, 003, 007, 012 to 014, 020, 021 |
| non-finite reported as out of range | STL-TP-012, 020, 021 |
| saturating `from_f32` in place of `try_from_f32` | STL-TP-012, 013, 020, 021 |
| triangle index in errors off by one | STL-TP-012, 013, 020, 021 |
| vertex iterator wraps after two vertices | STL-TP-002, 007, 014, 016, 020, 021, 030 |
| triangle indices not multiplied by three | STL-TP-002, 020, 021 |
| only vertex 0 of each triangle validated at parse | STL-TP-012, 013, 020, 021 |
| header and count taken as 80 bytes | STL-TP-002, 003, 005 to 014, 016, 020, 021, 030 |

