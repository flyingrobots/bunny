# bunny-mesh

Quantized mesh layouts and deterministic verification hashes for Bunny.

This crate owns the compact mesh-side data structures used by the Stanford
Bunny pipeline. It converts fixed-point vertices to 16-bit quantized coordinates
relative to explicit bounds and frames mesh hash input so vertex data, face
layout, and quantization bounds cannot collide.

## Features

* Q32.32-to-`u16` scalar and vertex quantization.
* Deterministic dequantization for fixed-point reconstruction.
* Wide-intermediate quantization and dequantization at raw Q32.32 extremes, so
  full-span bounds clamp or reconstruct without host-width overflow.
* 16-bit and 32-bit triangle index layout validation.
* SHA-256 mesh hashes with domain, bounds, vertex, and face framing
  (`compute_mesh_hash`, `mesh-hash` feature).
* `MeshView` and `MeshBvh`: a checked, borrowed view of a vertex buffer and an
  index buffer (triangle `i` is BVH primitive `i`), its triangle bounds for the
  BVH builders, and the closest-hit and closest-point searches over the built
  tree, with typed `MeshError` refusals. See `docs/topics/mesh-queries/` in the
  repository.

## Cargo Features

`mesh-hash` (on by default) provides `compute_mesh_hash` and is the crate's only
crates.io dependency, `sha2`. With `default-features = false` every other item is
available and the crate depends only on `bunny-broadphase`, `bunny-geom`,
`bunny-linalg`, `bunny-num` and `bunny-query`.

## License

Apache-2.0.
