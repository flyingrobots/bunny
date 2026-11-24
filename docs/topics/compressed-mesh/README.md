# Compressed Mesh Encoder

This chapter defines Bunny's current contract for encoding the Bunny
compressed mesh profile v1 in `bunny-codec`: `encode_compressed_mesh`,
`encode_compressed_mesh_into` and `encoded_compressed_mesh_len`, with the error
type `CompressedMeshEncodeError`. The profile is the one
`decode_compressed_mesh` reads, documented in
[`docs/goalposts/v0.4.0-gp3.md`](../../goalposts/v0.4.0-gp3.md#canonical-byte-profile);
neither the profile nor the decoder changes here.

## Status

This is a living topic chapter, not a proposal.

The encoder is implemented in `crates/bunny-codec/src/compressed_encode/` and
exported from `bunny_codec`. Evidence is listed in the
[test plan](test-plan.md).

## Input

A mesh is three values the caller already holds:

- `bounds: &FixedAabb3`, the quantization box, written as raw Q32.32 values;
- `vertices: &[QuantizedVertex]`, the `u16` coordinates, written as given;
- `triangles: IndexBufferLayout<'_>`, either `Width16(&[Triangle16])` or
  `Width32(&[Triangle32])`.

The encoder does not quantize, reorder, deduplicate or compress anything: it
writes the records it is given. The index width is the variant of
`triangles`; the encoder never picks one. A mesh with at most 65 536 vertices
can be written at either width, and the two streams differ, because the width
is part of what the decoder returns (`CompressedTriangle::Width16` or
`Width32`).

## Byte Layout

The stream is exactly the profile v1 layout, every integer little-endian:

| Offset | Size | Field | Written value |
| ---: | ---: | --- | --- |
| 0 | 8 | Magic | ASCII `BUNNYQZ!` |
| 8 | 1 | Version | `1` |
| 9 | 1 | Index width | `16` for `Width16`, `32` for `Width32` |
| 10 | 2 | Flags | `0` |
| 12 | 4 | Vertex count | `vertices.len()` |
| 16 | 4 | Triangle count | `triangles.len()` |
| 20 | 8 | Payload length | `6 * vertices + stride * triangles` |
| 28 | 48 | Bounds | `min.x`, `min.y`, `min.z`, `max.x`, `max.y`, `max.z` as raw `i64` |
| 76 | N | Vertex payload | `x`, `y`, `z` as `u16` per vertex, in order |
| 76 + N | M | Triangle payload | `v0`, `v1`, `v2` as `u16` (stride 6) or `u32` (stride 12) per triangle, in order |

## Round Trip And Canonical Bytes

For every mesh in the domain below, `decode_compressed_mesh` on the encoded
stream returns the same bounds, vertex count, triangle count and index width,
and `vertex(i)` and `triangle(i)` return the input records.

Every byte of the stream is fixed by the input: the constant fields are
constants, the counts and payload length are functions of the record counts and
the width, and the rest is the records in order. So a mesh has exactly one
stream, on every target. The converse also holds: the decoder accepts only
streams with these constants, an exact payload length and no trailing bytes,
so re-encoding the records decoded from any accepted stream gives that stream
back. The tests check this on the committed fixture, its width-32 form, two
constructed extremes and both widths of eight seeded meshes; the argument above
is what extends it to every accepted stream.

The committed fixture
`crates/bunny-codec/tests/fixtures/canonical_compressed_triangle.bunny.hex` is
the encoding of its documented records (unit bounds, vertices `(0, 0, 0)`,
`(65535, 0, 0)`, `(0, 65535, 0)`, triangle `(0, 1, 2)` at width 16), byte for
byte.

## Domain And Refusals

The encoder accepts exactly the meshes the decoder accepts, and refuses
everything else rather than write a stream the decoder would reject. The checks
run in this order, and the first failure is returned:

1. `InvalidVertexCount { count }`: fewer than 1 or more than 1 000 000
   vertices.
2. `InvalidTriangleCount { count }`: fewer than 1 or more than 1 000 000
   triangles.
3. `Width16VertexCapacity { vertex_count }`: more than 65 536 vertices with
   `Width16` triangles. 65 536 is accepted: index 65 535 addresses the last
   vertex.
4. `InvalidBounds`: `min` above `max` on any axis (`FixedAabb3::try_new`, the
   decoder's rule). Flat axes (`min == max`) and the whole raw range are
   accepted.
5. `IndexOutOfBounds { triangle, index }`: an index not below the vertex
   count; the first such triangle in order, and within it the first such
   corner (`v0`, `v1`, `v2`), with the index as a `u32`.
6. `OutputTooSmall { required, provided }`: for `encode_compressed_mesh_into`
   only, a buffer shorter than the stream.

`LengthOverflow` is returned where the stream's length does not fit `usize`;
see [Lengths And Targets](#lengths-and-targets).

Degenerate triangles, repeated triangles and vertices no triangle uses are
accepted and written as given, as the decoder accepts them.

`encoded_compressed_mesh_len` runs checks 1 to 3 only and returns the length;
a mesh it accepts can still fail checks 4 and 5.

## Output Buffer

`encode_compressed_mesh_into(bounds, vertices, triangles, output)` runs every
check before writing anything. On success it writes the stream to
`output[..len]` and returns `len`; bytes past `len` are untouched, and earlier
contents of `output` do not matter. On any error `output` is untouched.

## Allocation Contract

| Function | Heap allocations |
| --- | --- |
| `encoded_compressed_mesh_len` | none |
| `encode_compressed_mesh_into` | none |
| `encode_compressed_mesh` | one on success, a `Vec<u8>` whose length and capacity are the stream's length; none on an error, because every check runs first |

The encoder writes the stream from one iterator over the header fields and
records, so it needs no scratch and computes no offsets.

## Lengths And Targets

The stream is `76 + 6 * vertices + stride * triangles` bytes, so the largest
(1 000 000 vertices and 1 000 000 width-32 triangles) is 18 000 076 bytes,
which fits a 32-bit `usize` such as `wasm32`'s. Counts are converted to the
header's `u32` by checked conversion, and the lengths are computed by checked
`u64` arithmetic and converted to `usize` by checked conversion; nothing is
truncated or wrapped. A length that did not fit would be `LengthOverflow`;
that needs a `usize` narrower than 25 bits, which no target with `std` has.

There is no floating point, no map and no target-specific path, so every
target writes the same bytes for the same mesh.
