# bunny-optics

Deterministic camera, ray-generation and sampling math for Bunny.

This crate turns camera parameters and pixel samples into
`bunny_query::TraceRay`s using the deterministic Q32.32 numeric profile, with
integer arithmetic only, so every supported target generates the same rays.
It also chooses the samples: every sample position and random value is a pure
function of (seed, pixel, sample, dimension), so an image does not depend on
evaluation order or thread count.

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
* Golden vectors, an exact image-plane oracle and an exact stratification
  property; see `docs/topics/optics/`.

## License

Apache-2.0.
