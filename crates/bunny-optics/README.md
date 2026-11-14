# bunny-optics

Deterministic camera and ray-generation math for Bunny.

This crate turns camera parameters and pixel samples into
`bunny_query::TraceRay`s using the deterministic Q32.32 numeric profile, with
integer arithmetic only, so every supported target generates the same rays.

## Features

* `PinholeCamera`: built from a position, a look direction, an up hint, the
  tangent of half the vertical field of view and an integer image size; no
  trigonometry. Degenerate input returns a typed `CameraError`.
* A right-handed camera frame (`right = forward x up`) and a named image
  convention: pixel `(0, 0)` is bottom-left, rows grow upward.
* `PinholeCamera::ray(PixelSample)`: one trace ray per pixel and sub-pixel
  offset, with no square root or division in the camera. Directions are not
  normalized; a construction-time scale keeps every component at most 1.0, as
  the trace kernels require, and each component is within one raw unit plus
  `(width + height) / 2^33` of the exact image-plane point.
* `point_at`: the point at a trace distance along an unnormalized ray.
* Golden vectors and an exact image-plane oracle; see
  `docs/topics/optics/`.

## License

Apache-2.0.
