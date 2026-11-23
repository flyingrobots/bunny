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
* `PinholeCamera::try_ray` and `Albedo::try_reflect`: the diagnostic paths of
  `PinholeCamera::ray` and `Albedo::reflect`, returning the same value or a
  `CameraRayError` (the column, row or sub-pixel offset outside the image) or
  an `AlbedoReflectError` (the first channel whose radiance sum leaves
  Q32.32). The `Option` forms are unchanged.
* `ThinLensCamera` (`bunny-thin-lens/1`): a `CameraSpec` plus an aperture
  radius and a focus distance. Each ray starts at a lens-disc point chosen by
  one sampler pair (`lens_disc_point`, polar and area-preserving) and passes
  its sample's focus-plane point within a stated bound; aperture 0 gives the
  pinhole's rays bit for bit. Invalid lenses return a typed `ThinLensError`.
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
  `Option` forms are the fast path; the diagnostic `try_` forms
  `SurfaceHit::try_new`, `SurfaceHit::try_bounce_ray` and
  `ShadingFrame::try_cosine_direction`, called after a `None`, say why an
  input was refused, with a `SecondaryRayError` that carries the values it
  saw.
* `VertexNormals` (`bunny-vertex-normals/1`): one area-weighted unit normal
  per vertex of an indexed triangle mesh, from the exact sum of its faces'
  cross products, within one raw unit of the exact unit vector and independent
  of face order; `None` for a vertex with no face or a zero sum, and a typed
  `VertexNormalError` for a mesh outside the trace domain.
* `smooth_normal` (`bunny-smooth-normal/1`): the shading normal at a hit,
  interpolated from three vertex normals with the hit's exact barycentric
  numerators and normalized once, with no division by `det`; a shared vertex
  normal is returned unchanged, and `None` means the normals cancel there.
* `SmoothHit` (`bunny-smooth-shading/1`): a surface hit shaded with that
  normal, turned to the arrival side by the exact plane (or the geometric frame
  when it lies along the surface or is missing); its light term uses the
  shading cosine and the geometric shadow test; `bounce` rejects a direction
  at or below the exact geometric plane (`SmoothBounce::BelowSurface`).
* `SurfaceHit::specular_reflection` (`bunny-specular-reflection/1`): a
  perfect mirror's continuation ray, along the exact reflection about the
  frame normal normalized once, and its throughput, `schlick_fresnel` per
  channel from a base reflectance (each step rounded once; the base
  reflectance exactly at normal incidence), as a `SpecularSample`; a ray from
  below the face and a grazing reflection under the plane are refused with a
  `SecondaryRayError`.
* `PointLight` (`bunny-point-light/1`): a light at a point given by its
  radiant intensity; `irradiance` is `intensity * cos / d^2`, each step
  rounded once, and `SurfaceHit::point_light_sample` returns a `LightSample`
  whose shadow ray stops one raw unit short of the light
  (`LightSample::shadow_t_max`); a point at most 1/128 unit from the light is
  refused with `PointLightError::TooNear`.
* `DirectionalLight::from_angles` and `DirectionalLight::from_direction`
  (`bunny-light-direction/1`): a directional light toward
  `(cos e cos a, sin e, cos e sin a)` (azimuth 0 along `+x`, a positive
  azimuth toward `+z`, a positive elevation toward `+y`) or toward any nonzero
  vector, each component divided by the largest magnitude and rounded to
  nearest, ties to even, so the largest is exactly `+-1.0`; from angles each
  component is within 9.161 raw units of the exact scaled direction. The zero
  vector is refused with `LightDirectionError::ZeroDirection`.
* `SurfaceHit::dielectric_scatter` (`bunny-dielectric/1`): the reflected or
  refracted continuation ray at a smooth boundary between two refractive
  indices (`DielectricBoundary`, each in `1.0..=8.0`, ordered by the hit's
  facing), chosen by one sampler value. Total internal reflection is one exact
  256-bit integer comparison; the reflectance is the exact unpolarized Fresnel
  formula rounded once (within one raw unit) and is the probability of
  reflection, so each branch's throughput is exactly 1.0; the refracted
  direction is within one raw unit of Snell's law about the frame normal, and
  its ray starts at the nearest point strictly on the far side of the exact
  plane. The result is a `DielectricScatter` holding a `SpecularSample`. The
  throughput of 1.0 is a weight for power: a camera path that refracts from
  index `a` into index `b` must multiply it by `(a / b)^2` to carry radiance
  (`4 / 9` from air into glass at 1.5); the factors cancel on a path that
  enters a medium and leaves it.
* `interpolate_uv` and `bilinear` (`bunny-texture-bilinear/1`): a
  triangle's texture coordinates at a hit, rounded once from the exact
  barycentric interpolation, and the four texels and weights of a bilinear
  lookup (texel centres at `(i + 1/2) / size`, `TextureWrap::Repeat` or
  `Clamp` per axis, weights summing to exactly 1.0), as a `BilinearTaps`.
* `PinholeCamera::footprint` and `ThinLensCamera::footprint`
  (`bunny-ray-footprint/1`): how far the texture coordinates of a
  `TexturedTriangle` change across one pixel to the right and one pixel down,
  from the camera's rays through the same sub-pixel offset in the next column
  and row (the thin lens holding its lens point fixed) met with the
  triangle's plane. Each derivative is the exact difference rounded once to
  nearest, within half a raw unit. A neighbouring ray parallel to the plane,
  meeting it behind the origin, or giving a derivative outside Q32.32 is a
  typed `RayFootprint::Unbounded` result; invalid inputs return a
  `RayFootprintError`.
* `SurfaceHit::ggx_reflection` (`bunny-ggx-conductor/1`): one sample of a
  rough metal. A GGX microfacet normal visible from the incoming ray (Heitz
  2018) from one sampler pair, the reflection about it (`reflect`, shared with
  the mirror), and the weight `schlick_fresnel` times the height-correlated
  Smith ratio `G2 / G1`, as a `GgxScatter`: `Reflected(SpecularSample)`, or
  `Absorbed` when the direction does not leave the surface. A `GgxConductor`
  holds the base reflectance and the roughness `alpha` in `2^-16..=1.0`.
* `PixelAccumulator`: exact, order-independent `i64` sums of linear radiance
  samples (each channel `0..=2^52` raw, up to 1 024 samples, so no sum
  exceeds `2^62`), merging, and the mean rounded to nearest with ties to even.
  Out-of-range input returns a typed `PixelError`; nothing saturates or wraps.
* `ToneCurve`: `SrgbV1` (`bunny-tone-srgb/1`, the sRGB transfer function from a
  1 153-knot table, within `1.49 * 10^-6` of the exact curve and monotone) and
  `LinearClampV1` (`bunny-tone-linear-clamp/1`).
* `srgb8_to_linear` (`bunny-srgb8-decode/1`): the linear light of an 8-bit
  sRGB code from a 256-entry table, each entry the exact decode rounded to
  nearest; `bunny-tone-srgb/1` encodes each back to its code.
* `quantize_8`, `quantize_16` (`bunny-unorm8/1`, `bunny-unorm16/1`): codes
  rounded to nearest with ties to even; `write_rgba8` writes one opaque RGBA8
  pixel (`bunny-rgba8-opaque/1`) into a caller-provided four-byte slice.
* Golden vectors, an exact image-plane oracle, an exact stratification
  property, an `f64` sRGB reference oracle, and shading oracles and
  constructed scenes in the tests; see
  `docs/topics/optics/`.

## License

Apache-2.0.
