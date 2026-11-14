# bunny-optics

Deterministic camera, ray-generation, sampling, diffuse shading and pixel-value
math for Bunny.

This crate turns camera parameters and pixel samples into
`bunny_query::TraceRay`s using the deterministic Q32.32 numeric profile, with
integer arithmetic only, so every supported target generates the same rays.
It also chooses the samples: every sample position and random value is a pure
function of (seed, pixel, sample, dimension), so an image does not depend on
evaluation order or thread count. It shades trace hits on diffuse surfaces
under a directional light and a uniform sky, with at most one bounce. And it
turns accumulated radiance samples
into 8-bit and 16-bit output codes, with the same integer-only rule, so every
target produces the same pixels. It never allocates an image: callers own
every buffer.

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
* `PixelStream` (`bunny-sample-hash/1`): 64 mixed bits per (seed, pixel,
  sample, dimension) from a chain of SplitMix64 mixes, with no state;
  `fraction` and `fraction_from_bits` give the top 32 bits as a Q32.32
  fraction in `[0, 1)`.
* `PixelSampler` (`bunny-sobol-owen/1`): points in `[0, 1)^2` per (seed,
  pixel, sample, dimension pair) from the first two Sobol dimensions with a
  hash-based Owen scramble. Every aligned block of `2^k` samples is a
  (0, k, 2)-net: one point in every dyadic cell of area `2^-k`.
  `camera_sample` returns pair 0 as a `PixelSample` for `PinholeCamera::ray`.
* `Xoroshiro128Plus`: a sequential generator, bit for bit compatible with
  Echo's `warp-math` `Prng`, for code that draws in one fixed order.
* Diffuse shading (`bunny-diffuse-shading/1`): `TriangleSurface` (a
  triangle's exact plane, unit normal within one raw unit, and Duff et al.
  frame, built once per triangle; `None` for zero area), `SurfaceHit` (the
  facing rule, and secondary rays that start strictly on the arrival side of
  the exact plane, at most two raw-unit steps from the hit point, so they never
  hit their own or a coplanar surface), `ShadingFrame::cosine_direction` (a
  cosine-weighted bounce direction from a sampler point, within 4 raw units of
  the ideal map and never below the surface), `DirectionalLight` (irradiance
  convention: a white surface reflects `E * cos / pi`; the shadow ray and its
  limit `SHADOW_T_MAX`), `SkyLight` and `Albedo::reflect`
  (`albedo * (direct + indirect)`; the bounce term has no `pi`). The
  `try_` forms `SurfaceHit::try_new`, `SurfaceHit::try_bounce_ray` and
  `ShadingFrame::try_cosine_direction` say why an input was refused, with a
  `SecondaryRayError` that carries the values it saw.
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
* Golden vectors, an exact image-plane oracle, an exact stratification
  property, an `f64` sRGB reference oracle, and shading oracles and
  constructed scenes in the tests; see
  `docs/topics/optics/`.

## License

Apache-2.0.
