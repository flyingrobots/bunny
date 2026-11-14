# bunny-codec

`bunny-codec` contains deterministic mesh ingress codecs for Bunny.

## Public Profiles

| API | Profile | Allocation contract |
| --- | --- | --- |
| `parse_binary_ply` | Canonical binary little-endian PLY triangle mesh | Borrowed view, zero allocations after warm-up |
| `parse_ascii_ply` | ASCII PLY triangle mesh, `bunny-ascii-ply-triangle-mesh/1`, with exact decimal to Q32.32 positions | Borrowed view and iterators, zero allocations after warm-up |
| `parse_obj_text` | Supported OBJ text triangle mesh | Borrowed source view, zero allocations after warm-up |
| `decode_compressed_mesh` | Bunny-native compressed mesh profile v1 | Borrowed byte-section view, zero allocations after warm-up |

The compressed profile decodes into `bunny-mesh` record concepts through checked
accessors:

* `CompressedMesh::vertex` returns `QuantizedVertex`.
* `CompressedMesh::triangle` returns `CompressedTriangle::Width16` or
  `CompressedTriangle::Width32`.
* The view exposes borrowed raw vertex and triangle payload bytes for callers
  that need deterministic hashing or external validation.

The decoder does not reinterpret arbitrary input bytes as typed slices because
that would require `unsafe`. The repository forbids unsafe code.

## ASCII PLY

`parse_ascii_ply` reads `format ascii 1.0` PLY triangle meshes. Vertex positions
come back as `FixedQ32_32`, converted from the decimal text exactly, by integer
arithmetic only, as `round(value * 2^32)` to nearest with ties to even
(`bunny-decimal-q32-32/1`, also available as `parse_decimal_q32_32`). Vertex
properties other than `x`, `y`, `z` are skipped by their declaration, and errors
carry a 1-based line number. The profile, the line, whitespace and comment rules
and the error contract are in
[`docs/topics/ascii-ply`](../../docs/topics/ascii-ply/README.md). The binary reader
is unchanged.

## Verification

The compressed decoder is covered by:

* A checked-in canonical fixture at
  `tests/fixtures/canonical_compressed_triangle.bunny.hex`.
* Width-16 and width-32 accepted-path tests.
* A malformed-input corpus covering every public compressed decoder error.
* Native allocation witnesses.
* `wasm_bindgen_test` coverage for the public decoder corpus.
* The ASCII PLY reader is covered by hand-derived conversion vectors, an
  independent decimal-string oracle on seeded inputs, a refusal corpus with one
  typed error and line number per case, and a native allocation witness
  (`tests/ply_ascii_*.rs`).
