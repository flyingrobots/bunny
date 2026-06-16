# bunny-codec Fixtures

## Stanford Bunny OBJ Subset

`stanford_bunny_subset.obj` is a tiny regression fixture derived from the
Stanford-hosted Bunny OBJ mirror:

* Source: <https://graphics.stanford.edu/~mdfisher/Data/Meshes/bunny.obj>
* Downloaded for fixture extraction: 2026-06-14
* Source header reports `vertex count = 2503` and `face count = 4968`.
* The fixture uses source vertices 1069, 1647, and 1578, and source face
  `f 1069 1647 1578`, remapped locally to `f 1 2 3`.

## Stanford Bunny PLY Binary Subset

The binary PLY fixture bytes in `crates/bunny-codec/tests/stanford_fixture_tests.rs`
are a canonical binary little-endian fixture derived from the official Stanford
3D Scanning Repository Bunny archive:

* Source: <https://graphics.stanford.edu/pub/3Dscanrep/bunny.tar.gz>
* Archive member: `bunny/reconstruction/bun_zipper.ply`
* Downloaded for fixture extraction: 2026-06-14
* Source file reports `element vertex 35947` and `element face 69451`.
* The fixture uses source vertices 21216, 21215, and 20399, and source face
  `3 21216 21215 20399`, remapped locally to face indices `0 1 2`.

## Bunny Compressed Triangle

`canonical_compressed_triangle.bunny.hex` is a generated Bunny-native compressed
mesh profile v1 fixture:

* Generated locally for GP3 on 2026-06-15 from the documented
  `docs/goalposts/v0.4.0-gp3.md` byte profile.
* Magic bytes are `BUNNYQZ!`, version is `1`, index width is `16`, flags are
  `0`, vertex count is `3`, triangle count is `1`, and payload length is `24`.
* Bounds are Q32.32 raw min `(0, 0, 0)` and max `(1, 1, 1)`.
* Vertices are quantized `(0, 0, 0)`, `(65535, 0, 0)`, and `(0, 65535, 0)`.
* The single triangle is `(0, 1, 2)`.
* `compressed_tests.rs` asserts the fixture hex byte-for-byte before decoding it.
