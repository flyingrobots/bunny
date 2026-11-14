# Optics Test Plan

This document defines how Bunny verifies the [optics contract](README.md): the
pinhole camera in `bunny-optics` and the rays it generates, and the
deterministic sample hash, pixel sampler and sequential generator that choose
sample positions and random decisions.

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
- `Xoroshiro128Plus`, replaying Echo's `warp-math` golden vectors bit for bit.

Projection matrices (flyingrobots/bunny#201, #202), hemisphere and light
sampling (#160), shading and pixel values (#222, #160) are out of scope.

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
status = "planned"
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

## Determinism Obligations And Evidence

The camera and its tests use only integer arithmetic: `i64` and `i128`
additions, multiplications, shifts and floor divisions with defined results.
There is no floating point, trigonometry, square root per ray, randomness from
the host, time, filesystem access, parallel scheduling, map iteration, stdout,
stderr or logging in any oracle. The square roots in construction are those of
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
- sub-pixel offsets of 0, `1 - 2^-32`, 1.0 and below zero.

## Stress And Fuzz

No fuzz target exists today. `OPTICS-TP-003` is a seeded property case over
2 000 cameras.

Every minimized fuzz failure must become a committed deterministic regression
case with a stable test name.

## Open Gaps

| Gap | Blocking API |
| --- | --- |
| A differential check that points along a camera ray project, through `FixedMat4::try_look_at`, `FixedMat4::try_perspective` and `Viewport`, back to the ray's sample. | None; a follow-up case (the matrices landed while this chapter was written). |
| Sample positions from a sampling sequence. | flyingrobots/bunny#221. |
| Timing of ray generation. | A `bunny-perf` case once a renderer loop exists. |
