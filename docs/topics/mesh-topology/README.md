# Mesh Topology

`bunny_mesh::validate_topology` checks a quantized triangle mesh (`&[QuantizedVertex]` and an
`IndexBufferLayout`) for degenerate triangles, duplicate faces and edges that are not
manifold or not consistently wound. It returns its findings as `TopologyFinding` records in
one total order. It does not allocate: the caller provides the scratch storage and the
output slice. The contract is named `bunny-mesh-topology/1` (`MESH_TOPOLOGY_CONTRACT_ID`).

The verification design is [`test-plan.md`](test-plan.md).

## Findings

Vertex and triangle indices in records are `u32` for both index widths, so a `Width16` and
a `Width32` buffer with the same indices give equal records. "Distinct-index triangle" below
means a triangle whose three corner indices are pairwise different.

| Kind | Definition | Severity | Record fields |
| --- | --- | --- | --- |
| `IndexDegenerateTriangle` | Two or three corner indices are equal. | error | `triangle` |
| `GeometricallyDegenerateTriangle` | A distinct-index triangle whose three positions are collinear or coincident. | warning | `triangle` |
| `DuplicateFace` | A distinct-index triangle with the same three indices, in the same cyclic order, as a lower-indexed one. | error | `triangle`, `first` |
| `FlippedDuplicateFace` | The same, in the opposite cyclic order. | warning | `triangle`, `first` |
| `BoundaryEdge` | An undirected edge used by exactly one distinct-index triangle. | warning | `edge`, `triangle` |
| `WindingInconsistentEdge` | An undirected edge used by exactly two, both in the same direction. | warning | `edge`, `triangles` |
| `NonManifoldEdge` | An undirected edge used by three or more. | warning | `edge`, `uses`, `first_triangle` |

An edge used by exactly two distinct-index triangles in opposite directions is manifold
and is reported by nothing. A closed, consistently wound, manifold mesh has no edge finding.

### Degenerate Triangles

Index degeneracy is combinatorial and exact. An index-degenerate triangle takes part in no
other check: it is not tested for area, compared for duplicates, or counted on any edge.

Geometric degeneracy is decided on the quantized `u16` coordinates with no rounding, no
floating point and no tolerance. With `u = p1 - p0` and `v = p2 - p0`, the triangle is
degenerate when the cross product `u x v` is exactly zero, so both collinear and coincident
positions count. Each component of `u` and `v` is in `[-65535, 65535]` (17 signed bits).
Each product of two components is at most `65535^2 = 4_294_836_225 < 2^32` in magnitude.
Each cross component is twice the signed area of the triangle projected onto a coordinate
plane, which lies in a `65535` square, so it is at most `65535^2` too. The kernel computes
in `i64`. A test reaches `65535^2` exactly in each component, and sits on and one unit
either side of collinearity along the full diagonal and along a line through the interior
of the cube.

Two vertices at different indices with the same position make every triangle that uses
both geometrically degenerate. A mesh-wide search for duplicate positions (vertices no
triangle shares) is not done.

### Duplicate Faces

A face's canonical key is its indices rotated so that the smallest comes first, keeping
the cyclic order. Two distinct-index faces are exact duplicates when their keys are equal,
and flipped duplicates when one key is the other's cyclic order reversed. With three
distinct indices, every pair of faces with the same index set is one or the other, so
"same set, any order" is the union of the two kinds and has no kind of its own.

Within one index set, the lowest-indexed face is `first`. Every later face gives one record
against it: `DuplicateFace` when its cyclic order equals `first`'s, `FlippedDuplicateFace`
otherwise. A set used `k` times gives `k - 1` records, never a record per pair.

### Edges

Every distinct-index triangle `(a, b, c)` contributes the directed edges `a->b`, `b->c` and
`c->a`, duplicates included. `edge` is the vertex pair smallest first. `triangles` and
`first_triangle` are the lowest triangle indices using the edge. An exact duplicate
therefore also makes each of its edges winding-inconsistent (with two uses) or non-manifold
(with more), and a flipped duplicate of a lone face leaves its edges manifold: a two-sided
sheet. Which winding counts as front ([coordinate law](../coordinate-law/README.md))
does not matter here. Only whether two uses agree matters.

### Severity

An error is a defect of the index buffer, whatever surface it means to describe: a face
without three distinct corners, or the same face listed twice. Every other kind is a
warning: a property a valid mesh may have on purpose, such as zero area after
quantization, a two-sided sheet, an open surface, a reversed face or a junction of three
or more faces. Findings never make `validate_topology` fail. `TopologyReport::error_count`
and `warning_count` add them up.

## Record Order

The derived `Ord` of `TopologyFinding` is the record order: kind first, in the table's
order, then the fields in declaration order. Face kinds therefore go by triangle index and
edge kinds by edge. No two records of one kind compare equal, so the order is total, and
records come out strictly increasing. They depend only on the vertex positions and indices.
Pointer values, hashing, platform and scratch contents play no part, so the same mesh gives
equal records on every platform and run.

`validate_topology` writes the first `findings.len()` records in that order and counts all
of them. `TopologyReport::written` is the number written, `total` the number found and
`count(kind)` the number per kind. `is_complete` is `false` exactly when the slice was too
short, so a short buffer is never silent. Call again with `total` slots to get every
record. Entries past `written` are not touched. There are at most `5 n` records for `n`
triangles (one degeneracy and one duplicate record per triangle, one record per undirected
edge), so `5 n` slots always suffice.

## Storage Contract

`bunny-mesh` library code uses only slices, so this kernel takes caller storage:

| Scratch | Slots needed for `n` triangles | Function |
| --- | --- | --- |
| `&mut [FaceSlot]` | `n` | `face_scratch_len(n)` |
| `&mut [EdgeSlot]` | `3 n` | `edge_scratch_len(n)`, `None` when `3 n` overflows `usize` |

Longer slices are fine, and only the prefix is used. `FaceSlot::EMPTY` and `EdgeSlot::EMPTY`
fill new arrays. The contents on entry are overwritten before they are read, so they do not
change the result. A test reruns the validator on scratch left dirty by another mesh and
then permuted, and compares the output with a fresh run.

The face pass sorts the distinct-index faces' keys in place by (index set, triangle),
groups them, then sorts again by triangle: two `O(n log n)` sorts. The edge pass sorts the
`3 m` directed edges of the `m` distinct-index faces by (smaller vertex, larger vertex,
triangle) and scans the groups once per edge kind. Both use `sort_unstable`, which is in
`core` and does not allocate. Their keys include the triangle index and are unique, so the
sorted order is unique too.

## Refusals

`validate_topology` returns `Err(TopologyError)`, writing nothing, checked in this order:

1. `IndexOutOfRange`: an index is not below `vertices.len()`. This is the existing
   `IndexBufferLayout::is_valid`, called unchanged.
2. `TooManyTriangles`: more than `u32::MAX` triangles (a triangle index would not fit a
   record), or `3 n` overflows `usize`.
3. `FaceScratchTooSmall { required, provided }`: fewer than `n` face slots.
4. `EdgeScratchTooSmall { required, provided }`: fewer than `3 n` edge slots.

The index layouts are slices of `Triangle16` or `Triangle32`, so an index count that is
not a multiple of three cannot be represented and needs no refusal. The empty mesh is valid
and has no findings.

## Not Checked

- Mesh-wide duplicate vertex positions, and vertices no triangle uses.
- Connected components, holes as loops, genus and orientability beyond single edges.
- Self-intersection and other geometric overlap between non-adjacent faces.
- Non-manifold vertices (a vertex whose faces form more than one fan) when every edge is
  manifold.
- Which winding is front. Consistency is checked, the direction is not.
