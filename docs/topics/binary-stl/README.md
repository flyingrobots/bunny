# Binary STL Triangle Meshes

This chapter defines the contract of `parse_binary_stl` in `bunny-codec`: the binary STL
import profile `bunny-binary-stl-triangle-mesh/1`, which reads a binary STL file into a
borrowed, fully validated triangle mesh with Q32.32 vertex positions. It answers
flyingrobots/bunny#127.

## Status

This is a living topic chapter, not a proposal. It is implemented in
`crates/bunny-codec/src/stl.rs` and the modules beside it, and exported as:

- `parse_binary_stl` and `BINARY_STL_PROFILE_ID` (`"bunny-binary-stl-triangle-mesh/1"`);
- `StlMesh` (`triangle_count`, `vertex_count`, `vertices`, `triangles`), `StlVertices`,
  `StlTriangles` and `StlVertex` (`x`, `y`, `z`, each `FixedQ32_32`);
- `StlError` and `BINARY_STL_MAX_TRIANGLES` (1 431 655 765, which is `u32::MAX / 3`).

The test plan is [`test-plan.md`](test-plan.md).

## Scope

Binary STL only. ASCII STL (`solid ... facet normal ... endsolid` text) is not part of the
profile and is not detected. In particular a file that begins with the bytes `solid` is
**not** thereby ASCII: many binary exporters write that word into the 80-byte header, so the
reader never looks at it. Every input is read as binary; an ASCII STL file is refused by the
length rule below in practice, because its bytes 80 to 83 are text and give a triangle count
that does not match its length. If the length happens to match, the text is read as binary
data like any other bytes.

The other readers are unchanged: this is a new function beside `parse_binary_ply`,
`parse_ascii_ply` and `parse_obj_text`, not a format dispatch in front of them.

## The Profile

A file of `bunny-binary-stl-triangle-mesh/1` is, byte for byte:

| Offset | Size | Content | How it is read |
| --- | --- | --- | --- |
| 0 | 80 | header | not interpreted and not checked: any 80 bytes are accepted |
| 80 | 4 | triangle count `n`, `u32` little-endian | the length rule |
| `84 + 50 i` | 12 | stored normal of triangle `i`, three binary32 | not read |
| `84 + 50 i + 12` | 36 | vertices 0, 1, 2 of triangle `i`, each `x`, `y`, `z` binary32 little-endian | converted to Q32.32 |
| `84 + 50 i + 48` | 2 | attribute byte count, `u16` | not read |

The checks run in this order, and the first that fails decides the error:

1. **Header and count present.** An input shorter than 84 bytes is
   `StlError::TooShort { length }`.
2. **Length.** The input length must be exactly `84 + 50 n`. The product is formed in `u64`
   and compared with the length as `u64`, so it cannot overflow on any target, including
   `wasm32` where `usize` is 32 bits: the largest value, for `n = u32::MAX`, is
   214 748 364 834. Any other length is
   `StlError::LengthMismatch { triangles: n, expected, actual }`. That covers a truncated
   file (shorter, anywhere, including mid-triangle), a count that claims more triangles
   than are present, a count that claims fewer, and trailing bytes: a file even one byte
   longer is refused, because silently ignoring data is how a corrupted file passes. The
   count never sizes memory; a count of `u32::MAX` over a short buffer is this error, found
   without allocating.
3. **Triangle limit.** With the length right, `n` above `BINARY_STL_MAX_TRIANGLES`
   (`u32::MAX / 3`) is `StlError::TooManyTriangles { triangles: n }`, because the vertex
   indices `3 i + k` must fit `u32` (see [Output](#output)). Only an input of at least
   71 582 788 384 bytes reaches this check, so on a 32-bit target it cannot occur.
4. **Coordinates,** in file order: triangle 0 vertex 0 `x`, `y`, `z`, then vertex 1, and so
   on. Each is converted as stated in [The Conversion](#the-conversion), and the first that
   fails is reported with its position (below).

Byte order is little-endian for every field. A count written big-endian is read
little-endian like any other, and then fails the length rule: one triangle written as
`00 00 00 01` reads as 16 777 216 triangles.

## The Conversion

Each vertex coordinate is converted by `bunny_num::FixedQ32_32::try_from_f32`, unchanged;
this reader has no float rule of its own. That conversion is documented in the
[fixed-point rounding chapter](../fixed-point-rounding/README.md) and the
[numeric constitution](../../NUMERIC_CONSTITUTION.md): the result is the raw value
`round_half_even(v * 2^32)` of the float's exact value `v`, decided from its sign, exponent
and significand with integer arithmetic, so the error is at most half a raw unit
(`2^-33`) and an exact tie goes to the even raw value. So `2^-33` becomes 0, `3 * 2^-33`
becomes 2, `5 * 2^-33` becomes 2, `-3 * 2^-33` becomes -2, and every subnormal and both
zeros become 0.

Its refusals become typed errors that name the coordinate:

| `try_from_f32` returns | The reader returns |
| --- | --- |
| `FloatConversionError::NonFinite` (any NaN, `+inf`, `-inf`) | `StlError::NonFiniteVertex { triangle, vertex, axis }` |
| `FloatConversionError::OutOfRange` (finite, but the rounded raw value is outside `i64`) | `StlError::VertexOutOfRange { triangle, vertex, axis }` |

`triangle` is the 0-based triangle index in the file, `vertex` is 0, 1 or 2 in stored order,
and `axis` is 0, 1 or 2 for `x`, `y`, `z`. In binary32 the in-range values run from
`-2^31` (raw `i64::MIN`, accepted) to `2^31 - 128` (the largest binary32 below `2^31`,
accepted). The next values out, `2^31` and `-2^31 - 256`, are refused, as is every larger
magnitude up to `f32::MAX`.

## Winding, Normals And Attributes

One rule, with no exceptions:

- **The stored vertex order is authoritative and is kept exactly.** Triangle `i` of the file
  is triangle `i` of the mesh, with its vertices in file order. The intended convention is
  counter-clockwise seen from outside, but the reader neither checks nor repairs it.
- **The stored normal is ignored.** It is never read: not validated against the vertices,
  not used to reorder them, not returned. A normal that is wrong, zero, unnormalised, NaN or
  infinite is accepted like any other 12 bytes. A caller that needs a normal computes one
  from the vertices.
- **The attribute byte count is ignored.** It is not validated and not interpreted; some
  exporters store a colour there, and this profile does not read it.
- **Degenerate triangles are kept.** A triangle with repeated or collinear vertices is
  returned in its place; nothing is dropped. Topology validation is a separate step:
  `bunny_mesh::validate_topology` ([mesh topology](../mesh-topology/README.md)), which takes a
  quantized mesh; this reader does not call it. It matches edges by vertex index, and this
  reader does not weld, so no two triangles share an index: on this output every edge of
  every non-degenerate triangle is a `BoundaryEdge`, a closed solid included. Weld equal
  positions into shared vertices first for the edge findings to mean anything.
- **Zero triangles is a mesh.** An 84-byte file with count 0 is accepted as an empty mesh, as
  the PLY readers accept a zero-count mesh.

## Output

STL has no index buffer: every triangle carries its own three vertices. The reader returns
the indexed form the other readers return, without welding:

- `vertex_count` is `3 n`, and vertex `3 i + k` is vertex `k` of triangle `i`.
- Triangle `i` is `Triangle32 { v0: 3 i, v1: 3 i + 1, v2: 3 i + 2 }`.

This is exact and order-preserving. Welding equal positions into shared vertices is not
done: it is a separate transformation with its own determinism questions (which duplicate
wins, and whether equality is decided on the float or the fixed value).

`parse_binary_stl` follows the shape of `parse_ascii_ply`: it validates every coordinate of
every triangle up front, then returns `StlMesh`, a borrowed view of the input that re-reads
the bytes when iterated. `vertices` yields `Result<StlVertex, StlError>` and `triangles`
yields `Result<Triangle32, StlError>`, as the ASCII PLY iterators do, and every item of a
mesh returned by `parse_binary_stl` is `Ok`. The view and its iterators are `Copy` or
`Clone`, restart from the start, and allocate nothing; the reader allocates nothing either.
The view holds the triangle records only, not the header.

## Errors

`StlError` is `#[non_exhaustive]`, `Copy` and `Eq`, implements `core::error::Error` (no
source) and displays as:

| Variant | `Display` |
| --- | --- |
| `TooShort { length }` | `STL input is <length> bytes, shorter than the 84-byte header and triangle count` |
| `LengthMismatch { triangles, expected, actual }` | `STL declares <triangles> triangles, which needs exactly <expected> bytes; the input is <actual> bytes` |
| `TooManyTriangles { triangles }` | `STL declares <triangles> triangles; at most 1431655765 have vertex indices that fit u32` |
| `NonFiniteVertex { triangle, vertex, axis }` | `STL triangle <triangle> vertex <vertex> coordinate <x\|y\|z> is not finite` |
| `VertexOutOfRange { triangle, vertex, axis }` | `STL triangle <triangle> vertex <vertex> coordinate <x\|y\|z> is outside the Q32.32 range` |

## Fixtures

No STL file is checked in and no third-party STL file is used. Every fixture is built by the
tests from integer bit patterns (`crates/bunny-codec/tests/stl_support/mod.rs`); the
construction is recorded in `crates/bunny-codec/tests/fixtures/README.md`. Nothing is added
to the package `exclude` list, because every STL test reads only bytes it builds.

## Verification

See [`test-plan.md`](test-plan.md). Constructed cases cover each accepted shape and each
refusal, with every coordinate position refused for each non-finite value and for the
range; the counts at and around the `u32` and vertex-index boundaries are refused by the
length rule over a short buffer. An independent reader in the tests, which shares no code
with `bunny-codec` or `bunny-num`, is compared bit for bit with `parse_binary_stl` on 3 000
seeded generated files and on 20 000 random and mutated byte strings. Every test except the
native allocation witness is a `wasm_bindgen_test`, so it runs natively and on `wasm32`.
