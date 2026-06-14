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
