# bunny-optics

Deterministic camera, ray-generation and pixel-value math for Bunny.

This crate turns camera parameters and pixel samples into
`bunny_query::TraceRay`s, and accumulated radiance samples into 8-bit and
16-bit output codes, using the deterministic Q32.32 numeric profile with
integer arithmetic only, so every supported target produces the same rays and
the same pixels. It never allocates an image: callers own every buffer.

## Features

* `PinholeCamera`: built from a position, a look direction, an up hint, the
  tangent of half the vertical field of view and an integer image size; no
  trigonometry. Degenerate input returns a typed `CameraError`.
* A right-handed camera frame (`right = forward x up`) and a named image
  convention: pixel `(0, 0)` is top-left and rows grow downward, as in
  `bunny_linalg::Viewport`.
* `PinholeCamera::ray(PixelSample)`: one trace ray per pixel and sub-pixel
  offset, with no square root or division in the camera. Directions are not
  normalized; a construction-time scale keeps every component at most 1.0, as
  the trace kernels require, and each component is within one raw unit plus
  `(width + height) / 2^33` of the exact image-plane point.
* `point_at`: the point at a trace distance along an unnormalized ray.
* `PixelAccumulator`: exact, order-independent `i64` sums of linear radiance
  samples (each channel `0..=2^52` raw, up to 1 024 samples, so no sum
  exceeds `2^62`), merging, and the mean rounded to nearest with ties to even.
  Out-of-range input returns a typed `PixelError`; nothing saturates or wraps.
* `ToneCurve`: `SrgbV1` (`bunny-tone-srgb/1`, the sRGB transfer function from a
  1 153-knot table, within `1.49 * 10^-6` of the exact curve and monotone) and
  `LinearClampV1` (`bunny-tone-linear-clamp/1`).
* `quantize_8`, `quantize_16` (`bunny-unorm8/1`, `bunny-unorm16/1`): codes
  rounded to nearest with ties to even; `write_rgba8` writes one opaque RGBA8
  pixel (`bunny-rgba8-opaque/1`) into a caller-provided four-byte slice.
* Golden vectors, an exact image-plane oracle and an `f64` sRGB reference
  oracle in the tests; see `docs/topics/optics/`.

## License

Apache-2.0.
