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
* SHA-256 mesh hashes with domain, bounds, vertex, and face framing.

## License

Apache-2.0.
