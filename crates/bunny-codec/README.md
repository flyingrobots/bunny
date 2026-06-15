# bunny-codec

`bunny-codec` contains deterministic mesh ingress codecs for Bunny.

## Public Profiles

| API | Profile | Allocation contract |
| --- | --- | --- |
| `parse_binary_ply` | Canonical binary little-endian PLY triangle mesh | Borrowed view, zero allocations after warm-up |
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

## Verification

The compressed decoder is covered by:

* A checked-in canonical fixture at
  `tests/fixtures/canonical_compressed_triangle.bunny.hex`.
* Width-16 and width-32 accepted-path tests.
* A malformed-input corpus covering every public compressed decoder error.
* Native allocation witnesses.
* `wasm_bindgen_test` coverage for the public decoder corpus.
