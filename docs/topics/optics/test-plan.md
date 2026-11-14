# Optics Test Plan

This document defines how Bunny verifies the [optics contract](README.md): the
pinhole camera in `bunny-optics`, the rays it generates, and the pixel-value
functions that turn radiance samples into output codes.

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
- `PixelAccumulator`: exact, order-independent sums, their limits and the
  mean's rounding;
- `ToneCurve` (`SrgbV1`, `LinearClampV1`): golden vectors, the sRGB error
  bound against a reference oracle, and monotonicity;
- `quantize_8`, `quantize_16` and the codes they give against the correctly
  rounded sRGB codes;
- `write_rgba8` and the versioned identities.

Projection matrices (flyingrobots/bunny#201, #202), sampling sequences (#221)
and shading (#160) are out of scope.

## Test Goals

- Lock raw ray outputs with golden vectors for hand-checkable cameras:
  axis-aligned, power-of-two tangents and image sizes, plus one inexact case.
- Check every generated ray against an exact integer oracle of the intended
  image-plane point, over seeded cameras and at the extremes of the accepted
  domain.
- Show that the scaling rule is load bearing by breaking it on purpose.
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
| OPTICS-REQ-010 | `PixelAccumulator` sums each channel exactly in `i64`: the sums and the count do not depend on the order samples are added in or on how they are split between accumulators that are then merged. | `README.md#accumulation` |
| OPTICS-REQ-011 | A sample channel is accepted in `0..=2^52` raw and a pixel holds at most 1024 samples, so no sum exceeds `2^62`; a negative channel, a channel above `2^52` raw, a sample or merge past 1024 samples each return their own `PixelError` and leave the accumulator unchanged. | `README.md#accumulation` |
| OPTICS-REQ-012 | `resolve` returns each sum divided by the count, rounded to nearest with ties to even (for a power-of-two count, the same rounding of a right shift), and `NoSamples` for an empty pixel. | `README.md#resolve` |
| OPTICS-REQ-013 | `ToneCurve::SrgbV1` is 0 at or below zero, 1.0 at or above 1.0, `round(12.92 x)` below the first raw input at or above 0.0031308, and above it the floored linear interpolation of 1153 correctly rounded knots of `1.055 x^(1/2.4) - 0.055`, held at the toe's last value until it rises past it; it is within 6400 raw units (1.49e-6) of the exact sRGB curve and monotone over every input. | `README.md#tone-curves` |
| OPTICS-REQ-014 | `ToneCurve::LinearClampV1` clamps its input to `[0, 1]` and is otherwise the identity. | `README.md#tone-curves` |
| OPTICS-REQ-015 | `quantize_8` and `quantize_16` clamp a display value to `[0, 1]` and return `round(255 v)` and `round(65535 v)`, to nearest with ties to even; both are monotone. | `README.md#output-codes` |
| OPTICS-REQ-016 | Under `SrgbV1`, every 8-bit and 16-bit code is within one of the correctly rounded sRGB code of the same input; each code begins at most 9600 raw inputs after and at most 2 raw inputs before the input where the correctly rounded code begins. | `README.md#accuracy-of-the-codes` |
| OPTICS-REQ-017 | `write_rgba8` writes red, green and blue 8-bit codes and alpha 255 into a slice of exactly four bytes, and returns `WrongSliceLength` without writing for any other length. | `README.md#output-codes` |
| OPTICS-REQ-018 | Tone curves and output encodings have stable versioned identities (`bunny-tone-srgb/1`, `bunny-tone-linear-clamp/1`, `bunny-unorm8/1`, `bunny-unorm16/1`, `bunny-rgba8-opaque/1`); golden vectors lock their outputs, and any change to an output is a new version. | `README.md#versioned-identities` |

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
summary = "PixelAccumulator sums each channel exactly in i64: the sums and the count do not depend on the order samples are added in or on how they are split between accumulators that are then merged."
status = "active"

[[requirement]]
id = "OPTICS-REQ-011"
summary = "A sample channel is accepted in 0..=2^52 raw and a pixel holds at most 1024 samples, so no sum exceeds 2^62; a negative channel, a channel above 2^52 raw, a sample or merge past 1024 samples each return their own PixelError and leave the accumulator unchanged."
status = "active"

[[requirement]]
id = "OPTICS-REQ-012"
summary = "resolve returns each sum divided by the count, rounded to nearest with ties to even (for a power-of-two count, the same rounding of a right shift), and NoSamples for an empty pixel."
status = "active"

[[requirement]]
id = "OPTICS-REQ-013"
summary = "ToneCurve::SrgbV1 is 0 at or below zero, 1.0 at or above 1.0, round(12.92 x) below the first raw input at or above 0.0031308, and above it the floored linear interpolation of 1153 correctly rounded knots of 1.055 x^(1/2.4) - 0.055, held at the toe's last value until it rises past it; it is within 6400 raw units (1.49e-6) of the exact sRGB curve and monotone over every input."
status = "active"

[[requirement]]
id = "OPTICS-REQ-014"
summary = "ToneCurve::LinearClampV1 clamps its input to [0, 1] and is otherwise the identity."
status = "active"

[[requirement]]
id = "OPTICS-REQ-015"
summary = "quantize_8 and quantize_16 clamp a display value to [0, 1] and return round(255 v) and round(65535 v), to nearest with ties to even; both are monotone."
status = "active"

[[requirement]]
id = "OPTICS-REQ-016"
summary = "Under SrgbV1, every 8-bit and 16-bit code is within one of the correctly rounded sRGB code of the same input; each code begins at most 9600 raw inputs after and at most 2 raw inputs before the input where the correctly rounded code begins."
status = "active"

[[requirement]]
id = "OPTICS-REQ-017"
summary = "write_rgba8 writes red, green and blue 8-bit codes and alpha 255 into a slice of exactly four bytes, and returns WrongSliceLength without writing for any other length."
status = "active"

[[requirement]]
id = "OPTICS-REQ-018"
summary = "Tone curves and output encodings have stable versioned identities (bunny-tone-srgb/1, bunny-tone-linear-clamp/1, bunny-unorm8/1, bunny-unorm16/1, bunny-rgba8-opaque/1); golden vectors lock their outputs, and any change to an output is a new version."
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

The pixel cases (`crates/bunny-optics/tests/pixel_tests.rs`) use the same
generator with the seed `PIXEL_SEED` (`0xB002_0971_C500_0222`). Their golden
tone vectors come from an independent integer model of the curve whose knots
were computed with 60-digit decimal arithmetic (in a scratch directory, not
committed); every knot is also rechecked in `OPTICS-TP-014`.

### The sRGB Reference Oracle

`CODE_STANDARDS.md` Rule 2 bans floating point in canonical math, and the
Code Dojo float rule applies to core crates only, so the pixel test file uses
`f64` as a reference oracle, never the library. The oracle is the IEC
61966-2-1 formula, `12.92 x` below 0.0031308 and `1.055 x^(1/2.4) - 0.055`
above, on `x = raw / 2^32`, which is exact in `f64` for every input tested.

`f64` is close enough to decide each comparison the cases make:

- `powf` is accurate to about `10^-6` raw Q32.32 units of display value; the
  nearest knot to a rounding tie is `2.7 * 10^-4` raw units away, and
  `OPTICS-TP-014` asserts a margin of `10^-4`;
- where a code boundary is compared, one raw input changes the reference by
  at least `10^-6` codes, and on both sides of every 8-bit and 16-bit
  boundary the 60-digit model found the reference at least `5 * 10^-11` codes
  away from it, against an `f64` error near `10^-11` codes;
- the reference curve itself drops by `2.9 * 10^-8` at the toe end (the
  standard's constants do not meet), but no 8-bit or 16-bit code boundary lies
  in that drop, so each code still begins at one well-defined input.

The asserted bounds (`9600` and `2` inputs in `OPTICS-TP-017`) leave a margin
over the values the 60-digit model measured (9 447 and 9 522 late, 1 early) so
that a one-input `f64` misjudgement cannot fail the case.

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
| OPTICS-TP-010 | Golden accumulation | OPTICS-REQ-010, OPTICS-REQ-012 | Hand-computed sums and means for two, three, four and sixteen samples, including halves that round to the even neighbour. | `crates/bunny-optics/tests/pixel_tests.rs::optics_tp_010_golden_accumulation_and_mean` |
| OPTICS-TP-011 | Order independence | OPTICS-REQ-010 | 64 seeded pixels of 1 to 256 samples, large and tiny: forward, reverse, shuffled and both merge orders of a seeded split give identical accumulators, whose sums equal an independent `i128` sum. | `crates/bunny-optics/tests/pixel_tests.rs::optics_tp_011_accumulation_is_order_independent` |
| OPTICS-TP-012 | Accumulation limits | OPTICS-REQ-011, OPTICS-REQ-012 | 1024 samples at `2^52` sum to exactly `2^62` with mean `2^52`, also by merging two halves; the 1025th sample, an over-full merge, each out-of-range channel (with the first bad channel deciding the error) and an empty resolve are refused, leaving the accumulator unchanged. | `crates/bunny-optics/tests/pixel_tests.rs::optics_tp_012_accumulation_limits_are_exact_at_their_edges` |
| OPTICS-TP-013 | Mean rounding | OPTICS-REQ-012 | For every count 1 to 1024 over four seeded sample ranges, the mean equals an oracle that picks the nearer of the two neighbours (the even one on a tie) and, for power-of-two counts, a shift with the same rounding. | `crates/bunny-optics/tests/pixel_tests.rs::optics_tp_013_mean_rounds_to_nearest_with_ties_to_even` |
| OPTICS-TP-014 | sRGB knots | OPTICS-REQ-013 | Every knot at or above the toe end equals the `f64` reference value rounded to nearest; no knot is within `10^-4` raw units of a tie, so the `f64` oracle decides each one. | `crates/bunny-optics/tests/pixel_tests.rs::optics_tp_014_srgb_knots_are_the_correctly_rounded_curve` |
| OPTICS-TP-015 | Golden tone vectors | OPTICS-REQ-013, OPTICS-REQ-014, OPTICS-REQ-018 | Exact display values and 8-bit and 16-bit codes for both curves at 0, one raw unit, the toe end and the hold, 0.01, mid grey, 0.5, a mid-segment input, just below, at and above 1.0, the radiance cap and negative inputs; the identities. | `crates/bunny-optics/tests/pixel_tests.rs::optics_tp_015_golden_tone_vectors_lock_exact_outputs` |
| OPTICS-TP-016 | sRGB error bound | OPTICS-REQ-013 | Over every 4096th raw input from `-2^20` to `2^32 + 2^20`, every knot and its neighbours, every segment midpoint and every input within `2^16` of the toe end, the curve is within 6400 raw units of the `f64` reference, and the worst error exceeds 6200 (the bound is tight). | `crates/bunny-optics/tests/pixel_tests.rs::optics_tp_016_srgb_is_within_its_bound_of_the_exact_curve` |
| OPTICS-TP-017 | Codes against correct rounding | OPTICS-REQ-016 | For every 8-bit and 16-bit code, the first input reaching it (binary search over the monotone function) is compared with the first input whose `f64` reference reaches the code boundary: at most 9600 inputs later, at most 2 earlier, and never past the neighbouring codes' boundaries; the worst lag exceeds 9000. | `crates/bunny-optics/tests/pixel_tests.rs::optics_tp_017_codes_are_within_one_of_the_correctly_rounded_codes` |
| OPTICS-TP-018 | Monotonicity | OPTICS-REQ-013, OPTICS-REQ-014, OPTICS-REQ-015 | Display values, 8-bit and 16-bit codes of both curves never decrease over the dense inputs of OPTICS-TP-016, which include both sides of every place where curve pieces meet; quantization never decreases over every 4096th display value and every value within two raw units of each 8-bit and 16-bit boundary. | `crates/bunny-optics/tests/pixel_tests.rs::optics_tp_018_tone_curves_and_quantization_are_monotone` |
| OPTICS-TP-019 | Quantization rounding | OPTICS-REQ-015, OPTICS-REQ-018 | Golden codes on both sides of the first code boundaries, at 0.5 (the only exact tie, 127.5 and 32767.5, to even), at 1.0 and clamped outside `[0, 1]`; every code is the image of its own display value `c / 255` or `c / 65535`. | `crates/bunny-optics/tests/pixel_tests.rs::optics_tp_019_quantization_rounds_to_nearest_with_ties_to_even` |
| OPTICS-TP-020 | RGBA8 writer | OPTICS-REQ-017, OPTICS-REQ-018 | Golden bytes for both curves with alpha 255, one pixel written inside a row buffer, and lengths 0, 1, 3, 5 and 8 refused with the buffer unchanged. | `crates/bunny-optics/tests/pixel_tests.rs::optics_tp_020_rgba8_writes_four_opaque_bytes_or_refuses` |
| OPTICS-TP-021 | Pixel mutations | OPTICS-REQ-012, OPTICS-REQ-013, OPTICS-REQ-015 | Each rounding rule and the toe boundary, broken on purpose, makes at least one case above fail. | [Pixel mutation checks](#pixel-mutation-checks) |
| OPTICS-TP-022 | Pixel timing | OPTICS-REQ-012, OPTICS-REQ-013, OPTICS-REQ-017 | `pixel/resolve_srgb_rgba8` in `bunny-perf` times resolve, tone and RGBA8 output per pixel. | [Timing](#timing) |

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
test = "crates/bunny-optics/tests/pixel_tests.rs::optics_tp_010_golden_accumulation_and_mean"
oracle = "Hand-computed sums and means for two, three, four and sixteen samples, including halves that round to the even neighbour."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-011"
requirements = ["OPTICS-REQ-010"]
evidence = "property"
test = "crates/bunny-optics/tests/pixel_tests.rs::optics_tp_011_accumulation_is_order_independent"
oracle = "64 seeded pixels of 1 to 256 samples, large and tiny: forward, reverse, shuffled and both merge orders of a seeded split give identical accumulators, whose sums equal an independent i128 sum."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-012"
requirements = ["OPTICS-REQ-011", "OPTICS-REQ-012"]
evidence = "test"
test = "crates/bunny-optics/tests/pixel_tests.rs::optics_tp_012_accumulation_limits_are_exact_at_their_edges"
oracle = "1024 samples at 2^52 sum to exactly 2^62 with mean 2^52, also by merging two halves; the 1025th sample, an over-full merge, each out-of-range channel (with the first bad channel deciding the error) and an empty resolve are refused, leaving the accumulator unchanged."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-013"
requirements = ["OPTICS-REQ-012"]
evidence = "property"
test = "crates/bunny-optics/tests/pixel_tests.rs::optics_tp_013_mean_rounds_to_nearest_with_ties_to_even"
oracle = "For every count 1 to 1024 over four seeded sample ranges, the mean equals an oracle that picks the nearer of the two neighbours (the even one on a tie) and, for power-of-two counts, a shift with the same rounding."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-014"
requirements = ["OPTICS-REQ-013"]
evidence = "test"
test = "crates/bunny-optics/tests/pixel_tests.rs::optics_tp_014_srgb_knots_are_the_correctly_rounded_curve"
oracle = "Every knot at or above the toe end equals the f64 reference value rounded to nearest; no knot is within 10^-4 raw units of a tie, so the f64 oracle decides each one."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-015"
requirements = ["OPTICS-REQ-013", "OPTICS-REQ-014", "OPTICS-REQ-018"]
evidence = "test"
test = "crates/bunny-optics/tests/pixel_tests.rs::optics_tp_015_golden_tone_vectors_lock_exact_outputs"
oracle = "Exact display values and 8-bit and 16-bit codes for both curves at 0, one raw unit, the toe end and the hold, 0.01, mid grey, 0.5, a mid-segment input, just below, at and above 1.0, the radiance cap and negative inputs; the identities."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-016"
requirements = ["OPTICS-REQ-013"]
evidence = "property"
test = "crates/bunny-optics/tests/pixel_tests.rs::optics_tp_016_srgb_is_within_its_bound_of_the_exact_curve"
oracle = "Over every 4096th raw input from -2^20 to 2^32 + 2^20, every knot and its neighbours, every segment midpoint and every input within 2^16 of the toe end, the curve is within 6400 raw units of the f64 reference, and the worst error exceeds 6200 (the bound is tight)."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-017"
requirements = ["OPTICS-REQ-016"]
evidence = "property"
test = "crates/bunny-optics/tests/pixel_tests.rs::optics_tp_017_codes_are_within_one_of_the_correctly_rounded_codes"
oracle = "For every 8-bit and 16-bit code, the first input reaching it (binary search over the monotone function) is compared with the first input whose f64 reference reaches the code boundary: at most 9600 inputs later, at most 2 earlier, and never past the neighbouring codes' boundaries; the worst lag exceeds 9000."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-018"
requirements = ["OPTICS-REQ-013", "OPTICS-REQ-014", "OPTICS-REQ-015"]
evidence = "property"
test = "crates/bunny-optics/tests/pixel_tests.rs::optics_tp_018_tone_curves_and_quantization_are_monotone"
oracle = "Display values, 8-bit and 16-bit codes of both curves never decrease over the dense inputs of OPTICS-TP-016, which include both sides of every place where curve pieces meet; quantization never decreases over every 4096th display value and every value within two raw units of each 8-bit and 16-bit boundary."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-019"
requirements = ["OPTICS-REQ-015", "OPTICS-REQ-018"]
evidence = "test"
test = "crates/bunny-optics/tests/pixel_tests.rs::optics_tp_019_quantization_rounds_to_nearest_with_ties_to_even"
oracle = "Golden codes on both sides of the first code boundaries, at 0.5 (the only exact tie, 127.5 and 32767.5, to even), at 1.0 and clamped outside [0, 1]; every code is the image of its own display value c / 255 or c / 65535."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-020"
requirements = ["OPTICS-REQ-017", "OPTICS-REQ-018"]
evidence = "test"
test = "crates/bunny-optics/tests/pixel_tests.rs::optics_tp_020_rgba8_writes_four_opaque_bytes_or_refuses"
oracle = "Golden bytes for both curves with alpha 255, one pixel written inside a row buffer, and lengths 0, 1, 3, 5 and 8 refused with the buffer unchanged."
tier = "fast"
status = "implemented"

[[case]]
id = "OPTICS-TP-021"
requirements = ["OPTICS-REQ-012", "OPTICS-REQ-013", "OPTICS-REQ-015"]
evidence = "manual-audit"
artifact = "docs/topics/optics/test-plan.md#pixel-mutation-checks"
oracle = "Each rounding rule and the toe boundary, broken on purpose, makes at least one implemented pixel case fail."
tier = "manual"
status = "implemented"

[[case]]
id = "OPTICS-TP-022"
requirements = ["OPTICS-REQ-012", "OPTICS-REQ-013", "OPTICS-REQ-017"]
evidence = "benchmark"
artifact = "docs/topics/optics/test-plan.md#timing"
oracle = "pixel/resolve_srgb_rgba8 in bunny-perf times resolve, tone and RGBA8 output for 1024 seeded pixels of 16 samples."
tier = "bench"
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

## Pixel Mutation Checks

Each rule below was broken on purpose in a container copy of
`crates/bunny-optics/src/pixel/`, and the pixel tests were run
(`cargo test --locked -p bunny-optics --test pixel_tests`, with lints capped
to warnings so an unused helper does not stop the build). Every mutation made
at least one case fail; the restored source passed all eleven.

| Mutation | Rule broken | Failing cases |
| --- | --- | --- |
| Mean floored (`sum / count`) | The mean rounds to nearest | OPTICS-TP-010, OPTICS-TP-013 |
| Mean ties rounded up (`(sum + count / 2) / count`) | Ties go to the even neighbour | OPTICS-TP-010, OPTICS-TP-013 |
| Quantization floored | Codes round to nearest | OPTICS-TP-015, OPTICS-TP-017, OPTICS-TP-019, OPTICS-TP-020 |
| Quantization ties rounded down | The tie at 0.5 goes to the even code | OPTICS-TP-015, OPTICS-TP-019, OPTICS-TP-020 |
| Toe used for one more input (`<=` at `TOE_END`) | The toe/power boundary | OPTICS-TP-015, OPTICS-TP-018 |
| Hold at the toe end removed | The curve is monotone where the pieces meet | OPTICS-TP-015, OPTICS-TP-018 |
| Toe slope 12.93 instead of 12.92 | The toe is `12.92 x` | OPTICS-TP-015, OPTICS-TP-016, OPTICS-TP-017, OPTICS-TP-018 |
| Interpolation rounded to nearest instead of floored | The interpolation floors | OPTICS-TP-015 |

The last mutation stays within the stated bounds, so only the golden vectors
see it: a version is defined by its bits, not by its accuracy.

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

## Determinism Obligations And Evidence

The camera, the pixel functions and the camera tests use only integer
arithmetic: `i64` and `i128` additions, multiplications, shifts and floor or
rounded divisions with defined results. There is no floating point,
trigonometry, square root per ray, randomness from the host, time, filesystem
access, parallel scheduling, map iteration, stdout, stderr or logging in the
library or in any camera oracle. The pixel tests use `f64` only as the sRGB
reference oracle described under [Fixtures](#the-srgb-reference-oracle); every
value they lock is an integer. The square roots in construction are those of
`FixedUnitVec3::new`, an exact integer square root.

Test builds check integer overflow, so `OPTICS-TP-004` turns any overflow at
the corners of the accepted domain into a test failure.

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
- sample channels of zero, `2^52`, `2^52 + 1`, `-1`, `i64::MIN` and
  `i64::MAX`, and a full pixel of 1 024 samples at `2^52`;
- an empty pixel and an over-full merge;
- tone inputs at zero, one raw unit, both sides of the toe end, inside the
  hold, at every knot and its neighbours, at 1.0 and one raw unit on either
  side, the radiance cap and the most negative value;
- display values on both sides of the first 8-bit and 16-bit boundaries, the
  exact tie at 0.5, and values outside `[0, 1]`;
- output slices of lengths 0, 1, 3, 5 and 8.

## Stress And Fuzz

No fuzz target exists today. `OPTICS-TP-003` is a seeded property case over
2 000 cameras; `OPTICS-TP-011` and `OPTICS-TP-013` are seeded property cases
over accumulators, and `OPTICS-TP-016` to `OPTICS-TP-018` use dense
deterministic inputs rather than random ones.

Every minimized fuzz failure must become a committed deterministic regression
case with a stable test name.

## Open Gaps

| Gap | Blocking API |
| --- | --- |
| A differential check that points along a camera ray project, through `FixedMat4::try_look_at`, `FixedMat4::try_perspective` and `Viewport`, back to the ray's sample. | None; a follow-up case (the matrices landed while this chapter was written). |
| Sample positions from a sampling sequence. | flyingrobots/bunny#221. |
| Timing of ray generation. | A `bunny-perf` case once a renderer loop exists. |
| An exhaustive sweep of all `2^32` inputs in `[0, 1]` for the tone curves (the 60-digit model found every code threshold once, which decides every input of a monotone function; the test suite covers the structure densely instead). | None; too slow for a debug-build test, a candidate for a `slow` tier. |
