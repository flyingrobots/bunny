# Optics Test Plan

This document defines how Bunny verifies the [optics contract](README.md): the
pinhole camera in `bunny-optics` and the rays it generates, the
deterministic sample hash, pixel sampler and sequential generator that choose
sample positions and random decisions, and the pixel-value functions that turn
radiance samples into output codes.

## Scope

This plan covers `PinholeCamera`, `CameraSpec`, `PixelSample`, `CameraError`
and `point_at`:

- construction refusals for every degenerate input;
- the camera basis and its agreement with the coordinate law;
- the image convention: pixel origin, row and column directions, pixel centre;
- the direction scaling rule that keeps every direction component at most 1.0;
- the exact, floored direction formula and its bound against the intended
  image-plane point;
- sample refusals outside the image;
- the point at a hit distance and its rounding;
- the counter-based sample hash `bunny-sample-hash/1` (`PixelStream`,
  `fraction_from_bits`): golden vectors, range, domain separation of its key
  fields, evaluation-order independence and an avalanche guard;
- the pixel sampler `bunny-sobol-owen/1` (`PixelSampler`): golden vectors,
  exact stratification of every power-of-two block of samples, range,
  decorrelation between dimension pairs and pixels, and use as a camera sample;
- `Xoroshiro128Plus`, replaying Echo's `warp-math` golden vectors bit for bit;
- `PixelAccumulator`: exact, order-independent sums, their limits and the
  mean's rounding;
- `ToneCurve` (`SrgbV1`, `LinearClampV1`): golden vectors, the sRGB error
  bound against a reference oracle, and monotonicity;
- `quantize_8`, `quantize_16` and the codes they give against the correctly
  rounded sRGB codes;
- `write_rgba8` and the versioned identities;
- the diffuse shading model `bunny-diffuse-shading/1` (`TriangleSurface`,
  `ShadingFrame`, `SurfaceHit`, `DirectionalLight`, `LightSample`, `SkyLight`,
  `Albedo`, `ShadingError`, `SecondaryRayError` and the constants `PI`, `FRAC_1_PI`,
  `MAX_LIGHT_RAW`, `SHADOW_T_MAX`, `MIN_BOUNCE_COSINE_RAW`, `SHADING_ID`):
  golden vectors for every step, the normal, frame, hemisphere and cosine
  bounds against floating-point and exact integer oracles, a distribution
  guard, energy sanity, the self-intersection rule on constructed scenes, the
  shadow-ray limit, refusals, and composed paths that lock the order of
  operations.
- the typed refusals of `SurfaceHit::try_new`, `SurfaceHit::try_bounce_ray`
  and `ShadingFrame::try_cosine_direction`: one constructed case per
  reachable `SecondaryRayError` variant, and a seeded differential case that
  the `Option` forms keep the 0.7.1 outputs and agree with the `try_` forms.

Projection matrices (flyingrobots/bunny#201, #202), and reflection, refraction
and other material helpers (#160) are out of scope.

## Test Goals

- Lock raw ray outputs with golden vectors for hand-checkable cameras:
  axis-aligned, power-of-two tangents and image sizes, plus one inexact case.
- Check every generated ray against an exact integer oracle of the intended
  image-plane point, over seeded cameras and at the extremes of the accepted
  domain.
- Show that the scaling rule is load bearing by breaking it on purpose.
- Lock the sample hash and the sampler with golden vectors from an
  independent reference model, and check the sampler's stratification as an
  exact counting property, not as a look at the points.
- Show that each key field and the stratification rule are load bearing by
  breaking them on purpose.
- Lock every pixel-value function with golden vectors, check the sRGB curve
  against an `f64` reference oracle over dense deterministic inputs, and find
  exactly where each 8-bit and 16-bit code begins compared with the correctly
  rounded code.
- Keep the evidence native and WASM-compatible: every case is a
  `wasm_bindgen_test` that also runs as a native test.

## Requirements

The table is the human-readable view. The fenced TOML block immediately after
it is the contract graph consumed by `cargo run --locked -p xtask --
topic-docs`. Tooling reads only the structured block, not visual Markdown table
formatting.

| ID | Requirement | Current Source |
| --- | --- | --- |
| OPTICS-REQ-001 | Construction refuses a zero look direction, an up hint that is zero or within about `2^-16` of parallel to the look direction, a position outside the trace domain, a zero or too-large image dimension, and a non-positive or too-large tangent, each with its own `CameraError`; look and up vectors of any other length, from one raw unit to the full Q32.32 range, are accepted. | `README.md#construction` |
| OPTICS-REQ-002 | The basis is right-handed: forward is the normalized look direction, up is on the up hint's side and orthogonal to forward, and right is `forward x up`; a camera looking down `-Z` with up `+Y` has right `+X`. | `README.md#camera-frame` |
| OPTICS-REQ-003 | Pixel `(0, 0)` is the top-left pixel, as in the coordinate law's viewport conversion: column `x` grows toward right, row `y` grows toward down (`-up`), a sub-pixel offset of one half is the pixel centre, pixels are square and the image spans `-tan..=tan` vertically on the plane one unit ahead. | `README.md#image-convention` |
| OPTICS-REQ-004 | For every accepted camera and every in-range sample, `ray` returns a `TraceRay` whose direction components are at most 1.0 in magnitude and whose direction has a positive forward component. | `README.md#direction-scaling` |
| OPTICS-REQ-005 | Each direction component is the floor of one exact affine expression in the sample position, and lies within one raw unit plus `(width + height) / 2^33` raw units below, and `(width + height) / 2^33` above, `2^-scale_shift` times the intended image-plane point. | `README.md#accuracy` |
| OPTICS-REQ-006 | The scale shift is the smallest `k` in `0..=32` for which the four corner directions of the image rectangle fit the trace direction domain, inclusive of 1.0. | `README.md#direction-scaling` |
| OPTICS-REQ-007 | `ray` returns `None` for a column or row outside the image and for a sub-pixel offset outside `0..1`. | `README.md#ray-generation` |
| OPTICS-REQ-008 | `point_at` returns `origin + direction * t` with each product rounded to nearest (ties to even), and `None` when a coordinate is not representable. | `README.md#distances-along-a-ray` |
| OPTICS-REQ-009 | Ray generation is a pure integer function of the camera and the sample: the same inputs give the same raw outputs on every target. | `README.md#ray-generation` |
| OPTICS-REQ-010 | `PixelStream::bits` is the stated `bunny-sample-hash/1` function of (seed, x, y, sample, dimension): a pure function with no state, using only wrapping 64-bit multiplications, xors, shifts and additions. | `README.md#sample-hash` |
| OPTICS-REQ-011 | Keys that differ only in the pixel, or only in (sample, dimension), never give the same bits: each absorption step is a bijection of the absorbed word. The sampler and the hash use separate domains. | `README.md#sample-hash` |
| OPTICS-REQ-012 | `fraction_from_bits` and `PixelStream::fraction` return the top 32 bits as a Q32.32 fraction, which is in `[0, 1)` and never 1.0; every sampler coordinate is in `[0, 1)`. | `README.md#fractions` |
| OPTICS-REQ-013 | For every pixel, seed, dimension pair, `k` in `0..=32` and block `m`, the sampler's points for samples `m * 2^k .. (m + 1) * 2^k` form a (0, k, 2)-net in base 2: every cell of each of the `k + 1` partitions of the unit square into `2^p` by `2^(k - p)` dyadic cells holds exactly one point. | `README.md#stratification` |
| OPTICS-REQ-014 | Every value depends only on its key: computing the same keys in any order, with fresh or reused handles, gives the same bits. | `README.md#determinism` |
| OPTICS-REQ-015 | `PixelSampler::camera_sample` is pair 0 as a `PixelSample` of the sampler's own pixel, accepted by `PinholeCamera::ray` for every in-range pixel. | `README.md#pixel-sampler` |
| OPTICS-REQ-016 | `Xoroshiro128Plus` reproduces Echo's `warp-math` `Prng` bit for bit: `from_seed`, `from_seed_u64` (SplitMix64 expansion), the all-zero seed replacement, `next_u64`, `next_int` (refusing an empty range with `None`) and `next_unit` (Echo's `next_f32` value as exact Q32.32). | `README.md#sequential-generator` |
| OPTICS-REQ-017 | `PixelAccumulator` sums each channel exactly in `i64`: the sums and the count do not depend on the order samples are added in or on how they are split between accumulators that are then merged. | `README.md#accumulation` |
| OPTICS-REQ-018 | A sample channel is accepted in `0..=2^52` raw and a pixel holds at most 1024 samples, so no sum exceeds `2^62`; a negative channel, a channel above `2^52` raw, a sample or merge past 1024 samples each return their own `PixelError` and leave the accumulator unchanged. | `README.md#accumulation` |
| OPTICS-REQ-019 | `resolve` returns each sum divided by the count, rounded to nearest with ties to even (for a power-of-two count, the same rounding of a right shift), and `NoSamples` for an empty pixel. | `README.md#resolve` |
| OPTICS-REQ-020 | `ToneCurve::SrgbV1` is 0 at or below zero, 1.0 at or above 1.0, `round(12.92 x)` below the first raw input at or above 0.0031308, and above it the floored linear interpolation of 1153 correctly rounded knots of `1.055 x^(1/2.4) - 0.055`, held at the toe's last value until it rises past it; it is within 6400 raw units (1.49e-6) of the exact sRGB curve and monotone over every input. | `README.md#tone-curves` |
| OPTICS-REQ-021 | `ToneCurve::LinearClampV1` clamps its input to `[0, 1]` and is otherwise the identity. | `README.md#tone-curves` |
| OPTICS-REQ-022 | `quantize_8` and `quantize_16` clamp a display value to `[0, 1]` and return `round(255 v)` and `round(65535 v)`, to nearest with ties to even; both are monotone. | `README.md#output-codes` |
| OPTICS-REQ-023 | Under `SrgbV1`, every 8-bit and 16-bit code is within one of the correctly rounded sRGB code of the same input; each code begins at most 9600 raw inputs after and at most 2 raw inputs before the input where the correctly rounded code begins. | `README.md#accuracy-of-the-codes` |
| OPTICS-REQ-024 | `write_rgba8` writes red, green and blue 8-bit codes and alpha 255 into a slice of exactly four bytes, and returns `WrongSliceLength` without writing for any other length. | `README.md#output-codes` |
| OPTICS-REQ-025 | Tone curves and output encodings have stable versioned identities (`bunny-tone-srgb/1`, `bunny-tone-linear-clamp/1`, `bunny-unorm8/1`, `bunny-unorm16/1`, `bunny-rgba8-opaque/1`); golden vectors lock their outputs, and any change to an output is a new version. | `README.md#versioned-identities` |
| OPTICS-REQ-030 | `TriangleSurface::new` returns `None` exactly for zero-area triangles (an exact zero cross product); otherwise its unit normal points along `(v1 - v0) x (v2 - v0)` and each component is within one raw unit of the exact unit normal. | `README.md#geometric-normal` |
| OPTICS-REQ-031 | `ShadingFrame::new` is the stated Duff et al. frame with each component within half a raw unit of the exact formula on the given normal; every frame and its `flipped` form is right-handed and orthonormal to within 8 raw units on each pairwise dot product, squared length and cross-product component. | `README.md#orthonormal-frame` |
| OPTICS-REQ-032 | The facing rule: a `SurfaceHit` frame is the surface frame for a front-face hit and its `flipped` form (bitangent, tangent, negated normal) for a back-face hit, so its normal points to the side the ray arrived from. | `README.md#facing-and-secondary-rays` |
| OPTICS-REQ-033 | The self-intersection rule: the start point of every secondary ray is `point_at(ray, hit.distance)` moved by the fewest whole raw units (at most two) along the plane normal's dominant axis that put it strictly on the arrival side of the triangle's exact plane; bounce and shadow rays then never hit their own triangle, a coplanar neighbour, a coincident duplicate or a convex neighbour, and geometry at least 4 raw units away on the arrival side is not skipped. | `README.md#facing-and-secondary-rays` |
| OPTICS-REQ-034 | `ShadingFrame::cosine_direction` maps `[u, v]` in `[0, 1)^2` to the polar disk point lifted to the hemisphere (`cos theta = sqrt(1 - u)`, azimuth `v` turns): each component within 4 raw units of the ideal map on the frame and within `-1.0..=1.0`, length within 8 raw units of 1.0, dot product with the normal at least `MIN_BOUNCE_COSINE_RAW` (`2^15`) raw units; `None` for a coordinate outside `[0, 1)`. | `README.md#cosine-weighted-hemisphere` |
| OPTICS-REQ-035 | Over stratified sampler points the directions follow the cosine-weighted distribution to the stated regression thresholds: mean cosine within `2^-12` of 2/3 over 4 096 points; over 1 024 points the four cosine bands of weight 1/16, 3/16, 5/16 and 7/16 within one point and each azimuth quarter within two points of the exact count. | `README.md#cosine-weighted-hemisphere` |
| OPTICS-REQ-036 | The radiometric convention: a directional light's value is the irradiance on a surface perpendicular to it; a white diffuse surface lit at cosine `c` reflects `E * (c * FRAC_1_PI)`, each product rounded to nearest with ties to even; `PI` is raw 13 493 037 705 and `FRAC_1_PI` raw 1 367 130 551; the cosine is within 3 raw units of the exact cosine with the given normal and is clamped to `0..=1.0`. | `README.md#radiometric-convention` |
| OPTICS-REQ-037 | The estimator: `Albedo::reflect` is `albedo_c * (direct_c + indirect_c)` with an exact sum and one product rounded to nearest, ties to even; the cosine-weighted bounce term has no `pi`; a white surface under a uniform sky reflects exactly the sky radiance, and under a sky approximated by directional lights within `2^-16` of it. | `README.md#diffuse-reflection` |
| OPTICS-REQ-038 | The directional light's shadow ray starts at the surface hit's start point along `toward_light` when the light is strictly on the arrival side of the exact plane (none otherwise, and no light); `SHADOW_T_MAX` (`2^41` raw, 512 units) covers every occluder in the trace domain. | `README.md#directional-light` |
| OPTICS-REQ-039 | `DirectionalLight::new` refuses a direction whose largest component magnitude is not exactly 1.0; lights and skies refuse a negative channel or one above `MAX_LIGHT_RAW` (65 536.0); `Albedo::new` refuses a channel outside `0..=1.0`; `SurfaceHit::new` refuses a start point outside the trace domain. | `README.md#inputs-and-refusals` |
| OPTICS-REQ-040 | The shading model has the identity `bunny-diffuse-shading/1` (`SHADING_ID`), and one path composed in the reference order of operations gives locked radiance bits. | `README.md#order-of-operations` |
| OPTICS-REQ-041 | `SurfaceHit::try_new`, `SurfaceHit::try_bounce_ray` and `ShadingFrame::try_cosine_direction` name each refusal with a `SecondaryRayError` variant carrying the observed values, checked in this order: `HitPointUnrepresentable { distance }` when `point_at(ray, hit.distance)` is not representable, `HitPointOutsideTraceDomain { point }` when a coordinate of that point is more than two raw units outside the trace domain, `NotOnArrivalSide { point }` when no step of at most two raw units reaches the arrival side, `StartOutsideTraceDomain { start }` when the stepped start point is outside the trace domain, `SampleOutsideUnitInterval { sample }` when a sample coordinate is outside `[0, 1)`, and the defensive `BounceRayRefused { origin, direction }` when `TraceRay::new` refuses the bounce ray. | `README.md#inputs-and-refusals` |
| OPTICS-REQ-042 | `SurfaceHit::new`, `SurfaceHit::bounce_ray` and `ShadingFrame::cosine_direction` keep every output of 0.7.1: each returns `Some(x)` exactly when its `try_` form returns `Ok(x)`, with the same value, and agrees with the 0.7.1 code on every input, except that a hit point far enough outside the trace domain to overflow the 0.7.1 plane test in `i128` (a panic in debug builds) returns `None`, as 0.7.1 release builds did. | `README.md#inputs-and-refusals` |
| OPTICS-REQ-043 | The fast path and the diagnostic path agree: `SurfaceHit::new`, `SurfaceHit::bounce_ray` and `ShadingFrame::cosine_direction` (the fast path, the 0.7.1 bodies plus the plane-test overflow guard) return `Some(x)` exactly when `SurfaceHit::try_new`, `SurfaceHit::try_bounce_ray` and `ShadingFrame::try_cosine_direction` (the diagnostic path) return `Ok(x)`, with equal values, on generated inputs from a fixed and a fresh seed and on the constructed refusals; a change to either side alone breaks it. | `README.md#inputs-and-refusals` |

```toml
[[requirement]]
id = "OPTICS-REQ-001"
summary = "Construction refuses a zero look direction, an up hint that is zero or within about 2^-16 of parallel to the look direction, a position outside the trace domain, a zero or too-large image dimension, and a non-positive or too-large tangent, each with its own CameraError; look and up vectors of any other length, from one raw unit to the full Q32.32 range, are accepted."
status = "active"

[[requirement]]
id = "OPTICS-REQ-002"
summary = "The basis is right-handed: forward is the normalized look direction, up is on the up hint's side and orthogonal to forward, and right is forward x up; a camera looking down -Z with up +Y has right +X."
status = "active"

[[requirement]]
id = "OPTICS-REQ-003"
summary = "Pixel (0, 0) is the top-left pixel, as in the coordinate law's viewport conversion: column x grows toward right, row y grows toward down (-up), a sub-pixel offset of one half is the pixel centre, pixels are square and the image spans -tan..=tan vertically on the plane one unit ahead."
status = "active"

[[requirement]]
id = "OPTICS-REQ-004"
summary = "For every accepted camera and every in-range sample, ray returns a TraceRay whose direction components are at most 1.0 in magnitude and whose direction has a positive forward component."
status = "active"

[[requirement]]
id = "OPTICS-REQ-005"
summary = "Each direction component is the floor of one exact affine expression in the sample position, and lies within one raw unit plus (width + height) / 2^33 raw units below, and (width + height) / 2^33 above, 2^-scale_shift times the intended image-plane point."
status = "active"

[[requirement]]
id = "OPTICS-REQ-006"
summary = "The scale shift is the smallest k in 0..=32 for which the four corner directions of the image rectangle fit the trace direction domain, inclusive of 1.0."
status = "active"

[[requirement]]
id = "OPTICS-REQ-007"
summary = "ray returns None for a column or row outside the image and for a sub-pixel offset outside 0..1."
status = "active"

[[requirement]]
id = "OPTICS-REQ-008"
summary = "point_at returns origin + direction * t with each product rounded to nearest (ties to even), and None when a coordinate is not representable."
status = "active"

[[requirement]]
id = "OPTICS-REQ-009"
summary = "Ray generation is a pure integer function of the camera and the sample: the same inputs give the same raw outputs on every target."
status = "active"

[[requirement]]
id = "OPTICS-REQ-010"
summary = "PixelStream::bits is the stated bunny-sample-hash/1 function of (seed, x, y, sample, dimension): a pure function with no state, using only wrapping 64-bit multiplications, xors, shifts and additions."
status = "active"

[[requirement]]
id = "OPTICS-REQ-011"
summary = "Keys that differ only in the pixel, or only in (sample, dimension), never give the same bits: each absorption step is a bijection of the absorbed word. The sampler and the hash use separate domains."
status = "active"

[[requirement]]
id = "OPTICS-REQ-012"
summary = "fraction_from_bits and PixelStream::fraction return the top 32 bits as a Q32.32 fraction, which is in [0, 1) and never 1.0; every sampler coordinate is in [0, 1)."
status = "active"

[[requirement]]
id = "OPTICS-REQ-013"
summary = "For every pixel, seed, dimension pair, k in 0..=32 and block m, the sampler's points for samples m * 2^k .. (m + 1) * 2^k form a (0, k, 2)-net in base 2: every cell of each of the k + 1 partitions of the unit square into 2^p by 2^(k - p) dyadic cells holds exactly one point."
status = "active"

[[requirement]]
id = "OPTICS-REQ-014"
summary = "Every value depends only on its key: computing the same keys in any order, with fresh or reused handles, gives the same bits."
status = "active"

[[requirement]]
id = "OPTICS-REQ-015"
summary = "PixelSampler::camera_sample is pair 0 as a PixelSample of the sampler's own pixel, accepted by PinholeCamera::ray for every in-range pixel."
status = "active"

[[requirement]]
id = "OPTICS-REQ-016"
summary = "Xoroshiro128Plus reproduces Echo's warp-math Prng bit for bit: from_seed, from_seed_u64 (SplitMix64 expansion), the all-zero seed replacement, next_u64, next_int (refusing an empty range with None) and next_unit (Echo's next_f32 value as exact Q32.32)."
status = "active"

[[requirement]]
id = "OPTICS-REQ-017"
summary = "PixelAccumulator sums each channel exactly in i64: the sums and the count do not depend on the order samples are added in or on how they are split between accumulators that are then merged."
status = "active"

[[requirement]]
id = "OPTICS-REQ-018"
summary = "A sample channel is accepted in 0..=2^52 raw and a pixel holds at most 1024 samples, so no sum exceeds 2^62; a negative channel, a channel above 2^52 raw, a sample or merge past 1024 samples each return their own PixelError and leave the accumulator unchanged."
status = "active"

[[requirement]]
id = "OPTICS-REQ-019"
summary = "resolve returns each sum divided by the count, rounded to nearest with ties to even (for a power-of-two count, the same rounding of a right shift), and NoSamples for an empty pixel."
status = "active"

[[requirement]]
id = "OPTICS-REQ-020"
summary = "ToneCurve::SrgbV1 is 0 at or below zero, 1.0 at or above 1.0, round(12.92 x) below the first raw input at or above 0.0031308, and above it the floored linear interpolation of 1153 correctly rounded knots of 1.055 x^(1/2.4) - 0.055, held at the toe's last value until it rises past it; it is within 6400 raw units (1.49e-6) of the exact sRGB curve and monotone over every input."
status = "active"

[[requirement]]
id = "OPTICS-REQ-021"
summary = "ToneCurve::LinearClampV1 clamps its input to [0, 1] and is otherwise the identity."
status = "active"

[[requirement]]
id = "OPTICS-REQ-022"
summary = "quantize_8 and quantize_16 clamp a display value to [0, 1] and return round(255 v) and round(65535 v), to nearest with ties to even; both are monotone."
status = "active"

[[requirement]]
id = "OPTICS-REQ-023"
summary = "Under SrgbV1, every 8-bit and 16-bit code is within one of the correctly rounded sRGB code of the same input; each code begins at most 9600 raw inputs after and at most 2 raw inputs before the input where the correctly rounded code begins."
status = "active"

[[requirement]]
id = "OPTICS-REQ-024"
summary = "write_rgba8 writes red, green and blue 8-bit codes and alpha 255 into a slice of exactly four bytes, and returns WrongSliceLength without writing for any other length."
status = "active"

[[requirement]]
id = "OPTICS-REQ-025"
summary = "Tone curves and output encodings have stable versioned identities (bunny-tone-srgb/1, bunny-tone-linear-clamp/1, bunny-unorm8/1, bunny-unorm16/1, bunny-rgba8-opaque/1); golden vectors lock their outputs, and any change to an output is a new version."
status = "active"

[[requirement]]
id = "OPTICS-REQ-030"
summary = "TriangleSurface::new returns None exactly for zero-area triangles (an exact zero cross product); otherwise its unit normal points along (v1 - v0) x (v2 - v0) and each component is within one raw unit of the exact unit normal."
status = "active"

[[requirement]]
id = "OPTICS-REQ-031"
summary = "ShadingFrame::new is the stated Duff et al. frame with each component within half a raw unit of the exact formula on the given normal; every frame and its flipped form is right-handed and orthonormal to within 8 raw units on each pairwise dot product, squared length and cross-product component."
status = "active"

[[requirement]]
id = "OPTICS-REQ-032"
summary = "The facing rule: a SurfaceHit frame is the surface frame for a front-face hit and its flipped form (bitangent, tangent, negated normal) for a back-face hit, so its normal points to the side the ray arrived from."
status = "active"

[[requirement]]
id = "OPTICS-REQ-033"
summary = "The self-intersection rule: the start point of every secondary ray is point_at(ray, hit.distance) moved by the fewest whole raw units (at most two) along the plane normal's dominant axis that put it strictly on the arrival side of the triangle's exact plane; bounce and shadow rays then never hit their own triangle, a coplanar neighbour, a coincident duplicate or a convex neighbour, and geometry at least 4 raw units away on the arrival side is not skipped."
status = "active"

[[requirement]]
id = "OPTICS-REQ-034"
summary = "ShadingFrame::cosine_direction maps [u, v] in [0, 1)^2 to the polar disk point lifted to the hemisphere (cos theta = sqrt(1 - u), azimuth v turns): each component within 4 raw units of the ideal map on the frame and within -1.0..=1.0, length within 8 raw units of 1.0, dot product with the normal at least MIN_BOUNCE_COSINE_RAW (2^15) raw units; None for a coordinate outside [0, 1)."
status = "active"

[[requirement]]
id = "OPTICS-REQ-035"
summary = "Over stratified sampler points the directions follow the cosine-weighted distribution to the stated regression thresholds: mean cosine within 2^-12 of 2/3 over 4 096 points; over 1 024 points the four cosine bands of weight 1/16, 3/16, 5/16 and 7/16 within one point and each azimuth quarter within two points of the exact count."
status = "active"

[[requirement]]
id = "OPTICS-REQ-036"
summary = "The radiometric convention: a directional light's value is the irradiance on a surface perpendicular to it; a white diffuse surface lit at cosine c reflects E * (c * FRAC_1_PI), each product rounded to nearest with ties to even; PI is raw 13 493 037 705 and FRAC_1_PI raw 1 367 130 551; the cosine is within 3 raw units of the exact cosine with the given normal and is clamped to 0..=1.0."
status = "active"

[[requirement]]
id = "OPTICS-REQ-037"
summary = "The estimator: Albedo::reflect is albedo_c * (direct_c + indirect_c) with an exact sum and one product rounded to nearest, ties to even; the cosine-weighted bounce term has no pi; a white surface under a uniform sky reflects exactly the sky radiance, and under a sky approximated by directional lights within 2^-16 of it."
status = "active"

[[requirement]]
id = "OPTICS-REQ-038"
summary = "The directional light's shadow ray starts at the surface hit's start point along toward_light when the light is strictly on the arrival side of the exact plane (none otherwise, and no light); SHADOW_T_MAX (2^41 raw, 512 units) covers every occluder in the trace domain."
status = "active"

[[requirement]]
id = "OPTICS-REQ-039"
summary = "DirectionalLight::new refuses a direction whose largest component magnitude is not exactly 1.0; lights and skies refuse a negative channel or one above MAX_LIGHT_RAW (65 536.0); Albedo::new refuses a channel outside 0..=1.0; SurfaceHit::new refuses a start point outside the trace domain."
status = "active"

[[requirement]]
id = "OPTICS-REQ-040"
summary = "The shading model has the identity bunny-diffuse-shading/1 (SHADING_ID), and one path composed in the reference order of operations gives locked radiance bits."
status = "active"

[[requirement]]
id = "OPTICS-REQ-041"
summary = "SurfaceHit::try_new, SurfaceHit::try_bounce_ray and ShadingFrame::try_cosine_direction name each refusal with a SecondaryRayError variant carrying the observed values, checked in this order: HitPointUnrepresentable { distance } when point_at(ray, hit.distance) is not representable, HitPointOutsideTraceDomain { point } when a coordinate of that point is more than two raw units outside the trace domain, NotOnArrivalSide { point } when no step of at most two raw units reaches the arrival side, StartOutsideTraceDomain { start } when the stepped start point is outside the trace domain, SampleOutsideUnitInterval { sample } when a sample coordinate is outside [0, 1), and the defensive BounceRayRefused { origin, direction } when TraceRay::new refuses the bounce ray."
status = "active"

[[requirement]]
id = "OPTICS-REQ-042"
summary = "SurfaceHit::new, SurfaceHit::bounce_ray and ShadingFrame::cosine_direction keep every output of 0.7.1: each returns Some(x) exactly when its try_ form returns Ok(x), with the same value, and agrees with the 0.7.1 code on every input, except that a hit point far enough outside the trace domain to overflow the 0.7.1 plane test in i128 (a panic in debug builds) returns None, as 0.7.1 release builds did."
status = "active"

[[requirement]]
id = "OPTICS-REQ-043"
summary = "The fast path and the diagnostic path agree: SurfaceHit::new, SurfaceHit::bounce_ray and ShadingFrame::cosine_direction (the fast path, the 0.7.1 bodies plus the plane-test overflow guard) return Some(x) exactly when SurfaceHit::try_new, SurfaceHit::try_bounce_ray and ShadingFrame::try_cosine_direction (the diagnostic path) return Ok(x), with equal values, on generated inputs from a fixed and a fresh seed and on the constructed refusals; a change to either side alone breaks it."
status = "active"
```

## Fixtures

No external fixtures are required. Every case builds cameras and samples from
raw Q32.32 values in the test file.

Seeded cases use a fixed 64-bit linear congruential generator with the seed
`OPTICS_SEED` (`0xB002_0971_C500_0001`), the same generator as the ray-tracing
test plan. Failure messages name the case index.

The golden vectors in `OPTICS-TP-002` are hand computed; the comments beside
each case give the exact value each floor is taken of. Regenerating them is a
contract change.

### The Sampler Reference Model

The golden vectors of `OPTICS-TP-010` and `OPTICS-TP-013` were produced by a
separate Python model written from the definitions in the
[chapter](README.md#sample-hash), with Python's unbounded integers masked to
64 and 32 bits; it shares no code with the crate. The model's `SplitMix64`
finalizer was first checked against the published SplitMix64 outputs for the
seeds 0 (`0xE220A8397B1DCDAF`) and 1234567 (`6457827717110365317`,
`3203168211198807973`, ...), and the crate's copy is checked against the same
published values through `Xoroshiro128Plus::from_seed_u64` in `OPTICS-TP-020`.
The model also checked the stratification property of `OPTICS-TP-012` before
the crate existed. Regenerating these vectors is a contract change: a new
`bunny-sample-hash` or `bunny-sobol-owen` version.

The Echo vectors in `OPTICS-TP-019` are copied from Echo's
`crates/warp-math/tests/prng_golden_regression.rs` and
`crates/warp-math/tests/fixtures/math-fixtures.json` (the `next_f32` values
converted exactly: each is `k * 2^-23` for the integer `k` in the test).

The pixel cases (`crates/bunny-optics/tests/pixel_tests.rs`) use the camera
cases' linear congruential generator with the seed `PIXEL_SEED` (`0xB002_0971_C500_0222`). Their golden
tone vectors come from an independent integer model of the curve whose knots
were computed with 60-digit decimal arithmetic (in a scratch directory, not
committed); every knot is also rechecked in `OPTICS-TP-026`.

### The sRGB Reference Oracle

`CODE_STANDARDS.md` Rule 2 bans floating point in canonical math, and the
Code Dojo float rule applies to core crates only, so the pixel test file uses
`f64` as a reference oracle, never the library. The oracle is the IEC
61966-2-1 formula, `12.92 x` below 0.0031308 and `1.055 x^(1/2.4) - 0.055`
above, on `x = raw / 2^32`, which is exact in `f64` for every input tested.

`f64` is close enough to decide each comparison the cases make:

- `powf` is accurate to about `10^-6` raw Q32.32 units of display value; the
  nearest knot to a rounding tie is `2.7 * 10^-4` raw units away, and
  `OPTICS-TP-026` asserts a margin of `10^-4`;
- where a code boundary is compared, one raw input changes the reference by
  at least `10^-6` codes, and on both sides of every 8-bit and 16-bit
  boundary the 60-digit model found the reference at least `5 * 10^-11` codes
  away from it, against an `f64` error near `10^-11` codes;
- the reference curve itself drops by `2.9 * 10^-8` at the toe end (the
  standard's constants do not meet), but no 8-bit or 16-bit code boundary lies
  in that drop, so each code still begins at one well-defined input.

The asserted bounds (`9600` and `2` inputs in `OPTICS-TP-029`) leave a margin
over the values the 60-digit model measured (9 447 and 9 522 late, 1 early) so
that a one-input `f64` misjudgement cannot fail the case.

### The Shading Reference Model

The golden vectors of `OPTICS-TP-040`, `OPTICS-TP-043`, `OPTICS-TP-046`,
`OPTICS-TP-048` and `OPTICS-TP-053` that are not hand computed (each case says
which) were produced by a separate Python model written from the
[Diffuse Shading](README.md#diffuse-shading) definitions, with Python's
unbounded integers; it shares no code with the crate. It models
`FixedUnitVec3::new`, the Q32.32 multiplication and division rules, the exact
triangle test of `bunny-query` and the reference order of operations. Sines
and cosines enter it only at angles whose `FixedAngle::sin_cos` values are
exact or in that function's own golden table (multiples of an eighth of a
turn). Regenerating these vectors is a contract change: a new
`bunny-diffuse-shading` version.

The property cases use floating point only as an oracle in the test files
(never in the crate), as `crates/bunny-num/tests/fixed_angle_tests.rs` does;
its error is about `2^-53` relative, far below the raw-unit bounds checked.
The frame and self-intersection cases are exact `i128` integer checks.
Seeded shading cases use the plan's generator with the seed
`SHADING_TEST_SEED` (`0xB002_0160_D1FF_0001`), and sampler points from
`PixelSampler` with the seed `0x0123_4567_89AB_CDEF`.

### The Image-Plane Oracle

The oracle shares no code or rounding with the camera. For a sample with
doubled raw offsets `a = 2 * (x + sub_x) - width` and `b = height - 2 * (y +
sub_y)` (raw Q32.32, so `a` is an exact integer), it reads the camera's stored
basis `F`, `R`, `U`, tangent `T` (all raw) and scale shift `k`, and forms in
`i128`

```text
S_i   = F_i * H * 2^64 + a * T * R_i + b * T * U_i
D     = H * 2^(64 + k)
slack = (W + H) * H * 2^(31 + k)
```

`S_i / D` is the raw value of `2^-k` times component `i` of the intended
image-plane point. The case passes when `-D - slack < d_i * D - S_i <= slack`
for each component, which is `OPTICS-REQ-005` multiplied through by `D`. All
products are checked; an overflow fails the case.

## Test Cases

| ID | Category | Requirements | Oracle | Test |
| --- | --- | --- | --- | --- |
| OPTICS-TP-001 | Golden basis | OPTICS-REQ-002 | Exact raw forward, right and up for cameras looking down `-Z`, along `+X` and along `+Y`, and for an up hint tilted toward the viewer; right equals `forward x up` exactly for each. | `crates/bunny-optics/tests/camera_tests.rs::optics_tp_001_golden_basis_is_right_handed` |
| OPTICS-TP-002 | Golden rays | OPTICS-REQ-003, OPTICS-REQ-005, OPTICS-REQ-006, OPTICS-REQ-009 | Hand-computed raw directions and scale shifts for five cameras, including a corner sample exactly at 1.0, a 2:1 image that needs one halving, a 3x3 image whose directions are floored thirds, and a moved camera along `+X`. | `crates/bunny-optics/tests/camera_tests.rs::optics_tp_002_golden_rays_lock_raw_outputs` |
| OPTICS-TP-003 | Image-plane property | OPTICS-REQ-002, OPTICS-REQ-004, OPTICS-REQ-005 | The exact integer image-plane oracle over 2 000 seeded camera specifications (at least 1 990 must construct) and four samples each; the basis is orthogonal to within `2^-28`, right-handed and on the up hint's side. | `crates/bunny-optics/tests/camera_tests.rs::optics_tp_003_rays_pass_through_the_intended_image_plane_point` |
| OPTICS-TP-004 | Domain extremes | OPTICS-REQ-004, OPTICS-REQ-005, OPTICS-REQ-006 | At the widest accepted tangent, the largest image, the most extreme aspect ratios, the narrowest tangent and a tangent just above 1.0, with axis-aligned and diagonal look directions, every corner pixel at sub-pixel offsets 0 and `1 - 2^-32` yields a ray with a positive forward component that satisfies the image-plane oracle. | `crates/bunny-optics/tests/camera_tests.rs::optics_tp_004_corner_samples_of_extreme_cameras_stay_in_the_trace_domain` |
| OPTICS-TP-005 | Construction refusals | OPTICS-REQ-001 | Each degenerate input returns its error; values at each accepted bound construct and values one raw unit or one pixel past it are refused; the shortest and longest look and up vectors construct. | `crates/bunny-optics/tests/camera_tests.rs::optics_tp_005_degenerate_cameras_are_refused` |
| OPTICS-TP-006 | Sample refusals | OPTICS-REQ-007 | Columns and rows at the image size, sub-pixel offsets of 1.0 and below zero return `None`; offsets 0 and `1 - 2^-32` return rays. | `crates/bunny-optics/tests/camera_tests.rs::optics_tp_006_samples_outside_the_image_are_refused` |
| OPTICS-TP-007 | Image orientation | OPTICS-REQ-002, OPTICS-REQ-003 | For a camera looking down `-Z` with up `+Y`, pixel `(0, 0)` points left and up, the last pixel points right and down, and moving one column or row changes the direction only along right or up. | `crates/bunny-optics/tests/camera_tests.rs::optics_tp_007_pixel_origin_is_top_left_and_rows_grow_downward` |
| OPTICS-TP-008 | Point at a distance | OPTICS-REQ-008 | Exact points for whole distances, ties rounded to even, an overflow refused, and a camera ray's triangle hit point within two raw units of the hit plane. | `crates/bunny-optics/tests/camera_tests.rs::optics_tp_008_point_at_rounds_and_refuses_overflow` |
| OPTICS-TP-009 | Deliberate mutations | OPTICS-REQ-004, OPTICS-REQ-005, OPTICS-REQ-006 | Each scaling or rounding rule, broken on purpose, makes at least one case above fail. | [Mutation checks](#mutation-checks) |
| OPTICS-TP-010 | Golden sample hash | OPTICS-REQ-010, OPTICS-REQ-012 | Raw bits and fractions of ten keys from the reference model: all zeros, one unit in each field, all ones, and keys adjacent in `x` and in the sample index; the identity string. | `crates/bunny-optics/tests/sampler_tests.rs::optics_tp_010_sample_hash_golden_vectors` |
| OPTICS-TP-011 | Fraction range | OPTICS-REQ-012 | `fraction_from_bits` maps 0 to 0, all ones to `1 - 2^-32`, drops the low 32 bits exactly; 4 096 seeded hash fractions and sampler points are in `[0, 1)`. | `crates/bunny-optics/tests/sampler_tests.rs::optics_tp_011_fractions_are_in_the_unit_interval` |
| OPTICS-TP-012 | Stratification property | OPTICS-REQ-013 | For 12 (seed, pixel, pair) keys (fixed extremes and seeded), blocks 0 and 3 of every size `2^k`, `k = 0..=10`, and the last block of 1 024 below `2^32`: each of the `k + 1` dyadic partitions has every cell hit exactly once. | `crates/bunny-optics/tests/sampler_tests.rs::optics_tp_012_power_of_two_blocks_are_stratified` |
| OPTICS-TP-013 | Golden sampler points | OPTICS-REQ-013, OPTICS-REQ-015 | Raw `(u, v)` of ten keys from the reference model, including samples 0 to 3, a step in each pixel coordinate and in the pair, all ones, and a mixed key in pairs 0 and 2; `camera_sample` equals pair 0; the identity string. | `crates/bunny-optics/tests/sampler_tests.rs::optics_tp_013_sampler_golden_vectors` |
| OPTICS-TP-014 | Evaluation order | OPTICS-REQ-014 | Bits and points of an 8 x 8 image, 16 samples and 4 dimensions computed in forward, reverse and seeded shuffled order are identical, and equal to values from reused handles. | `crates/bunny-optics/tests/sampler_tests.rs::optics_tp_014_values_do_not_depend_on_evaluation_order` |
| OPTICS-TP-015 | Domain separation | OPTICS-REQ-011 | `(x, y) = (1, 0)` and `(0, 1)` differ, as do swapped sample and dimension; 64 x 64 grids of pixels, of (sample, dimension) and of sampler pixels (including the top of each range) give 4 096 distinct values each; the sampler's first point differs from the hash of the same key. | `crates/bunny-optics/tests/sampler_tests.rs::optics_tp_015_key_fields_are_domain_separated` |
| OPTICS-TP-016 | Avalanche guard | OPTICS-REQ-010 | For each of the 192 key bits, flipping it changes between 28 and 36 of the 64 output bits on average over 256 seeded keys. A regression guard on the mixing, not a statistical test. | `crates/bunny-optics/tests/sampler_tests.rs::optics_tp_016_adjacent_keys_avalanche` |
| OPTICS-TP-017 | Decorrelation guard | OPTICS-REQ-013 | Pair 0 against pair 1 over 1 024 samples puts between 32 and 96 points in each cell of a 4 x 4 grid (64 expected), for three pixels; no sample of a 16 x 16 block shares its pair-0 point with the same sample of its right or lower neighbour. | `crates/bunny-optics/tests/sampler_tests.rs::optics_tp_017_dimension_pairs_and_pixels_are_decorrelated` |
| OPTICS-TP-018 | Camera samples | OPTICS-REQ-015 | Every one of 16 samples of every pixel of a 16 x 9 camera is a sample of that pixel and yields a ray. | `crates/bunny-optics/tests/sampler_tests.rs::optics_tp_018_camera_samples_are_accepted_by_the_camera` |
| OPTICS-TP-019 | Echo golden vectors | OPTICS-REQ-016 | Echo's `next_int` regression values and its `math-fixtures.json` `next_f32` and `next_int(5, 10)` values, exactly. | `crates/bunny-optics/tests/xoroshiro_tests.rs::optics_tp_019_echo_golden_vectors_replay` |
| OPTICS-TP-020 | Sequential seeding | OPTICS-REQ-016 | The published SplitMix64 outputs for seed 1234567 as the state of `from_seed_u64`; draws after both constructors and the zero-seed replacement from the reference model; one-value and empty ranges draw nothing. | `crates/bunny-optics/tests/xoroshiro_tests.rs::optics_tp_020_seeding_and_range_rules` |
| OPTICS-TP-021 | Sampler mutations | OPTICS-REQ-011, OPTICS-REQ-013 | Each key field and the stratification rule, broken on purpose, makes at least one case above fail. | [Mutation checks](#mutation-checks) |
| OPTICS-TP-022 | Golden accumulation | OPTICS-REQ-017, OPTICS-REQ-019 | Hand-computed sums and means for two, three, four and sixteen samples, including halves that round to the even neighbour. | `crates/bunny-optics/tests/pixel_tests.rs::optics_tp_022_golden_accumulation_and_mean` |
| OPTICS-TP-023 | Order independence | OPTICS-REQ-017 | 64 seeded pixels of 1 to 256 samples, large and tiny: forward, reverse, shuffled and both merge orders of a seeded split give identical accumulators, whose sums equal an independent `i128` sum. | `crates/bunny-optics/tests/pixel_tests.rs::optics_tp_023_accumulation_is_order_independent` |
| OPTICS-TP-024 | Accumulation limits | OPTICS-REQ-018, OPTICS-REQ-019 | 1024 samples at `2^52` sum to exactly `2^62` with mean `2^52`, also by merging two halves; the 1025th sample, an over-full merge, each out-of-range channel (with the first bad channel deciding the error) and an empty resolve are refused, leaving the accumulator unchanged. | `crates/bunny-optics/tests/pixel_tests.rs::optics_tp_024_accumulation_limits_are_exact_at_their_edges` |
| OPTICS-TP-025 | Mean rounding | OPTICS-REQ-019 | For every count 1 to 1024 over four seeded sample ranges, the mean equals an oracle that picks the nearer of the two neighbours (the even one on a tie) and, for power-of-two counts, a shift with the same rounding. | `crates/bunny-optics/tests/pixel_tests.rs::optics_tp_025_mean_rounds_to_nearest_with_ties_to_even` |
| OPTICS-TP-026 | sRGB knots | OPTICS-REQ-020 | Every knot at or above the toe end equals the `f64` reference value rounded to nearest; no knot is within `10^-4` raw units of a tie, so the `f64` oracle decides each one. | `crates/bunny-optics/tests/pixel_tests.rs::optics_tp_026_srgb_knots_are_the_correctly_rounded_curve` |
| OPTICS-TP-027 | Golden tone vectors | OPTICS-REQ-020, OPTICS-REQ-021, OPTICS-REQ-025 | Exact display values and 8-bit and 16-bit codes for both curves at 0, one raw unit, the toe end and the hold, 0.01, mid grey, 0.5, a mid-segment input, just below, at and above 1.0, the radiance cap and negative inputs; the identities. | `crates/bunny-optics/tests/pixel_tests.rs::optics_tp_027_golden_tone_vectors_lock_exact_outputs` |
| OPTICS-TP-028 | sRGB error bound | OPTICS-REQ-020 | Over every 4096th raw input from `-2^20` to `2^32 + 2^20`, every knot and its neighbours, every segment midpoint and every input within `2^16` of the toe end, the curve is within 6400 raw units of the `f64` reference, and the worst error exceeds 6200 (the bound is tight). | `crates/bunny-optics/tests/pixel_tests.rs::optics_tp_028_srgb_is_within_its_bound_of_the_exact_curve` |
| OPTICS-TP-029 | Codes against correct rounding | OPTICS-REQ-023 | For every 8-bit and 16-bit code, the first input reaching it (binary search over the monotone function) is compared with the first input whose `f64` reference reaches the code boundary: at most 9600 inputs later, at most 2 earlier, and never past the neighbouring codes' boundaries; the worst lag exceeds 9000. | `crates/bunny-optics/tests/pixel_tests.rs::optics_tp_029_codes_are_within_one_of_the_correctly_rounded_codes` |
| OPTICS-TP-030 | Monotonicity | OPTICS-REQ-020, OPTICS-REQ-021, OPTICS-REQ-022 | Display values, 8-bit and 16-bit codes of both curves never decrease over the dense inputs of OPTICS-TP-028, which include both sides of every place where curve pieces meet; quantization never decreases over every 4096th display value and every value within two raw units of each 8-bit and 16-bit boundary. | `crates/bunny-optics/tests/pixel_tests.rs::optics_tp_030_tone_curves_and_quantization_are_monotone` |
| OPTICS-TP-031 | Quantization rounding | OPTICS-REQ-022, OPTICS-REQ-025 | Golden codes on both sides of the first code boundaries, at 0.5 (the only exact tie, 127.5 and 32767.5, to even), at 1.0 and clamped outside `[0, 1]`; every code is the image of its own display value `c / 255` or `c / 65535`. | `crates/bunny-optics/tests/pixel_tests.rs::optics_tp_031_quantization_rounds_to_nearest_with_ties_to_even` |
| OPTICS-TP-032 | RGBA8 writer | OPTICS-REQ-024, OPTICS-REQ-025 | Golden bytes for both curves with alpha 255, one pixel written inside a row buffer, and lengths 0, 1, 3, 5 and 8 refused with the buffer unchanged. | `crates/bunny-optics/tests/pixel_tests.rs::optics_tp_032_rgba8_writes_four_opaque_bytes_or_refuses` |
| OPTICS-TP-033 | Pixel mutations | OPTICS-REQ-019, OPTICS-REQ-020, OPTICS-REQ-022 | Each rounding rule and the toe boundary, broken on purpose, makes at least one case above fail. | [Pixel mutation checks](#pixel-mutation-checks) |
| OPTICS-TP-034 | Pixel timing | OPTICS-REQ-019, OPTICS-REQ-020, OPTICS-REQ-024 | `pixel/resolve_srgb_rgba8` in `bunny-perf` times resolve, tone and RGBA8 output per pixel. | [Timing](#timing) |
| OPTICS-TP-040 | Golden surfaces | OPTICS-REQ-030, OPTICS-REQ-031, OPTICS-REQ-032 | Exact unit normals, tangents and bitangents for the six axis-aligned faces, a triangle with one-raw-unit edges, and (model) a diagonal, a domain-corner, a sliver and a downward-tilted triangle; the flipped ground frame; collinear, duplicate and all-equal vertices return None. | `crates/bunny-optics/tests/shading_tests.rs::optics_tp_040_golden_surfaces_lock_normals_and_frames` |
| OPTICS-TP-041 | Normal property | OPTICS-REQ-030 | 4 000 seeded triangles at scales from one raw unit to the whole domain (at least 3 900 checked): each normal component within one raw unit of the floating-point unit vector of the exact i128 cross product; exact zero area gives None. | `crates/bunny-optics/tests/shading_property_tests.rs::optics_tp_041_normals_are_within_one_raw_unit_of_the_exact_unit_normal` |
| OPTICS-TP-042 | Frame property | OPTICS-REQ-031 | 4 000 seeded and special unit normals (the axes, z exactly 0, z one raw unit either side of 0): exact i128 residuals of every frame and its flipped form within 8 raw units, and each tangent and bitangent component within half a raw unit of the formula, checked as an exact rational inequality. | `crates/bunny-optics/tests/shading_property_tests.rs::optics_tp_042_frames_are_orthonormal_and_follow_the_stated_formula` |
| OPTICS-TP-043 | Golden directions | OPTICS-REQ-034 | Seven sample points (0, the centre, 3/4, 1 - 2^-32, eighth turns) on the +Z frame (hand computed), the downward and flipped frames (permutations of it) and (model) a diagonal and a tilted frame; points outside [0, 1) return None. | `crates/bunny-optics/tests/shading_tests.rs::optics_tp_043_golden_directions_lock_the_hemisphere_map` |
| OPTICS-TP-044 | Hemisphere property | OPTICS-REQ-034 | 600 seeded and special frames, both orientations, 16 sampler points and 36 edge points (u, v in 0, 2^-32, 1/4, 1/2, 1 - 2^-16, 1 - 2^-32) each: components within 1.0, exact dot with the normal at least 2^15 raw units, length within 8 raw units, each component within 4 raw units of the floating-point ideal map. | `crates/bunny-optics/tests/shading_property_tests.rs::optics_tp_044_bounce_directions_are_above_the_surface_and_near_the_ideal_map` |
| OPTICS-TP-045 | Distribution guard | OPTICS-REQ-035 | For the +Z frame and four seeded frames: the mean cosine of 4 096 sampler points within 2^-12 of 2/3; the cosine bands of 1 024 points within one of 64, 192, 320 and 448; each azimuth quarter within two of 256. A regression guard with stated thresholds, not a statistical test. | `crates/bunny-optics/tests/shading_property_tests.rs::optics_tp_045_directions_follow_the_cosine_distribution` |
| OPTICS-TP-046 | Golden surface hits | OPTICS-REQ-032, OPTICS-REQ-033, OPTICS-REQ-039 | Start points and facing normals for front and back hits on the ground (each stepped once off the plane), an oblique ground hit, and (model) diagonal and tilted hits that need no step; a start point stepped past the top of the trace domain returns None. | `crates/bunny-optics/tests/shading_tests.rs::optics_tp_046_golden_surface_hits_lock_facing_and_start_points` |
| OPTICS-TP-047 | Self-intersection scenes | OPTICS-REQ-033, OPTICS-REQ-038 | Camera rays at points exactly on and within 16 raw units of the shared edge of a square ground, of a tilted coplanar pair, of a coincident duplicate with opposite winding, and of a convex ridge (at least 700 hits): no bounce ray (8 sampler points each) and no shadow ray (three lights, at most SHADOW_T_MAX) hits any triangle of the scene. | `crates/bunny-optics/tests/shading_property_tests.rs::optics_tp_047_secondary_rays_never_hit_their_own_or_a_coplanar_surface` |
| OPTICS-TP-048 | Golden light terms | OPTICS-REQ-036, OPTICS-REQ-038 | PI and FRAC_1_PI raw values; cosines and reflected radiance (hand computed straight above, the rest model) for lights straight above, at 45 degrees, in the consumer's example direction, grazing at 2^-20, along and below the surface, on the ground and diagonal normals; the shadow ray's origin and direction; no ray and no light along or below the plane. | `crates/bunny-optics/tests/shading_tests.rs::optics_tp_048_golden_light_terms_lock_the_radiometric_convention` |
| OPTICS-TP-049 | Cosine property | OPTICS-REQ-036 | 400 seeded normalized light directions against 400 seeded and special unit normals: the cosine within 3 raw units of the clamped floating-point cosine. | `crates/bunny-optics/tests/shading_property_tests.rs::optics_tp_049_light_cosines_are_within_three_raw_units` |
| OPTICS-TP-050 | Energy sanity | OPTICS-REQ-036, OPTICS-REQ-037 | A white ground under a uniform sky: 1 024 bounce rays all escape and the mean reflected radiance equals the sky exactly; the same sky as 1 024 directional lights over equal-solid-angle bands, each of irradiance L * 2 pi / 1024, reflects L within 2^-16 relative (a missing or extra pi is off by a factor of pi). | `crates/bunny-optics/tests/shading_property_tests.rs::optics_tp_050_white_surfaces_conserve_energy` |
| OPTICS-TP-051 | Shadow limit | OPTICS-REQ-038 | From one face of the trace domain to the opposite face, axis-aligned and oblique: the occluder is within SHADOW_T_MAX and within 4 units of it, and half the limit misses it. | `crates/bunny-optics/tests/shading_property_tests.rs::optics_tp_051_the_shadow_limit_covers_the_trace_domain` |
| OPTICS-TP-052 | Refusals | OPTICS-REQ-037, OPTICS-REQ-039 | Light directions with a largest component one raw unit either side of 1.0, zero or 0.5; negative and too-large irradiance and sky radiance (and both: negative first); albedo one raw unit outside 0..=1; reflection ties to even and an unrepresentable sum refused. | `crates/bunny-optics/tests/shading_tests.rs::optics_tp_052_lights_and_albedo_outside_the_domain_are_refused` |
| OPTICS-TP-053 | Composed paths | OPTICS-REQ-036, OPTICS-REQ-037, OPTICS-REQ-040 | The identity string; radiance bits of a path lit from straight above whose bounce sees the sky (hand computed, including two ties to even), and (model) a path whose bounce hits a lit wall, the same path with the sun blocked by an occluder, and a camera ray that sees the sky. | `crates/bunny-optics/tests/shading_tests.rs::optics_tp_053_composed_paths_lock_the_order_of_operations` |
| OPTICS-TP-054 | Nearby surfaces | OPTICS-REQ-033 | A cover 2^-20 units above the ground shadows a ground point beside it through a shadow ray at slope 2^-4, which a distance epsilon of 2^-10 would miss; a cover 4 raw units above the ground shadows the point under it; both start points stay below the cover. | `crates/bunny-optics/tests/shading_property_tests.rs::optics_tp_054_nearby_surfaces_are_not_skipped` |
| OPTICS-TP-055 | Shading mutations | OPTICS-REQ-032, OPTICS-REQ-033, OPTICS-REQ-034, OPTICS-REQ-036, OPTICS-REQ-038 | The facing rule, the hemisphere map, the convention's pi, the self-intersection rule and the light's arrival-side test, each broken on purpose, make at least one case above fail. | [Shading mutation checks](#shading-mutation-checks) |
| OPTICS-TP-056 | Shading timing | OPTICS-REQ-033, OPTICS-REQ-034 | `shade/surface_new`, `shade/bounce_direction` and `shade/hit_shading` in `bunny-perf` time the per-triangle and per-hit steps. | [Timing](#timing) |
| OPTICS-TP-057 | Unrepresentable hit point | OPTICS-REQ-041, OPTICS-REQ-043 | A real ground hit with its distance replaced by the most negative Q32.32 value: `-1.0 * distance` overflows, so `try_new` returns `HitPointUnrepresentable` with that distance, its `Display` text names the raw value, and `new` returns None. | `crates/bunny-optics/tests/shading_refusal_tests.rs::optics_tp_057_an_unrepresentable_hit_point_names_its_distance` |
| OPTICS-TP-058 | Hit point beyond the domain | OPTICS-REQ-041, OPTICS-REQ-042, OPTICS-REQ-043 | A real ground hit with its distance replaced by -1 024 units and by -2^30 units: the point is 1 026 and 2^30 + 2 units above the ground, more than two raw units outside the domain, so `try_new` returns `HitPointOutsideTraceDomain` with that point (hand computed) and `new` returns None; at 2^30 units the 0.7.1 plane test needed 2^134 and overflowed `i128`. | `crates/bunny-optics/tests/shading_refusal_tests.rs::optics_tp_058_a_hit_point_beyond_the_domain_is_refused_before_the_plane_test` |
| OPTICS-TP-059 | Hit not on the arrival side | OPTICS-REQ-041, OPTICS-REQ-043 | A real ground hit from 2 units above with its face flipped and its distance set to zero: the point is the ray origin, 2 units on the wrong side, so `try_new` returns `NotOnArrivalSide` with that point and `new` returns None. | `crates/bunny-optics/tests/shading_refusal_tests.rs::optics_tp_059_a_hit_off_the_arrival_side_names_its_point` |
| OPTICS-TP-060 | Start point outside the domain | OPTICS-REQ-041, OPTICS-REQ-043 | The edge of OPTICS-TP-046: the ray arrives exactly at `y = 2^40 - 1` raw and the one step goes to `y = 2^40`, so `try_new` returns `StartOutsideTraceDomain` with start `(0, 2^40, 0)` raw (hand computed) and `new` returns None. | `crates/bunny-optics/tests/shading_refusal_tests.rs::optics_tp_060_a_start_point_outside_the_domain_names_the_stepped_point` |
| OPTICS-TP-061 | Sample outside the unit interval | OPTICS-REQ-041, OPTICS-REQ-043 | `u` or `v` of 1.0, `-2^-32` and the extreme raw values: `try_cosine_direction` and `try_bounce_ray` return `SampleOutsideUnitInterval` with the sample as given, and the `Option` forms return None. | `crates/bunny-optics/tests/shading_refusal_tests.rs::optics_tp_061_a_sample_outside_the_unit_interval_is_named` |
| OPTICS-TP-062 | Fast and diagnostic paths agree | OPTICS-REQ-041, OPTICS-REQ-042, OPTICS-REQ-043 | Seeded triangles and rays, each hit also given seeded distances (full raw range, large positive and negative, near either end of the raw range, within two raw units of the domain's boundary on the ray's dominant axis, a few raw units from the reported one) and the other face, and seeded samples inside and outside `[0, 1)`: the 0.7.1 code retained in the test (`tests/shading_reference/mod.rs`) gives the same start point, frame, direction and bounce ray as the `Option` form, the fast (`Option`) form is `Some` exactly when the diagnostic (`try_`) form is `Ok` with the same value, and every `Err` is the variant and values the reference names, except that a hit point beyond reach of the domain is expected as `HitPointOutsideTraceDomain` (where the 0.7.1 plane test overflows there, the reference checks the arithmetic instead). Witness counts require every reachable outcome, and at least ten overflowing inputs, to occur and `BounceRayRefused` never to. The seed and case are printed on failure. | `crates/bunny-optics/tests/shading_refusal_tests.rs::optics_tp_062_option_and_result_forms_agree_with_the_released_code` |
| OPTICS-TP-063 | Refusal mutations | OPTICS-REQ-041, OPTICS-REQ-042, OPTICS-REQ-043 | Each side of the fast/diagnostic pair broken alone (the stepped-point domain check of either, the overflow guard of the fast path, the beyond-reach check of the diagnostic path), the stepped start point reported as the unstepped one, and a sample coordinate of exactly 1.0 accepted, each make at least one case above fail. | [Shading mutation checks](#shading-mutation-checks) |
| OPTICS-TP-064 | Agreement from a fresh seed | OPTICS-REQ-041, OPTICS-REQ-042, OPTICS-REQ-043 | The check of OPTICS-TP-062 from a seed drawn per run (`RandomState`), printed in every failure message, so coverage accumulates across runs; 30 consecutive runs passed. On `wasm32-unknown-unknown` the standard library's hash keys are fixed, so there the seed repeats. | `crates/bunny-optics/tests/shading_refusal_tests.rs::optics_tp_064_option_and_result_forms_agree_from_a_fresh_seed` |

```toml
[[case]]
id = "OPTICS-TP-001"
requirements = ["OPTICS-REQ-002"]
evidence = "test"
test = "crates/bunny-optics/tests/camera_tests.rs::optics_tp_001_golden_basis_is_right_handed"
oracle = "Exact raw forward, right and up for cameras looking down -Z, along +X and along +Y, and for an up hint tilted toward the viewer; right equals forward x up exactly for each."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-002"
requirements = ["OPTICS-REQ-003", "OPTICS-REQ-005", "OPTICS-REQ-006", "OPTICS-REQ-009"]
evidence = "test"
test = "crates/bunny-optics/tests/camera_tests.rs::optics_tp_002_golden_rays_lock_raw_outputs"
oracle = "Hand-computed raw directions and scale shifts for five cameras, including a corner sample exactly at 1.0, a 2:1 image that needs one halving, a 3x3 image whose directions are floored thirds, and a moved camera along +X."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-003"
requirements = ["OPTICS-REQ-002", "OPTICS-REQ-004", "OPTICS-REQ-005"]
evidence = "property"
test = "crates/bunny-optics/tests/camera_tests.rs::optics_tp_003_rays_pass_through_the_intended_image_plane_point"
oracle = "The exact integer image-plane oracle over 2 000 seeded camera specifications (at least 1 990 must construct) and four samples each; the basis is orthogonal to within 2^-28, right-handed and on the up hint's side."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-004"
requirements = ["OPTICS-REQ-004", "OPTICS-REQ-005", "OPTICS-REQ-006"]
evidence = "test"
test = "crates/bunny-optics/tests/camera_tests.rs::optics_tp_004_corner_samples_of_extreme_cameras_stay_in_the_trace_domain"
oracle = "At the widest accepted tangent, the largest image, the most extreme aspect ratios, the narrowest tangent and a tangent just above 1.0, with axis-aligned and diagonal look directions, every corner pixel at sub-pixel offsets 0 and 1 - 2^-32 yields a ray with a positive forward component that satisfies the image-plane oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-005"
requirements = ["OPTICS-REQ-001"]
evidence = "test"
test = "crates/bunny-optics/tests/camera_tests.rs::optics_tp_005_degenerate_cameras_are_refused"
oracle = "Each degenerate input returns its error; values at each accepted bound construct and values one raw unit or one pixel past it are refused; the shortest and longest look and up vectors construct."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-006"
requirements = ["OPTICS-REQ-007"]
evidence = "test"
test = "crates/bunny-optics/tests/camera_tests.rs::optics_tp_006_samples_outside_the_image_are_refused"
oracle = "Columns and rows at the image size, sub-pixel offsets of 1.0 and below zero return None; offsets 0 and 1 - 2^-32 return rays."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-007"
requirements = ["OPTICS-REQ-002", "OPTICS-REQ-003"]
evidence = "test"
test = "crates/bunny-optics/tests/camera_tests.rs::optics_tp_007_pixel_origin_is_top_left_and_rows_grow_downward"
oracle = "For a camera looking down -Z with up +Y, pixel (0, 0) points left and up, the last pixel points right and down, and moving one column or row changes the direction only along right or up."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-008"
requirements = ["OPTICS-REQ-008"]
evidence = "test"
test = "crates/bunny-optics/tests/camera_tests.rs::optics_tp_008_point_at_rounds_and_refuses_overflow"
oracle = "Exact points for whole distances, ties rounded to even, an overflow refused, and a camera ray's triangle hit point within two raw units of the hit plane."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-009"
requirements = ["OPTICS-REQ-004", "OPTICS-REQ-005", "OPTICS-REQ-006"]
evidence = "manual-audit"
artifact = "docs/topics/optics/test-plan.md#mutation-checks"
oracle = "Each scaling or rounding rule, broken on purpose, makes at least one implemented case fail."
tier = "manual"
status = "implemented"

[[case]]
id = "OPTICS-TP-010"
requirements = ["OPTICS-REQ-010", "OPTICS-REQ-012"]
evidence = "test"
test = "crates/bunny-optics/tests/sampler_tests.rs::optics_tp_010_sample_hash_golden_vectors"
oracle = "Raw bits and fractions of ten keys from the reference model: all zeros, one unit in each field, all ones, and keys adjacent in x and in the sample index; the identity string."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-011"
requirements = ["OPTICS-REQ-012"]
evidence = "test"
test = "crates/bunny-optics/tests/sampler_tests.rs::optics_tp_011_fractions_are_in_the_unit_interval"
oracle = "fraction_from_bits maps 0 to 0, all ones to 1 - 2^-32, drops the low 32 bits exactly; 4 096 seeded hash fractions and sampler points are in [0, 1)."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-012"
requirements = ["OPTICS-REQ-013"]
evidence = "property"
test = "crates/bunny-optics/tests/sampler_tests.rs::optics_tp_012_power_of_two_blocks_are_stratified"
oracle = "For 12 (seed, pixel, pair) keys (fixed extremes and seeded), blocks 0 and 3 of every size 2^k, k = 0..=10, and the last block of 1 024 below 2^32: each of the k + 1 dyadic partitions has every cell hit exactly once."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-013"
requirements = ["OPTICS-REQ-013", "OPTICS-REQ-015"]
evidence = "test"
test = "crates/bunny-optics/tests/sampler_tests.rs::optics_tp_013_sampler_golden_vectors"
oracle = "Raw (u, v) of ten keys from the reference model, including samples 0 to 3, a step in each pixel coordinate and in the pair, all ones, and a mixed key in pairs 0 and 2; camera_sample equals pair 0; the identity string."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-014"
requirements = ["OPTICS-REQ-014"]
evidence = "property"
test = "crates/bunny-optics/tests/sampler_tests.rs::optics_tp_014_values_do_not_depend_on_evaluation_order"
oracle = "Bits and points of an 8 x 8 image, 16 samples and 4 dimensions computed in forward, reverse and seeded shuffled order are identical, and equal to values from reused handles."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-015"
requirements = ["OPTICS-REQ-011"]
evidence = "property"
test = "crates/bunny-optics/tests/sampler_tests.rs::optics_tp_015_key_fields_are_domain_separated"
oracle = "(x, y) = (1, 0) and (0, 1) differ, as do swapped sample and dimension; 64 x 64 grids of pixels, of (sample, dimension) and of sampler pixels (including the top of each range) give 4 096 distinct values each; the sampler's first point differs from the hash of the same key."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-016"
requirements = ["OPTICS-REQ-010"]
evidence = "property"
test = "crates/bunny-optics/tests/sampler_tests.rs::optics_tp_016_adjacent_keys_avalanche"
oracle = "For each of the 192 key bits, flipping it changes between 28 and 36 of the 64 output bits on average over 256 seeded keys. A regression guard on the mixing, not a statistical test."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-017"
requirements = ["OPTICS-REQ-013"]
evidence = "property"
test = "crates/bunny-optics/tests/sampler_tests.rs::optics_tp_017_dimension_pairs_and_pixels_are_decorrelated"
oracle = "Pair 0 against pair 1 over 1 024 samples puts between 32 and 96 points in each cell of a 4 x 4 grid (64 expected), for three pixels; no sample of a 16 x 16 block shares its pair-0 point with the same sample of its right or lower neighbour."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-018"
requirements = ["OPTICS-REQ-015"]
evidence = "test"
test = "crates/bunny-optics/tests/sampler_tests.rs::optics_tp_018_camera_samples_are_accepted_by_the_camera"
oracle = "Every one of 16 samples of every pixel of a 16 x 9 camera is a sample of that pixel and yields a ray."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-019"
requirements = ["OPTICS-REQ-016"]
evidence = "test"
test = "crates/bunny-optics/tests/xoroshiro_tests.rs::optics_tp_019_echo_golden_vectors_replay"
oracle = "Echo's next_int regression values and its math-fixtures.json next_f32 and next_int(5, 10) values, exactly."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-020"
requirements = ["OPTICS-REQ-016"]
evidence = "test"
test = "crates/bunny-optics/tests/xoroshiro_tests.rs::optics_tp_020_seeding_and_range_rules"
oracle = "The published SplitMix64 outputs for seed 1234567 as the state of from_seed_u64; draws after both constructors and the zero-seed replacement from the reference model; one-value and empty ranges draw nothing."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-021"
requirements = ["OPTICS-REQ-011", "OPTICS-REQ-013"]
evidence = "manual-audit"
artifact = "docs/topics/optics/test-plan.md#mutation-checks"
oracle = "Each key field and the stratification rule, broken on purpose, makes at least one implemented case fail."
tier = "manual"
status = "implemented"

[[case]]
id = "OPTICS-TP-022"
requirements = ["OPTICS-REQ-017", "OPTICS-REQ-019"]
evidence = "test"
test = "crates/bunny-optics/tests/pixel_tests.rs::optics_tp_022_golden_accumulation_and_mean"
oracle = "Hand-computed sums and means for two, three, four and sixteen samples, including halves that round to the even neighbour."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-023"
requirements = ["OPTICS-REQ-017"]
evidence = "property"
test = "crates/bunny-optics/tests/pixel_tests.rs::optics_tp_023_accumulation_is_order_independent"
oracle = "64 seeded pixels of 1 to 256 samples, large and tiny: forward, reverse, shuffled and both merge orders of a seeded split give identical accumulators, whose sums equal an independent i128 sum."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-024"
requirements = ["OPTICS-REQ-018", "OPTICS-REQ-019"]
evidence = "test"
test = "crates/bunny-optics/tests/pixel_tests.rs::optics_tp_024_accumulation_limits_are_exact_at_their_edges"
oracle = "1024 samples at 2^52 sum to exactly 2^62 with mean 2^52, also by merging two halves; the 1025th sample, an over-full merge, each out-of-range channel (with the first bad channel deciding the error) and an empty resolve are refused, leaving the accumulator unchanged."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-025"
requirements = ["OPTICS-REQ-019"]
evidence = "property"
test = "crates/bunny-optics/tests/pixel_tests.rs::optics_tp_025_mean_rounds_to_nearest_with_ties_to_even"
oracle = "For every count 1 to 1024 over four seeded sample ranges, the mean equals an oracle that picks the nearer of the two neighbours (the even one on a tie) and, for power-of-two counts, a shift with the same rounding."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-026"
requirements = ["OPTICS-REQ-020"]
evidence = "test"
test = "crates/bunny-optics/tests/pixel_tests.rs::optics_tp_026_srgb_knots_are_the_correctly_rounded_curve"
oracle = "Every knot at or above the toe end equals the f64 reference value rounded to nearest; no knot is within 10^-4 raw units of a tie, so the f64 oracle decides each one."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-027"
requirements = ["OPTICS-REQ-020", "OPTICS-REQ-021", "OPTICS-REQ-025"]
evidence = "test"
test = "crates/bunny-optics/tests/pixel_tests.rs::optics_tp_027_golden_tone_vectors_lock_exact_outputs"
oracle = "Exact display values and 8-bit and 16-bit codes for both curves at 0, one raw unit, the toe end and the hold, 0.01, mid grey, 0.5, a mid-segment input, just below, at and above 1.0, the radiance cap and negative inputs; the identities."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-028"
requirements = ["OPTICS-REQ-020"]
evidence = "property"
test = "crates/bunny-optics/tests/pixel_tests.rs::optics_tp_028_srgb_is_within_its_bound_of_the_exact_curve"
oracle = "Over every 4096th raw input from -2^20 to 2^32 + 2^20, every knot and its neighbours, every segment midpoint and every input within 2^16 of the toe end, the curve is within 6400 raw units of the f64 reference, and the worst error exceeds 6200 (the bound is tight)."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-029"
requirements = ["OPTICS-REQ-023"]
evidence = "property"
test = "crates/bunny-optics/tests/pixel_tests.rs::optics_tp_029_codes_are_within_one_of_the_correctly_rounded_codes"
oracle = "For every 8-bit and 16-bit code, the first input reaching it (binary search over the monotone function) is compared with the first input whose f64 reference reaches the code boundary: at most 9600 inputs later, at most 2 earlier, and never past the neighbouring codes' boundaries; the worst lag exceeds 9000."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-030"
requirements = ["OPTICS-REQ-020", "OPTICS-REQ-021", "OPTICS-REQ-022"]
evidence = "property"
test = "crates/bunny-optics/tests/pixel_tests.rs::optics_tp_030_tone_curves_and_quantization_are_monotone"
oracle = "Display values, 8-bit and 16-bit codes of both curves never decrease over the dense inputs of OPTICS-TP-028, which include both sides of every place where curve pieces meet; quantization never decreases over every 4096th display value and every value within two raw units of each 8-bit and 16-bit boundary."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-031"
requirements = ["OPTICS-REQ-022", "OPTICS-REQ-025"]
evidence = "test"
test = "crates/bunny-optics/tests/pixel_tests.rs::optics_tp_031_quantization_rounds_to_nearest_with_ties_to_even"
oracle = "Golden codes on both sides of the first code boundaries, at 0.5 (the only exact tie, 127.5 and 32767.5, to even), at 1.0 and clamped outside [0, 1]; every code is the image of its own display value c / 255 or c / 65535."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-032"
requirements = ["OPTICS-REQ-024", "OPTICS-REQ-025"]
evidence = "test"
test = "crates/bunny-optics/tests/pixel_tests.rs::optics_tp_032_rgba8_writes_four_opaque_bytes_or_refuses"
oracle = "Golden bytes for both curves with alpha 255, one pixel written inside a row buffer, and lengths 0, 1, 3, 5 and 8 refused with the buffer unchanged."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-033"
requirements = ["OPTICS-REQ-019", "OPTICS-REQ-020", "OPTICS-REQ-022"]
evidence = "manual-audit"
artifact = "docs/topics/optics/test-plan.md#pixel-mutation-checks"
oracle = "Each rounding rule and the toe boundary, broken on purpose, makes at least one implemented pixel case fail."
tier = "manual"
status = "implemented"

[[case]]
id = "OPTICS-TP-034"
requirements = ["OPTICS-REQ-019", "OPTICS-REQ-020", "OPTICS-REQ-024"]
evidence = "benchmark"
artifact = "docs/topics/optics/test-plan.md#timing"
oracle = "pixel/resolve_srgb_rgba8 in bunny-perf times resolve, tone and RGBA8 output for 1024 seeded pixels of 16 samples."
tier = "bench"
status = "implemented"

[[case]]
id = "OPTICS-TP-040"
requirements = ["OPTICS-REQ-030", "OPTICS-REQ-031", "OPTICS-REQ-032"]
evidence = "test"
test = "crates/bunny-optics/tests/shading_tests.rs::optics_tp_040_golden_surfaces_lock_normals_and_frames"
oracle = "Exact unit normals, tangents and bitangents for the six axis-aligned faces, a triangle with one-raw-unit edges, and (model) a diagonal, a domain-corner, a sliver and a downward-tilted triangle; the flipped ground frame; collinear, duplicate and all-equal vertices return None."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-041"
requirements = ["OPTICS-REQ-030"]
evidence = "property"
test = "crates/bunny-optics/tests/shading_property_tests.rs::optics_tp_041_normals_are_within_one_raw_unit_of_the_exact_unit_normal"
oracle = "4 000 seeded triangles at scales from one raw unit to the whole domain (at least 3 900 checked): each normal component within one raw unit of the floating-point unit vector of the exact i128 cross product; exact zero area gives None."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-042"
requirements = ["OPTICS-REQ-031"]
evidence = "property"
test = "crates/bunny-optics/tests/shading_property_tests.rs::optics_tp_042_frames_are_orthonormal_and_follow_the_stated_formula"
oracle = "4 000 seeded and special unit normals (the axes, z exactly 0, z one raw unit either side of 0): exact i128 residuals of every frame and its flipped form within 8 raw units, and each tangent and bitangent component within half a raw unit of the formula, checked as an exact rational inequality."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-043"
requirements = ["OPTICS-REQ-034"]
evidence = "test"
test = "crates/bunny-optics/tests/shading_tests.rs::optics_tp_043_golden_directions_lock_the_hemisphere_map"
oracle = "Seven sample points (0, the centre, 3/4, 1 - 2^-32, eighth turns) on the +Z frame (hand computed), the downward and flipped frames (permutations of it) and (model) a diagonal and a tilted frame; points outside [0, 1) return None."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-044"
requirements = ["OPTICS-REQ-034"]
evidence = "property"
test = "crates/bunny-optics/tests/shading_property_tests.rs::optics_tp_044_bounce_directions_are_above_the_surface_and_near_the_ideal_map"
oracle = "600 seeded and special frames, both orientations, 16 sampler points and 36 edge points (u, v in 0, 2^-32, 1/4, 1/2, 1 - 2^-16, 1 - 2^-32) each: components within 1.0, exact dot with the normal at least 2^15 raw units, length within 8 raw units, each component within 4 raw units of the floating-point ideal map."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-045"
requirements = ["OPTICS-REQ-035"]
evidence = "property"
test = "crates/bunny-optics/tests/shading_property_tests.rs::optics_tp_045_directions_follow_the_cosine_distribution"
oracle = "For the +Z frame and four seeded frames: the mean cosine of 4 096 sampler points within 2^-12 of 2/3; the cosine bands of 1 024 points within one of 64, 192, 320 and 448; each azimuth quarter within two of 256. A regression guard with stated thresholds, not a statistical test."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-046"
requirements = ["OPTICS-REQ-032", "OPTICS-REQ-033", "OPTICS-REQ-039"]
evidence = "test"
test = "crates/bunny-optics/tests/shading_tests.rs::optics_tp_046_golden_surface_hits_lock_facing_and_start_points"
oracle = "Start points and facing normals for front and back hits on the ground (each stepped once off the plane), an oblique ground hit, and (model) diagonal and tilted hits that need no step; a start point stepped past the top of the trace domain returns None."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-047"
requirements = ["OPTICS-REQ-033", "OPTICS-REQ-038"]
evidence = "property"
test = "crates/bunny-optics/tests/shading_property_tests.rs::optics_tp_047_secondary_rays_never_hit_their_own_or_a_coplanar_surface"
oracle = "Camera rays at points exactly on and within 16 raw units of the shared edge of a square ground, of a tilted coplanar pair, of a coincident duplicate with opposite winding, and of a convex ridge (at least 700 hits): no bounce ray (8 sampler points each) and no shadow ray (three lights, at most SHADOW_T_MAX) hits any triangle of the scene."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-048"
requirements = ["OPTICS-REQ-036", "OPTICS-REQ-038"]
evidence = "test"
test = "crates/bunny-optics/tests/shading_tests.rs::optics_tp_048_golden_light_terms_lock_the_radiometric_convention"
oracle = "PI and FRAC_1_PI raw values; cosines and reflected radiance (hand computed straight above, the rest model) for lights straight above, at 45 degrees, in the consumer's example direction, grazing at 2^-20, along and below the surface, on the ground and diagonal normals; the shadow ray's origin and direction; no ray and no light along or below the plane."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-049"
requirements = ["OPTICS-REQ-036"]
evidence = "property"
test = "crates/bunny-optics/tests/shading_property_tests.rs::optics_tp_049_light_cosines_are_within_three_raw_units"
oracle = "400 seeded normalized light directions against 400 seeded and special unit normals: the cosine within 3 raw units of the clamped floating-point cosine."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-050"
requirements = ["OPTICS-REQ-036", "OPTICS-REQ-037"]
evidence = "property"
test = "crates/bunny-optics/tests/shading_property_tests.rs::optics_tp_050_white_surfaces_conserve_energy"
oracle = "A white ground under a uniform sky: 1 024 bounce rays all escape and the mean reflected radiance equals the sky exactly; the same sky as 1 024 directional lights over equal-solid-angle bands, each of irradiance L * 2 pi / 1024, reflects L within 2^-16 relative (a missing or extra pi is off by a factor of pi)."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-051"
requirements = ["OPTICS-REQ-038"]
evidence = "test"
test = "crates/bunny-optics/tests/shading_property_tests.rs::optics_tp_051_the_shadow_limit_covers_the_trace_domain"
oracle = "From one face of the trace domain to the opposite face, axis-aligned and oblique: the occluder is within SHADOW_T_MAX and within 4 units of it, and half the limit misses it."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-052"
requirements = ["OPTICS-REQ-037", "OPTICS-REQ-039"]
evidence = "test"
test = "crates/bunny-optics/tests/shading_tests.rs::optics_tp_052_lights_and_albedo_outside_the_domain_are_refused"
oracle = "Light directions with a largest component one raw unit either side of 1.0, zero or 0.5; negative and too-large irradiance and sky radiance (and both: negative first); albedo one raw unit outside 0..=1; reflection ties to even and an unrepresentable sum refused."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-053"
requirements = ["OPTICS-REQ-036", "OPTICS-REQ-037", "OPTICS-REQ-040"]
evidence = "test"
test = "crates/bunny-optics/tests/shading_tests.rs::optics_tp_053_composed_paths_lock_the_order_of_operations"
oracle = "The identity string; radiance bits of a path lit from straight above whose bounce sees the sky (hand computed, including two ties to even), and (model) a path whose bounce hits a lit wall, the same path with the sun blocked by an occluder, and a camera ray that sees the sky."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-054"
requirements = ["OPTICS-REQ-033"]
evidence = "test"
test = "crates/bunny-optics/tests/shading_property_tests.rs::optics_tp_054_nearby_surfaces_are_not_skipped"
oracle = "A cover 2^-20 units above the ground shadows a ground point beside it through a shadow ray at slope 2^-4, which a distance epsilon of 2^-10 would miss; a cover 4 raw units above the ground shadows the point under it; both start points stay below the cover."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-055"
requirements = ["OPTICS-REQ-032", "OPTICS-REQ-033", "OPTICS-REQ-034", "OPTICS-REQ-036", "OPTICS-REQ-038"]
evidence = "manual-audit"
artifact = "docs/topics/optics/test-plan.md#shading-mutation-checks"
oracle = "The facing rule, the hemisphere map, the convention's pi, the self-intersection rule and the light's arrival-side test, each broken on purpose, make at least one implemented case fail."
tier = "manual"
status = "implemented"

[[case]]
id = "OPTICS-TP-056"
requirements = ["OPTICS-REQ-033", "OPTICS-REQ-034"]
evidence = "manual-audit"
artifact = "docs/topics/optics/test-plan.md#timing"
oracle = "shade/surface_new, shade/bounce_direction and shade/hit_shading in bunny-perf time the per-triangle and per-hit shading steps; a reading, not a speed claim."
tier = "manual"
status = "implemented"

[[case]]
id = "OPTICS-TP-057"
requirements = ["OPTICS-REQ-041", "OPTICS-REQ-043"]
evidence = "test"
test = "crates/bunny-optics/tests/shading_refusal_tests.rs::optics_tp_057_an_unrepresentable_hit_point_names_its_distance"
oracle = "Exact construction: -1.0 times the most negative raw distance is not representable, so the variant and its distance are known in advance; the Display text is specified."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-058"
requirements = ["OPTICS-REQ-041", "OPTICS-REQ-042", "OPTICS-REQ-043"]
evidence = "test"
test = "crates/bunny-optics/tests/shading_refusal_tests.rs::optics_tp_058_a_hit_point_beyond_the_domain_is_refused_before_the_plane_test"
oracle = "Hand computed: the ray straight down from 2 units above the ground, taken 1 024 and 2^30 units backward, gives the points (0, 1 026, 0) and (0, 2^30 + 2, 0) units, both beyond the domain; the second makes the 0.7.1 plane test exceed i128."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-059"
requirements = ["OPTICS-REQ-041", "OPTICS-REQ-043"]
evidence = "test"
test = "crates/bunny-optics/tests/shading_refusal_tests.rs::optics_tp_059_a_hit_off_the_arrival_side_names_its_point"
oracle = "Exact construction: at distance zero the point is the ray origin, 2 units on the side the flipped face points away from, which two raw-unit steps cannot cross."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-060"
requirements = ["OPTICS-REQ-041", "OPTICS-REQ-043"]
evidence = "test"
test = "crates/bunny-optics/tests/shading_refusal_tests.rs::optics_tp_060_a_start_point_outside_the_domain_names_the_stepped_point"
oracle = "Hand computed: the ray along y = 2^40 - 1 raw arrives exactly on the plane 2x + 4y = 4(2^40 - 1) at x = 0, and one step up the dominant y axis gives (0, 2^40, 0)."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-061"
requirements = ["OPTICS-REQ-041", "OPTICS-REQ-043"]
evidence = "test"
test = "crates/bunny-optics/tests/shading_refusal_tests.rs::optics_tp_061_a_sample_outside_the_unit_interval_is_named"
oracle = "Specified: coordinates of 1.0, -2^-32, i64::MIN and i64::MAX raw are outside [0, 1), so the variant carries the sample as given."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-062"
requirements = ["OPTICS-REQ-041", "OPTICS-REQ-042", "OPTICS-REQ-043"]
evidence = "test"
test = "crates/bunny-optics/tests/shading_refusal_tests.rs::optics_tp_062_option_and_result_forms_agree_with_the_released_code"
oracle = "Cross-version differential: the 0.7.1 code of SurfaceHit::new and ShadingFrame::cosine_direction, retained in the test over the public API, names the expected result and refusal cause of every generated input; witness counts show each reachable outcome occurs."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-063"
requirements = ["OPTICS-REQ-041", "OPTICS-REQ-042", "OPTICS-REQ-043"]
evidence = "manual-audit"
artifact = "docs/topics/optics/test-plan.md#shading-mutation-checks"
oracle = "Each side of the fast/diagnostic pair broken alone, the stepped start point reported as the unstepped one, and a sample coordinate of exactly 1.0 accepted, each make at least one implemented case fail."
tier = "manual"
status = "implemented"

[[case]]
id = "OPTICS-TP-064"
requirements = ["OPTICS-REQ-041", "OPTICS-REQ-042", "OPTICS-REQ-043"]
evidence = "test"
test = "crates/bunny-optics/tests/shading_refusal_tests.rs::optics_tp_064_option_and_result_forms_agree_from_a_fresh_seed"
oracle = "The cross-version differential and fast/diagnostic agreement of OPTICS-TP-062, from a per-run seed printed on failure."
tier = "fast"
status = "implemented"
```

## Mutation Checks

Each rule below was broken on purpose in a container copy of
`crates/bunny-optics/src/plane.rs`, and the camera tests were run
(`cargo test --locked -p bunny-optics --test camera_tests`). Every mutation
made at least one case fail; the restored source passed all eight.

| Mutation in `plane.rs` | Rule broken | Failing cases |
| --- | --- | --- |
| Scale search limited to `k = 0` (`.take(1)`) | The scaling rule: some cameras need a halving | OPTICS-TP-001 to OPTICS-TP-008 (construction refuses the shared fixtures) |
| Corners at pixel centres (`+-(width - 1)`, `+-(height - 1)`) instead of the rectangle corners | The corners checked must bound every sample | OPTICS-TP-004, OPTICS-TP-006 |
| Direction limit made exclusive (`< 1.0`) | The limit is inclusive of 1.0, as in the trace domain | OPTICS-TP-002, OPTICS-TP-007 |
| Final floor replaced by rounding to nearest | Every rounding in ray generation is a floor | OPTICS-TP-002, OPTICS-TP-003, OPTICS-TP-004 |

The pixel-centre mutation is caught by the camera with tangent `1 + 2^-20`
on a 1024-pixel square image in `OPTICS-TP-004`: its pixel-centre corners fit
at `k = 0`, but a sample at the very corner of the image has a component of
`-(1 + 2^-20)`, which `TraceRay::new` refuses. That case was added for this
reason.

### Sampler Mutations

Each rule below was broken on purpose in a container copy of
`crates/bunny-optics/src/hash.rs` or `sampler.rs`, and the sampler and
generator tests were run (`cargo test --locked -p bunny-optics --test
sampler_tests --test xoroshiro_tests`). Every mutation made at least one case
fail; the restored source passed all eleven.

| Mutation | Rule broken | Failing cases |
| --- | --- | --- |
| `y` dropped from the pixel word (`pack(x, y & 0)`) | Every key field reaches the hash | OPTICS-TP-010, OPTICS-TP-013, OPTICS-TP-015, OPTICS-TP-016, OPTICS-TP-017 |
| `dimension` dropped from the sample word | Every key field reaches the hash | OPTICS-TP-010, OPTICS-TP-015, OPTICS-TP-016 |
| `pair` dropped from the sampler's pair seed | Each dimension pair has its own scrambles | OPTICS-TP-013, OPTICS-TP-017 |
| Second Sobol dimension made van der Corput (`direction >>= 1`) | The two coordinates form a (0, 2)-sequence | OPTICS-TP-012, OPTICS-TP-013 |
| First Laine-Karras constant made odd (`0x6C50B47D`) | The scramble is nested | OPTICS-TP-012, OPTICS-TP-013, OPTICS-TP-017 |
| Index shuffle replaced by an odd multiply and add | The shuffle maps aligned blocks onto aligned blocks | OPTICS-TP-012, OPTICS-TP-013, OPTICS-TP-017 |
| `mix64` replaced by the identity | The hash mixes | OPTICS-TP-010, OPTICS-TP-013, OPTICS-TP-016 |

## Pixel Mutation Checks

Each rule below was broken on purpose in a container copy of
`crates/bunny-optics/src/pixel/`, and the pixel tests were run
(`cargo test --locked -p bunny-optics --test pixel_tests`, with lints capped
to warnings so an unused helper does not stop the build). Every mutation made
at least one case fail; the restored source passed all eleven.

| Mutation | Rule broken | Failing cases |
| --- | --- | --- |
| Mean floored (`sum / count`) | The mean rounds to nearest | OPTICS-TP-022, OPTICS-TP-025 |
| Mean ties rounded up (`(sum + count / 2) / count`) | Ties go to the even neighbour | OPTICS-TP-022, OPTICS-TP-025 |
| Quantization floored | Codes round to nearest | OPTICS-TP-027, OPTICS-TP-029, OPTICS-TP-031, OPTICS-TP-032 |
| Quantization ties rounded down | The tie at 0.5 goes to the even code | OPTICS-TP-027, OPTICS-TP-031, OPTICS-TP-032 |
| Toe used for one more input (`<=` at `TOE_END`) | The toe/power boundary | OPTICS-TP-027, OPTICS-TP-030 |
| Hold at the toe end removed | The curve is monotone where the pieces meet | OPTICS-TP-027, OPTICS-TP-030 |
| Toe slope 12.93 instead of 12.92 | The toe is `12.92 x` | OPTICS-TP-027, OPTICS-TP-028, OPTICS-TP-029, OPTICS-TP-030 |
| Interpolation rounded to nearest instead of floored | The interpolation floors | OPTICS-TP-027 |

The last mutation stays within the stated bounds, so only the golden vectors
see it: a version is defined by its bits, not by its accuracy.

## Shading Mutation Checks

Each rule below was broken on purpose in a container copy of
`crates/bunny-optics/src/shading.rs` or a file under `src/shading/`, and the
shading tests were run (`timeout 120 cargo test --locked -p bunny-optics
--test shading_tests --test shading_property_tests`). Every mutation made at
least one case fail; the restored source passed all fifteen.

| Mutation | Rule broken | Failing cases |
| --- | --- | --- |
| The hit's frame chosen for the other face (`facing_frame(!hit.front_face)`) | The facing rule | OPTICS-TP-046, OPTICS-TP-047, OPTICS-TP-048, OPTICS-TP-050, OPTICS-TP-053 |
| Uniform instead of cosine-weighted hemisphere (`cos theta = 1 - u`, radius `sqrt(u (2 - u))`) | The hemisphere map | OPTICS-TP-043, OPTICS-TP-044, OPTICS-TP-045 |
| `FRAC_1_PI` replaced by 1.0 | The radiometric convention's pi | OPTICS-TP-048, OPTICS-TP-050, OPTICS-TP-053 |
| No step off the plane (`MAX_STEPS = 0`) | The self-intersection rule | OPTICS-TP-046, OPTICS-TP-047, OPTICS-TP-048, OPTICS-TP-050, OPTICS-TP-053, OPTICS-TP-054 |
| The light's arrival-side test made inclusive (`< 0` for `<= 0`) | No shadow ray for a light along the surface | OPTICS-TP-048 |

Without the step, start points on the plane make `SurfaceHit::new` return
`None`, and the cases that build hits fail on that. The two variants below
accept a start point on the plane instead, which is what a rule with no step
and no epsilon does: bounce and shadow rays then hit their own triangle or a
coplanar neighbour at distance zero.

| Mutation | Rule broken | Failing cases |
| --- | --- | --- |
| A start point on the plane accepted (`>= offset` for `> offset`) | Strictly on the arrival side | OPTICS-TP-046, OPTICS-TP-047, OPTICS-TP-048, OPTICS-TP-050, OPTICS-TP-053, OPTICS-TP-054 |
| No step and a start point on the plane accepted | Both | OPTICS-TP-046, OPTICS-TP-047, OPTICS-TP-048, OPTICS-TP-050, OPTICS-TP-053, OPTICS-TP-054 |

The typed refusals (`OPTICS-TP-063`) were checked the same way, with `timeout
120 cargo test --locked -p bunny-optics --no-fail-fast --test
shading_property_tests --test shading_refusal_tests --test shading_tests` per
mutation; the restored source passed every case. The first four break one
side of the fast/diagnostic pair alone, which the agreement cases must catch.

| Mutation | Rule broken | Failing cases |
| --- | --- | --- |
| Fast path: a stepped point one raw unit past the domain accepted | Fast and diagnostic paths agree | OPTICS-TP-046, OPTICS-TP-060, OPTICS-TP-062, OPTICS-TP-064 |
| Fast path: the overflow guard removed (the 0.7.1 body) | The plane test stays inside `i128` | OPTICS-TP-058, OPTICS-TP-062, OPTICS-TP-064 (each panics: "attempt to multiply with overflow") |
| Diagnostic path: a stepped point one raw unit past the domain accepted | Fast and diagnostic paths agree | OPTICS-TP-060, OPTICS-TP-062, OPTICS-TP-064 |
| Diagnostic path: the beyond-reach check removed | Far points are named before the plane test | OPTICS-TP-058, OPTICS-TP-062, OPTICS-TP-064 |
| `StartOutsideTraceDomain` carries the unstepped hit point | The variant names the stepped start point | OPTICS-TP-060 |
| A sample coordinate of exactly 1.0 accepted (`<=` for `<`) | Samples are in `[0, 1)` | OPTICS-TP-043, OPTICS-TP-061, OPTICS-TP-062, OPTICS-TP-064 |

The second row is the 0.7.1 failure itself: its plane test overflows `i128` on
a point `2^30` units above the ground (a panic in a debug build; release
builds return `None`).

## Timing

`pixel/resolve_srgb_rgba8` in `crates/bunny-perf` (feature group `pixel`)
resolves 1 024 seeded pixels of 16 samples each (linear radiance up to 2.0 per
channel), applies `SrgbV1` and writes each pixel's RGBA8 bytes; every result
passes through `black_box`. Until a base revision has the pixel functions,
`cargo xtask perf` cannot compare it (with `--filter pixel/` it reports that no
case was measured on both sides), so run the harness directly:

```sh
cargo run --locked --release -p bunny-perf -- --filter pixel/
```

Once a revision with this case is the base, the usual comparison applies:

```sh
cargo run --locked -p xtask -- perf --base <rev> --rounds 11 --align-code --filter pixel/
```

The readings are for future comparisons, not a speed claim, so the case has no
ledger entry in the [performance chapter](../performance/README.md).

The `shade` group (`crates/bunny-perf/src/shade.rs`) times the shading steps
over 1 024 seeded triangles within 8 units of the origin, each hit by a ray
toward its centroid, with one sampler point (pair 1) per hit:
`shade/surface_new` builds a `TriangleSurface` (the per-triangle work),
`shade/bounce_direction` maps a sampler point with a held frame, and
`shade/hit_shading` runs the whole per-hit step (`SurfaceHit::new`, the light
sample with its shadow ray, the bounce ray and the reflection of direct plus
sky). Traversal is not included. Every result passes through `black_box`. Run
it with `--filter shade/` as above; the readings are in the
[chapter](README.md#accuracy-and-cost), not a speed claim.

## Determinism Obligations And Evidence

The camera, the sample hash, the sampler, the sequential generator, the pixel
functions and their tests use only integer arithmetic: `i64` and `i128`
additions, multiplications, shifts and floor or rounded divisions, and
wrapping `u64` and `u32` arithmetic and bit reversal, all with defined
results. There is no floating point, trigonometry, square root per ray,
randomness from the host, time, filesystem access, parallel scheduling, map
iteration, stdout, stderr or logging in the library or in any camera, hash or
sampler oracle. The pixel tests use `f64` only as the sRGB reference oracle
described under [Fixtures](#the-srgb-reference-oracle); every value they lock
is an integer. The square roots in construction are those of
`FixedUnitVec3::new`, an exact integer square root.

Test builds check integer overflow, so `OPTICS-TP-004` turns any overflow at
the corners of the accepted domain into a test failure. The hash and sampler
use explicit wrapping operations, so the checks do not apply to them; their
golden vectors run natively and under WebAssembly, which locks the wrapping
results on both.

`OPTICS-TP-014` is the evaluation-order evidence: the values are pure
functions, so no thread-count test is needed beyond showing that order and
handle reuse do not matter. No statistical test suite (PractRand, TestU01) has
been run; `OPTICS-TP-016` and `OPTICS-TP-017` are regression guards with
stated thresholds, not statistical certification.

The shading functions use only `i128` additions, multiplications, shifts and
divisions with stated rounding, the exact integer square root and the
table-based `FixedAngle::sin_cos`. Their property cases use floating point as
an oracle only, in the test files. `OPTICS-TP-045` is a regression guard on
the hemisphere distribution with stated thresholds, not a statistical test;
`OPTICS-TP-050` checks energy on two constructed scenes, not convergence in
general. Every shading case runs natively and under WebAssembly.

## Known Failures

The current executable surface has no known failing cases.

## Edge Cases And Unusual Inputs

Current tests cover:

- a corner sample whose direction component is exactly 1.0;
- a 1x1 image and images of one row or one column at the largest width;
- the narrowest tangent (one raw unit) and the widest (256.0);
- a tangent just above 1.0, where pixel-centre corners fit but rectangle
  corners do not;
- look directions along an axis and along a diagonal;
- an up hint that is zero, equal to, opposite to or a multiple of the look
  direction, and one tilted toward the viewer;
- look and up vectors one raw unit long and as long as Q32.32 allows;
- up hints just inside and just outside the near-parallel threshold;
- positions one raw unit inside and outside the trace domain;
- sub-pixel offsets of 0, `1 - 2^-32`, 1.0 and below zero;
- hash and sampler keys of all zeros and all ones, keys one unit apart in each
  field, swapped pixel coordinates and swapped sample and dimension;
- the last block of 1 024 samples below `2^32`;
- `fraction_from_bits` of 0, all ones, and values on either side of a step of
  the top 32 bits;
- the all-zero seed of `Xoroshiro128Plus`, one-value and empty ranges;
- sample channels of zero, `2^52`, `2^52 + 1`, `-1`, `i64::MIN` and
  `i64::MAX`, and a full pixel of 1 024 samples at `2^52`;
- an empty pixel and an over-full merge;
- tone inputs at zero, one raw unit, both sides of the toe end, inside the
  hold, at every knot and its neighbours, at 1.0 and one raw unit on either
  side, the radiance cap and the most negative value;
- display values on both sides of the first 8-bit and 16-bit boundaries, the
  exact tie at 0.5, and values outside `[0, 1]`;
- output slices of lengths 0, 1, 3, 5 and 8;
- triangles with one-raw-unit edges, a 255-unit sliver three raw units wide,
  and one spanning the corners of the trace domain; collinear, duplicate and
  all-equal vertices;
- unit normals with `z` exactly 0 and one raw unit either side of it (the
  frame's sign switch), and the six axes;
- sample points `u` of 0, `2^-32`, `1 - 2^-16` and `1 - 2^-32` (the lowest
  bounce, `2^-16` above the surface), and `v` at every eighth of a turn;
- hits exactly on a shared edge and a shared vertex, on a coincident duplicate
  with opposite winding, on a convex ridge, and a start point stepped past the
  top of the trace domain;
- lights straight above, at 45 degrees, grazing at `2^-20` and `2^-32` above
  the surface, along it and below it; a cover `2^-20` units and 4 raw units
  above the surface;
- light directions whose largest component is one raw unit either side of 1.0.
- hit distances of the most negative raw value, zero with the face flipped,
  large positive and negative values and a few raw units from the reported
  distance; sample coordinates of exactly 1.0, `-2^-32` and the extreme raw
  values.

## Stress And Fuzz

No fuzz target exists today. `OPTICS-TP-003` is a seeded property case over
2 000 cameras; `OPTICS-TP-023` and `OPTICS-TP-025` are seeded property cases
over accumulators, and `OPTICS-TP-028` to `OPTICS-TP-030` use dense
deterministic inputs rather than random ones.

Every minimized fuzz failure must become a committed deterministic regression
case with a stable test name.

## Open Gaps

| Gap | Blocking API |
| --- | --- |
| A differential check that points along a camera ray project, through `FixedMat4::try_look_at`, `FixedMat4::try_perspective` and `Viewport`, back to the ray's sample. | None; a follow-up case (the matrices landed while this chapter was written). |
| A statistical test suite (PractRand or TestU01) over the sample hash and the sampler. | None; it needs a harness outside the core crates. |
| Stratification evidence above 1 024 samples per pixel (the property is proven for every `k` up to 32 and tested to 10). | None; a slow-tier case. |
| Timing of ray generation. | A `bunny-perf` case once a renderer loop exists. |
| An exhaustive sweep of all `2^32` inputs in `[0, 1]` for the tone curves (the 60-digit model found every code threshold once, which decides every input of a monotone function; the test suite covers the structure densely instead). | None; too slow for a debug-build test, a candidate for a `slow` tier. |
| Shrinking for `OPTICS-TP-062` and `OPTICS-TP-064`: a failure reports its seed, case and variant, not a reduced input. | None; a property-testing harness or a hand-written reducer. |
| A constructed case for `SecondaryRayError::BounceRayRefused`: no input is known to reach it (the start point is inside the domain and every bounce direction is a valid trace direction), and `OPTICS-TP-062` checks that none of its generated inputs does. | None; a constructed case if one is found. |
| A start point needing two steps off the plane: the proof allows two, the seeded cases and a 400 000-hit search in the reference model needed at most one. | None; an exact constructed case if one is found. |
| The hemisphere distribution beyond the stated guard (for example a chi-squared test over many cells, or convergence of a full renderer against an independent one). | None; a slow-tier case or a renderer-level comparison. |
