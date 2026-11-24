# Mesh Topology Test Plan

This document defines how Bunny proves [`bunny-mesh-topology/1`](README.md):
`validate_topology` and its records, order, storage contract and refusals. It answers
flyingrobots/bunny#122.

## Scope

- Index-degenerate and geometrically degenerate triangles, exact and flipped duplicate
  faces, and boundary, winding-inconsistent and non-manifold edges in `bunny-mesh`, with
  their severities, the record order, the output and scratch contracts and the refusals.

Out of scope: everything the chapter lists under [Not Checked](README.md#not-checked).

## Requirements

The table is the human-readable view. The fenced TOML block immediately after it is the
contract graph consumed by `cargo run --locked -p xtask -- topic-docs`.

| ID | Requirement | Current Source |
| --- | --- | --- |
| MTOPO-REQ-001 | A triangle with two or three equal corner indices is one `IndexDegenerateTriangle` (error) and takes part in no other check. | `README.md#degenerate-triangles` |
| MTOPO-REQ-002 | A distinct-index triangle is `GeometricallyDegenerateTriangle` (warning) exactly when the cross product of its edges from corner 0 is zero on the `u16` coordinates, computed exactly in `i64` with no tolerance; collinear and coincident positions both count. | `README.md#degenerate-triangles` |
| MTOPO-REQ-003 | Each later distinct-index face with the same index set as the lowest-indexed one, `first`, is one `DuplicateFace` (error) when its cyclic order (canonical key: rotated smallest first) equals `first`'s, and one `FlippedDuplicateFace` (warning) otherwise. | `README.md#duplicate-faces` |
| MTOPO-REQ-004 | Each undirected edge of the distinct-index faces is a `BoundaryEdge` with one use, a `WindingInconsistentEdge` with two uses in one direction, a `NonManifoldEdge` with three or more, and nothing with two uses in opposite directions; all warnings. | `README.md#edges` |
| MTOPO-REQ-005 | Records come in the derived `Ord` of `TopologyFinding` (kind, then fields), strictly increasing, equal for `Width16` and `Width32`; the first `findings.len()` are written, the report counts every one per kind, `is_complete` is false exactly when the slice was short, and entries past `written` are untouched. | `README.md#record-order` |
| MTOPO-REQ-006 | The kernel does not allocate: it needs `face_scratch_len(n) = n` face slots and `edge_scratch_len(n) = 3 n` edge slots, accepts longer ones, and its output does not depend on the scratch contents on entry. | `README.md#storage-contract` |
| MTOPO-REQ-007 | Invalid input is refused, writing nothing, in the order `IndexOutOfRange` (the existing `IndexBufferLayout::is_valid`), `TooManyTriangles`, `FaceScratchTooSmall`, `EdgeScratchTooSmall`; the empty mesh is valid with no findings. | `README.md#refusals` |

```toml
[[requirement]]
id = "MTOPO-REQ-001"
summary = "A triangle with a repeated corner index is one IndexDegenerateTriangle error and takes part in no other check."
status = "active"

[[requirement]]
id = "MTOPO-REQ-002"
summary = "A distinct-index triangle is GeometricallyDegenerateTriangle exactly when its edge cross product is zero in exact i64 on the u16 coordinates; no tolerance."
status = "active"

[[requirement]]
id = "MTOPO-REQ-003"
summary = "Each later face with the same index set as the lowest-indexed one is a DuplicateFace error in the same cyclic order, or a FlippedDuplicateFace warning in the opposite one."
status = "active"

[[requirement]]
id = "MTOPO-REQ-004"
summary = "Each undirected edge is a boundary (one use), winding-inconsistent (two, same direction) or non-manifold (three or more) warning; two opposite uses are manifold."
status = "active"

[[requirement]]
id = "MTOPO-REQ-005"
summary = "Records follow the derived Ord, strictly increasing and equal for both index widths; a short output keeps the ordered prefix and the report counts every finding."
status = "active"

[[requirement]]
id = "MTOPO-REQ-006"
summary = "No allocation: n face slots and 3n edge slots of caller scratch, longer accepted, output independent of the scratch contents on entry."
status = "active"

[[requirement]]
id = "MTOPO-REQ-007"
summary = "Refusals write nothing and come in the order IndexOutOfRange, TooManyTriangles, FaceScratchTooSmall, EdgeScratchTooSmall; the empty mesh is valid."
status = "active"
```

## Oracles

Each test file writes its own brute-force oracle and shares no code with the kernel. It
allocates freely: `BTreeSet` decides index degeneracy, a `BTreeMap` from the sorted index
set groups faces, and the cyclic class of a face is the parity of the permutation that
sorts it (an inversion count), not a rotation. A `BTreeMap` from the vertex pair groups the
directed edges. Collinearity is decided in `i128` with edge vectors taken from the second
corner, not the first. The oracle's records are sorted with the derived `Ord`. The record
order is also pinned by hand in MTOPO-TP-010. The Stanford counts were also reproduced by a
separate Python count over the raw PLY face list, posted on the pull request and not kept
in the tree.

## Fixtures

- Hand-built meshes in `crates/bunny-mesh/tests/topology_tests.rs`: a closed, outward-wound
  tetrahedron with no findings, and open, index-degenerate, collinear, duplicate, flipped,
  reversed-face and edge-sharing variants, each showing one finding kind. The exception is
  an exact duplicate, whose implied same-direction edge uses are listed with it. There is
  also one mixed mesh with five kinds.
- `crates/bunny-codec/tests/fixtures/stanford-bunny-ascii/bun_zipper_res3.ply` (1889
  vertices, 3851 triangles), read with `parse_ascii_ply` and quantized over its exact bounds
  with `quantize_vertex`. Its findings: 0 index-degenerate, 0 geometrically degenerate, 10
  exact duplicates, 73 flipped duplicates, 60 boundary edges, 0 winding-inconsistent edges
  and 141 non-manifold edges. The test lives in `bunny-codec` because the asset is inside
  that crate, and it is excluded from the package with the asset.
- Seeded meshes: MTOPO-TP-013 draws 2 000 meshes from seed `0xB002_0122_0000_0001` with a
  64-bit LCG. Each has 3 to 8 vertices whose coordinates are drawn from `0..3` (half the
  time), `{0, 65535}` or uniform `u16`, and which repeat the previous vertex one time in
  five. Each has 0 to 16 faces, drawn as an earlier face rotated, an earlier face reversed,
  a forced repeated index, or three uniform indices. The test asserts every kind occurs at
  least 100 times. MTOPO-TP-014 draws 300 more meshes from seed
  `0xB002_0122_0000_0001 ^ 0x5C7A_7C11`. Every failure message names the seed and the case.

## Test Cases

| ID | Category | Requirements | Oracle | Test |
| --- | --- | --- | --- | --- |
| MTOPO-TP-001 | Fixture | MTOPO-REQ-004, MTOPO-REQ-005 | The closed outward tetrahedron and the empty mesh give no records; contract identity pinned. | `crates/bunny-mesh/tests/topology_tests.rs::mtopo_tp_001_closed_tetrahedron_has_no_findings` |
| MTOPO-TP-002 | Fixture | MTOPO-REQ-004 | The tetrahedron without its fourth face gives exactly its three boundary edges. | `crates/bunny-mesh/tests/topology_tests.rs::mtopo_tp_002_open_mesh_has_boundary_edges_only` |
| MTOPO-TP-003 | Fixture | MTOPO-REQ-001 | `(0,0,1)`, `(0,1,1)`, `(1,0,1)`, `(2,2,2)` on collinear positions give four `IndexDegenerateTriangle` records and nothing else. | `crates/bunny-mesh/tests/topology_tests.rs::mtopo_tp_003_repeated_indices_are_index_degenerate_only` |
| MTOPO-TP-004 | Fixture | MTOPO-REQ-002 | A closed tetrahedron with one collinear face gives exactly that one geometric record. | `crates/bunny-mesh/tests/topology_tests.rs::mtopo_tp_004_collinear_face_in_a_closed_mesh_is_geometrically_degenerate_only` |
| MTOPO-TP-005 | Boundary | MTOPO-REQ-002 | On and one unit either side of collinearity along the full diagonal (cross `0`, `+-65535`), cross `+-1`, coincident corners, every cross component exactly `-65535^2`, and a 3D line at `z = 65535` with each coordinate one unit off. | `crates/bunny-mesh/tests/topology_tests.rs::mtopo_tp_005_the_zero_area_test_is_exact_at_the_coordinate_extremes` |
| MTOPO-TP-006 | Fixture | MTOPO-REQ-003, MTOPO-REQ-004 | A face and its rotation give `DuplicateFace` and the three implied winding-inconsistent edges. | `crates/bunny-mesh/tests/topology_tests.rs::mtopo_tp_006_exact_duplicates_compare_rotated_keys` |
| MTOPO-TP-007 | Fixture | MTOPO-REQ-003 | A face and its reverse give one `FlippedDuplicateFace` only; four faces of one set are each reported against the lowest-indexed. | `crates/bunny-mesh/tests/topology_tests.rs::mtopo_tp_007_flipped_duplicates_are_reported_against_the_first_face` |
| MTOPO-TP-008 | Fixture | MTOPO-REQ-004 | The tetrahedron with one face reversed gives exactly its three winding-inconsistent edges. | `crates/bunny-mesh/tests/topology_tests.rs::mtopo_tp_008_one_reversed_face_gives_winding_inconsistent_edges_only` |
| MTOPO-TP-009 | Fixture | MTOPO-REQ-004 | Two closed tetrahedra sharing one edge give one `NonManifoldEdge` with four uses. | `crates/bunny-mesh/tests/topology_tests.rs::mtopo_tp_009_two_tetrahedra_sharing_an_edge_give_one_non_manifold_edge` |
| MTOPO-TP-010 | Hand-derived order | MTOPO-REQ-001, MTOPO-REQ-003, MTOPO-REQ-005 | A mixed mesh listed out of record order gives the hand-written ten records, strictly increasing; the severity table and error and warning counts are pinned. | `crates/bunny-mesh/tests/topology_tests.rs::mtopo_tp_010_records_come_in_kind_then_field_order_with_fixed_severities` |
| MTOPO-TP-011 | Boundary | MTOPO-REQ-005 | Output lengths 0 to 12 for ten records: the written prefix is the ordered prefix, `total` is 10, `is_complete` flips at 10, and slots past `written` keep their fill. | `crates/bunny-mesh/tests/topology_tests.rs::mtopo_tp_011_a_short_output_buffer_keeps_the_ordered_prefix_and_reports_the_total` |
| MTOPO-TP-012 | Refusal | MTOPO-REQ-006, MTOPO-REQ-007 | Out-of-range indices (both widths) refused before scratch sizes; face before edge scratch; one slot short of each refused, the exact sizes and longer accepted; a refusal writes nothing; `edge_scratch_len(usize::MAX)` is `None`. | `crates/bunny-mesh/tests/topology_tests.rs::mtopo_tp_012_invalid_input_is_refused_in_a_fixed_order` |
| MTOPO-TP-013 | Seeded oracle | MTOPO-REQ-001, MTOPO-REQ-002, MTOPO-REQ-003, MTOPO-REQ-004, MTOPO-REQ-005 | 2 000 seeded meshes with duplicates, flips and degenerate faces: kernel records equal the oracle's bit for bit, for both widths, with per-kind counts; every kind at least 100 times. | `crates/bunny-mesh/tests/topology_tests.rs::mtopo_tp_013_seeded_meshes_match_the_oracle_bit_for_bit` |
| MTOPO-TP-014 | Property | MTOPO-REQ-006 | 300 seeded meshes run on scratch left dirty by another mesh and then reversed and rotated give the same records as fresh scratch. | `crates/bunny-mesh/tests/topology_tests.rs::mtopo_tp_014_output_does_not_depend_on_scratch_contents_on_entry` |
| MTOPO-TP-015 | Real asset | MTOPO-REQ-001, MTOPO-REQ-002, MTOPO-REQ-003, MTOPO-REQ-004 | The Stanford bunny, quantized over its exact bounds: records equal the oracle's, and the counts per kind are pinned (0, 0, 10, 73, 60, 0, 141), matching a separate Python count. | `crates/bunny-codec/tests/ply_ascii_stanford_topology_tests.rs::mtopo_tp_015_stanford_bunny_findings_match_the_oracle_and_pinned_counts` |
| MTOPO-TP-016 | Mutation | MTOPO-REQ-001, MTOPO-REQ-002, MTOPO-REQ-003, MTOPO-REQ-004, MTOPO-REQ-005, MTOPO-REQ-006, MTOPO-REQ-007 | Each rule broken on purpose fails at least one case above. | [Mutations](#mutations) |

```toml
[[case]]
id = "MTOPO-TP-001"
requirements = ["MTOPO-REQ-004", "MTOPO-REQ-005"]
evidence = "test"
test = "crates/bunny-mesh/tests/topology_tests.rs::mtopo_tp_001_closed_tetrahedron_has_no_findings"
oracle = "Hand-checked closed outward tetrahedron and the empty mesh; brute-force oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "MTOPO-TP-002"
requirements = ["MTOPO-REQ-004"]
evidence = "test"
test = "crates/bunny-mesh/tests/topology_tests.rs::mtopo_tp_002_open_mesh_has_boundary_edges_only"
oracle = "Hand-listed boundary edges; brute-force oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "MTOPO-TP-003"
requirements = ["MTOPO-REQ-001"]
evidence = "test"
test = "crates/bunny-mesh/tests/topology_tests.rs::mtopo_tp_003_repeated_indices_are_index_degenerate_only"
oracle = "Hand-listed index-degenerate records; brute-force oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "MTOPO-TP-004"
requirements = ["MTOPO-REQ-002"]
evidence = "test"
test = "crates/bunny-mesh/tests/topology_tests.rs::mtopo_tp_004_collinear_face_in_a_closed_mesh_is_geometrically_degenerate_only"
oracle = "Hand-built collinear face in a closed mesh; brute-force oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "MTOPO-TP-005"
requirements = ["MTOPO-REQ-002"]
evidence = "test"
test = "crates/bunny-mesh/tests/topology_tests.rs::mtopo_tp_005_the_zero_area_test_is_exact_at_the_coordinate_extremes"
oracle = "Hand-derived cross products on and one unit either side of collinearity at the u16 extremes; i128 oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "MTOPO-TP-006"
requirements = ["MTOPO-REQ-003", "MTOPO-REQ-004"]
evidence = "test"
test = "crates/bunny-mesh/tests/topology_tests.rs::mtopo_tp_006_exact_duplicates_compare_rotated_keys"
oracle = "Hand-listed duplicate and implied edge records; brute-force oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "MTOPO-TP-007"
requirements = ["MTOPO-REQ-003"]
evidence = "test"
test = "crates/bunny-mesh/tests/topology_tests.rs::mtopo_tp_007_flipped_duplicates_are_reported_against_the_first_face"
oracle = "Hand-listed duplicate records against the lowest-indexed face; brute-force oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "MTOPO-TP-008"
requirements = ["MTOPO-REQ-004"]
evidence = "test"
test = "crates/bunny-mesh/tests/topology_tests.rs::mtopo_tp_008_one_reversed_face_gives_winding_inconsistent_edges_only"
oracle = "Hand-listed winding-inconsistent edges; brute-force oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "MTOPO-TP-009"
requirements = ["MTOPO-REQ-004"]
evidence = "test"
test = "crates/bunny-mesh/tests/topology_tests.rs::mtopo_tp_009_two_tetrahedra_sharing_an_edge_give_one_non_manifold_edge"
oracle = "Hand-listed non-manifold edge; brute-force oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "MTOPO-TP-010"
requirements = ["MTOPO-REQ-001", "MTOPO-REQ-003", "MTOPO-REQ-005"]
evidence = "test"
test = "crates/bunny-mesh/tests/topology_tests.rs::mtopo_tp_010_records_come_in_kind_then_field_order_with_fixed_severities"
oracle = "Hand-written record sequence and severity table; brute-force oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "MTOPO-TP-011"
requirements = ["MTOPO-REQ-005"]
evidence = "test"
test = "crates/bunny-mesh/tests/topology_tests.rs::mtopo_tp_011_a_short_output_buffer_keeps_the_ordered_prefix_and_reports_the_total"
oracle = "Prefixes of the hand-written record sequence at every output length."
tier = "fast"
status = "implemented"

[[case]]
id = "MTOPO-TP-012"
requirements = ["MTOPO-REQ-006", "MTOPO-REQ-007"]
evidence = "test"
test = "crates/bunny-mesh/tests/topology_tests.rs::mtopo_tp_012_invalid_input_is_refused_in_a_fixed_order"
oracle = "Hand-derived refusals at and one slot below each documented size."
tier = "fast"
status = "implemented"

[[case]]
id = "MTOPO-TP-013"
requirements = ["MTOPO-REQ-001", "MTOPO-REQ-002", "MTOPO-REQ-003", "MTOPO-REQ-004", "MTOPO-REQ-005"]
evidence = "property"
test = "crates/bunny-mesh/tests/topology_tests.rs::mtopo_tp_013_seeded_meshes_match_the_oracle_bit_for_bit"
oracle = "Independent BTreeMap, permutation-parity and i128 oracle over 2 000 seeded meshes."
tier = "fast"
status = "implemented"

[[case]]
id = "MTOPO-TP-014"
requirements = ["MTOPO-REQ-006"]
evidence = "property"
test = "crates/bunny-mesh/tests/topology_tests.rs::mtopo_tp_014_output_does_not_depend_on_scratch_contents_on_entry"
oracle = "Bit equality with a fresh-scratch run over 300 seeded meshes on dirty, permuted scratch."
tier = "fast"
status = "implemented"

[[case]]
id = "MTOPO-TP-015"
requirements = ["MTOPO-REQ-001", "MTOPO-REQ-002", "MTOPO-REQ-003", "MTOPO-REQ-004"]
evidence = "test"
test = "crates/bunny-codec/tests/ply_ascii_stanford_topology_tests.rs::mtopo_tp_015_stanford_bunny_findings_match_the_oracle_and_pinned_counts"
oracle = "Independent brute-force oracle on the committed Stanford asset; counts matched by a separate Python count."
tier = "fast"
status = "implemented"

[[case]]
id = "MTOPO-TP-016"
requirements = ["MTOPO-REQ-001", "MTOPO-REQ-002", "MTOPO-REQ-003", "MTOPO-REQ-004", "MTOPO-REQ-005", "MTOPO-REQ-006", "MTOPO-REQ-007"]
evidence = "manual-audit"
artifact = "docs/topics/mesh-topology/test-plan.md#mutations"
oracle = "Each rule broken on purpose fails at least one case."
tier = "manual"
status = "implemented"
```

## Mutations

Each mutation was applied to the kernel. Then `topology_tests` and
`ply_ascii_stanford_topology_tests` were run and the kernel restored. Each mutation fails
the named cases, except M03, which is equivalent (see the note).

| ID | Mutation | Fails |
| --- | --- | --- |
| M01 | The repeated-index test drops `a == c` | MTOPO-TP-003, -013 |
| M02 | The zero-area test drops the third cross component | MTOPO-TP-001, -002, -005 to -011, -013 |
| M03 | Each cross product compared after truncation to `i32` (wrapping) | none: equivalent, see the note |
| M03b | Positions, differences and products in `i32` (overflowing) | MTOPO-TP-005, -013, -014 |
| M04 | The geometric pass also tests index-degenerate triangles | MTOPO-TP-003, -010, -011, -013 |
| M05 | The canonical key is not rotated | MTOPO-TP-006, -007, -010, -011, -013, -015 |
| M06 | Exact and flipped duplicates swapped | MTOPO-TP-006, -007, -010, -011, -013, -015 |
| M07 | Duplicate records emitted in key order, not triangle order | MTOPO-TP-013, -015 |
| M08 | A duplicate is reported against itself, not the first face | MTOPO-TP-006, -007, -010, -011, -013, -015 |
| M09 | Two uses in opposite directions called winding-inconsistent | MTOPO-TP-001, -002, -004, -006 to -010, -013, -015 |
| M10 | Boundary edges not reported | MTOPO-TP-002, -005, -010, -011, -013, -015 |
| M11 | Non-manifold edges emitted before boundary edges | MTOPO-TP-010, -011, -013, -015 |
| M12 | `uses` one short | MTOPO-TP-007, -009, -010, -011, -013, -015 |
| M13 | `written` counts records that did not fit | MTOPO-TP-011 |
| M14 | `DuplicateFace` severity made a warning | MTOPO-TP-010 |
| M15 | The face pass keeps the mark a slot held on entry | MTOPO-TP-014 |
| M16 | The edge scratch checked before the face scratch | MTOPO-TP-012 |
| M17 | The index range check skipped | MTOPO-TP-012 |

Note on M03: a wrapped `i32` comparison of two products is equal exactly when their
difference is a multiple of `2^32`. That difference is a cross component, whose magnitude
is at most `65535^2 < 2^32`, so wrapping gives the same answer. Real 32-bit arithmetic
overflows, which M03b shows. The kernel uses `i64`, which needs neither argument.
