# Optics

This chapter defines Bunny's current contract for camera, ray-generation,
sampling, shading and pixel-value math in `bunny-optics`: a deterministic
pinhole camera that turns pixel samples into
[trace rays](../ray-tracing/README.md), the deterministic sample hash and pixel
sampler that choose those samples and every later random decision, the
diffuse shading steps that turn trace hits into radiance, and the functions
that turn that radiance into 8-bit and 16-bit output codes.

## Status

This is a living topic chapter, not a proposal.

Current behavior is implemented in `crates/bunny-optics/src` and exported as:

- `PinholeCamera` (`new`, `ray`, `position`, `forward`, `right`, `up`,
  `tan_half_fov`, `width`, `height`, `scale_shift`);
- `CameraSpec`, the camera's inputs;
- `PixelSample` (`new`, `centre`);
- `CameraError`;
- `point_at`;
- `MAX_IMAGE_DIMENSION` and `TAN_HALF_FOV_LIMIT_RAW`;
- `PixelStream` (`new`, `bits`, `fraction`), `fraction_from_bits` and
  `SAMPLE_HASH_ID`, the counter-based [sample hash](#sample-hash);
- `PixelSampler` (`new`, `point`, `camera_sample`, `x`, `y`) and `SAMPLER_ID`,
  the [pixel sampler](#pixel-sampler);
- `Xoroshiro128Plus` (`from_seed`, `from_seed_u64`, `state`, `next_u64`,
  `next_int`, `next_unit`), a [sequential generator](#sequential-generator)
  compatible with Echo's;
- for [pixel values](#pixel-values): `PixelAccumulator` (`new`, `EMPTY`,
  `add_sample`, `merge`, `count`, `sums`, `resolve`), `ToneCurve` (`SrgbV1`,
  `LinearClampV1`; `identity`, `apply`, `encode_8`, `encode_16`),
  `quantize_8`, `quantize_16`, `write_rgba8`, `PixelError`,
  `MAX_SAMPLE_RADIANCE_RAW`, `MAX_SAMPLES_PER_PIXEL`, `UNORM8_IDENTITY`,
  `UNORM16_IDENTITY` and `RGBA8_OPAQUE_IDENTITY`;
- for [diffuse shading](#diffuse-shading) (`bunny-diffuse-shading/1`):
  `TriangleSurface` (`new`, `frame`, `facing_frame`), `ShadingFrame` (`new`,
  `flipped`, `tangent`, `bitangent`, `normal`, `cosine_direction`,
  `try_cosine_direction`), `SurfaceHit` (`new`, `try_new`, `point`, `frame`,
  `bounce_ray`, `try_bounce_ray`, `light_sample`),
  `DirectionalLight` (`new`, `toward_light`, `irradiance`, `cosine`,
  `reflected_radiance`), `LightSample` (`shadow_ray`, `radiance`), `SkyLight`
  (`new`, `radiance`), `Albedo` (`new`, `channels`, `reflect`),
  `ShadingError`, `SecondaryRayError`, `SHADING_ID`, `PI`, `FRAC_1_PI`,
  `MAX_LIGHT_RAW`, `SHADOW_T_MAX` and `MIN_BOUNCE_COSINE_RAW`;
- for [vertex normals](#vertex-normals) (`bunny-vertex-normals/1`):
  `VertexNormals` (`new`, `normals`), `VertexNormalError`,
  `MAX_NORMAL_TRIANGLES` and `VERTEX_NORMALS_ID`;
- for the [smooth normal](#smooth-normal) (`bunny-smooth-normal/1`):
  `smooth_normal` and `SMOOTH_NORMAL_ID`, with `bunny_query::BarycentricNumerators`;
- for [smooth shading](#smooth-shading) (`bunny-smooth-shading/1`):
  `SmoothHit` (`new`, `surface_hit`, `frame`, `light_sample`,
  `is_above_surface`, `bounce`), `SmoothBounce` (`Ray`, `BelowSurface`) and
  `SMOOTH_SHADING_ID`.

Evidence is listed in the [test plan](test-plan.md). Reflection, refraction,
attenuation and materials other than diffuse (the rest of #160) are not part
of this chapter. The camera uses the same conventions as `FixedMat4::try_look_at`
(right-handed, `right = forward x up`), `FixedMat4::try_perspective` (vertical
field of view given by its tangent, aspect `width / height`) and `Viewport`
(top-left origin, `y` down), but no test yet checks a ray against those
matrices; that is an open gap in the test plan.

All arithmetic is integer arithmetic on raw Q32.32 values (`i64`, widened to
`i128`). There is no floating point, no trigonometry, no `unsafe` code and no
target-specific path. The camera's part of generating a ray takes no square
root and no division: six `i128` multiplications, additions and shifts. The
`TraceRay::new` call that follows computes the ray's three reciprocals, as for
any trace ray.

## Inputs

A `CameraSpec` holds:

| Field | Meaning | Accepted |
| --- | --- | --- |
| `position` | The eye point and origin of every ray. | Inside the trace domain: every `abs(raw) < 2^40` (below 256 units). |
| `look` | The viewing direction, any length. | Nonzero. |
| `up` | A hint that fixes the roll, any length. | Nonzero and not within about `2^-16` of parallel to `look`. |
| `tan_half_fov` | The tangent of half the vertical field of view. | `0 < tan <= 256.0` (`TAN_HALF_FOV_LIMIT_RAW = 2^40`). |
| `width`, `height` | The image size in pixels; their ratio is the aspect ratio. | `1..=32768` each (`MAX_IMAGE_DIMENSION = 2^15`). |

The field of view is given by its tangent so that no trigonometry is needed: a
caller with an angle converts it once, at the boundary. A tangent of 1.0 is a
90-degree vertical field of view.

## Construction

`PinholeCamera::new` checks its inputs in this order and returns the first
error:

1. a position coordinate outside the trace domain:
   `PositionOutsideTraceDomain`;
2. a zero width or height: `EmptyImage`; one above 32768: `ImageTooLarge`;
3. a zero or negative tangent: `TangentNotPositive`; one above 256.0:
   `TangentTooLarge`;
4. a zero look direction: `ZeroLook`;
5. a zero up hint: `UpParallelToLook`;
6. an up hint parallel or nearly parallel to the look direction, meaning
   every component of `unit(look) x unit(up)` is below `2^16` raw units (the
   sine of the angle between them is below about `2^-16`): `UpParallelToLook`;
7. a basis vector that `FixedUnitVec3::new` fails to normalize, or no
   direction scale that fits (see below): `Unrepresentable`. This is a
   defensive refusal; no accepted input is known to reach it.

Look and up vectors of any nonzero length are accepted, from one raw unit to
the full Q32.32 range: each vector is scaled by a power of two before it is
normalized (see Camera Frame), so its length can neither overflow nor be too
short for `FixedUnitVec3::new` to normalize. That is why there is no overflow
error.

## Camera Frame

The camera derives a right-handed unit basis from `look` and `up`, using
`FixedVec3::cross` and `FixedUnitVec3::new`:

```text
forward     = unit(look)
side        = forward x unit(up)            // all components < 2^16 raw: UpParallelToLook
first_right = unit(side)
up          = unit(first_right x forward)
right       = unit(forward x up)
```

Here `unit(v)` is `FixedUnitVec3::new` applied to `v` scaled by the power of
two that puts its largest component magnitude in `[2^61, 2^62)`. Scaling up
is exact; scaling down floors each component, and only the direction matters.
`FixedUnitVec3::new` on its own refuses many vectors much shorter than one
unit, because its floored length loses relative precision. A side vector of
length 0.18 misses the unit-length check by two raw units, for example, and
about one random up hint in twenty would be refused. The prescale removes
that.

So `right = forward x up` and `forward = up x right`, the right-hand rule of
[the coordinate law](../coordinate-law/README.md). A camera looking down `-Z`
with up `+Y` has right `+X`: its image is the canonical `XY` view, with `+X`
right and `+Y` up. The camera's up is the part of the hint orthogonal to
`forward`, on the hint's side.

The second round of cross products starts from two nearly orthogonal unit
vectors, so the basis is orthogonal to a few raw units even when the hint is
nearly parallel to `look`. The test plan checks every pairwise dot product
below `2^36` raw-squared units, a cosine below `2^-28`.

## Image Convention

The image is a rectangle on the plane one unit ahead of the position, centred
on `forward`. It spans `-tan..=tan` along `up` and
`-tan * width / height..=tan * width / height` along `right`; pixels are
square.

- **Origin.** Pixel `(0, 0)` is the top-left pixel.
- **Columns.** `x` counts from the left edge and grows toward `right`.
- **Rows.** `y` counts from the top edge and grows toward `-up`, downward.
- **Sub-pixel offsets.** A `PixelSample` names pixel `(x, y)` and an offset
  `(sub_x, sub_y)` in `[0, 1)` on each axis, as Q32.32 fractions, measured
  rightward and downward from the pixel's top-left corner.
  `PixelSample::centre` uses one half on both axes, the pixel centre.

This is the convention of the coordinate law's
[viewport conversion](../coordinate-law/README.md#viewport-conversion) and of
`bunny_linalg::Viewport`: origin at the top-left corner, pixel `y` running
down, pixel `i` spanning `[i, i + 1)`. A sample's continuous viewport
coordinate is exactly `(x + sub_x, y + sub_y)`, and rows come out in the order
image buffers store them.

The sample's point on the image rectangle is, relative to the position,

```text
P = forward + a * (tan / height) * right + b * (tan / height) * up
a = 2 * (x + sub_x) - width
b = height - 2 * (y + sub_y)
```

where `a` and `b` are the doubled offsets from the image centre in pixels.
Both are exact in Q32.32 for every sample.

## Ray Generation

`PinholeCamera::ray(sample)` returns a `TraceRay` from the position toward the
sample's point, or `None` when the sample is outside the image: `x >= width`,
`y >= height`, or a sub-pixel offset outside `[0, 1)`. It is a pure function
of the camera and the sample.

Construction stores three `i128` vectors in fine units of `2^-64` and a scale
shift `k` (with `F`, `R`, `U` the raw basis and `T` the raw tangent):

```text
centre_i = F_i * 2^32 >> k                       // exact for k <= 32
step_x_i = floor(2 * T * R_i / (height * 2^k))   // change per pixel along x
step_y_i = floor(2 * T * U_i / (height * 2^k))   // change per pixel along y
```

and each direction component is the floor of one exact expression:

```text
d_i = floor((centre_i * 2^33 + a_raw * step_x_i + b_raw * step_y_i) / 2^65)
```

with `a_raw` and `b_raw` the raw Q32.32 values of `a` and `b`. The division by
`2^65` is an arithmetic shift, so the floor is toward negative infinity. Every
rounding in ray generation is a floor.

## Direction Scaling

The direction is not normalized; that is what keeps square roots and divisions
out of the per-ray path. Instead every direction is `2^-k` times the sample's
image-plane offset `P`, and `k` is chosen once, at construction, so that every
component of every direction is at most 1.0 in magnitude, as
`TraceRay::new` requires.

**The rule.** `k` is the smallest value in `0..=32` for which the four corner
directions of the image rectangle (`a = +-width`, `b = +-height`, computed with
the formula above) have every component in `-1.0..=1.0`, inclusive.

**Why the corners are enough.** For a fixed camera, the expression inside the
floor is an affine function of `(a_raw, b_raw)`. An affine function on a
rectangle takes its largest and smallest values at the rectangle's corners,
and the floor is monotone, so each `d_i` over the whole rectangle lies between
its values at the corners. Every sample has `-width <= a < width` and
`-height <= b < height`, inside the closed rectangle. So if the corners fit,
every sample of every pixel fits.

**Why a fitting `k` always exists.** Each basis component is at most 1.0, so
each component of `P` is at most `1 + tan * (width + height) / height` in
magnitude, which is below `1 + 256 * (2^15 + 1) < 2^24` for every accepted
camera. At `k = 24` the exact corner values are below `1 - 2^-24` units, and
rounding adds at most `(width + height) / 2^33` raw units, so the corners fit.
The search therefore always succeeds by `k = 24`; the most extreme accepted
cameras (tangent 256.0, a 32768 by 1 image) need `k = 23`.

**Forward-facing directions.** Construction also checks that each corner
direction `d` has `d . F > 2 * |F|_1` (raw `i128` arithmetic). Because
`d . F` is within `|F|_1` of an affine function of the sample, this makes
`d . F > 0` for every sample, so no direction is zero and every ray points
ahead of the camera. The check passes with a wide margin for every camera the
tests construct; failing it would return `Unrepresentable`.

**Precision.** Halving the direction for wide images or fields of view costs
one bit of each component per halving. A 90-degree 16:9 camera has `k = 1`;
the widest accepted field of view at the largest square image has `k = 8` or
`9`, depending on the look direction.

## Accuracy

Let `S_i` be component `i` of `2^-k * P` in raw units, computed exactly from
the camera's stored basis, tangent and image size. Then

```text
S_i - 1 - (width + height) / 2^33  <  d_i  <=  S_i + (width + height) / 2^33
```

raw units. The final floor contributes the one raw unit below; flooring the
two steps contributes less than `(width + height) / 2^33` raw units in either
direction, which is at most `2^-17` raw units for every accepted image. In other
words: the ray reaches, at distance `2^k`, a point within
`2^k * (1 + 2^-17)` raw units of the intended image-plane point on each axis.

The intended point is defined with the stored basis, which is unit length and
orthogonal only up to its own rounding (see Camera Frame).

## Distances Along A Ray

Because directions are not unit length, a trace distance `t` (from
`TraceRay::box_entry` or `TriangleHit::distance`) is a multiple of that ray's
direction, and rays through different pixels have different direction lengths.
Distances from different rays are therefore not comparable as Euclidean
lengths. What a renderer can rely on:

- **The hit point.** `point_at(ray, t)` returns `origin + direction * t`. Each
  product is rounded to nearest with ties to even (the Q32.32 multiplication
  rule of the [numeric constitution](../../NUMERIC_CONSTITUTION.md)) and the sum
  is exact; it returns `None` when a coordinate is not representable. With the
  floored distance of `TriangleHit`, the point is on the camera's side of the
  surface by less than one raw unit of distance times the direction, plus the
  product rounding.
- **Depth along the view axis.** The forward component of every direction is
  `2^-k` before rounding, so `t * 2^-k` is, up to the accuracy bound and the
  basis rounding, the hit's depth along `forward`. That depth is comparable
  across pixels; it is not computed by this crate.
- **Euclidean distance.** `t * |direction|`, which needs a square root per ray;
  a caller that needs it computes it with `FixedVec3::length`.

## Accepted Limits

| Limit | Value | Why |
| --- | --- | --- |
| Image width and height | `1..=32768` | Keeps the step rounding at most `2^-17` raw units and every intermediate below `2^123` in `i128`; covers 16K images (15360 by 8640). |
| Tangent of half the vertical field of view | `(0, 256.0]` | 256.0 is a vertical field of view just below 179.56 degrees. With it, a step at `k = 0` is below `2^73` fine units, so each product in the direction formula stays below `2^121`. |
| Position | `abs(raw) < 2^40` | The trace domain of `TraceRay::new`. |
| Scale shift | `0..=32` | The centre vector `F * 2^32 >> k` is exact up to 32; accepted cameras need at most 24. |

Narrow fields of view are accepted down to a tangent of one raw unit. The ray
is still exact to the bound above, but the per-pixel step can shrink below one
raw unit of direction, so neighbouring pixels may share a ray.

## Sampling

A renderer draws several random values per ray: the sub-pixel offset, then
two values per bounce direction, and so on. If those values came from a
generator with state, each value would depend on how many values were drawn
before it, and so on the order in which pixels and samples were computed and
on how many threads computed them. Every value here is instead a pure function
of its key:

| Key field | Type | Meaning |
| --- | --- | --- |
| `seed` | `u64` | The render's seed. |
| `x`, `y` | `u32` | The pixel, with the camera's convention (top-left origin). |
| `sample` | `u32` | The sample index within the pixel, from 0. |
| `dimension` or `pair` | `u32` | Which value of the sample: a hash dimension, or a sampler dimension pair. |

All arithmetic is wrapping `u64` and `u32` multiplication, addition, xor,
shifts and bit reversal, whose results Rust defines identically on every
target, including `wasm32`. There is no floating point, no table and no state.

### Identities

The functions are contracts, named so that an artifact can record which one
produced its samples:

| Identity | Function | Constant |
| --- | --- | --- |
| `bunny-sample-hash/1` | `PixelStream::bits`, and so `PixelStream::fraction` | `SAMPLE_HASH_ID` |
| `bunny-sobol-owen/1` | `PixelSampler::point`, and so `PixelSampler::camera_sample` | `SAMPLER_ID` |

Any change to any output bit of either function is a new version (`/2`) with
a new function or type; under the Fidelity Rule the existing one keeps its
output. The golden vectors in the [test plan](test-plan.md) define version 1.

### Sample Hash

`bunny-sample-hash/1` is built from `mix64`, the output function of
SplitMix64 (Steele, Lea and Flood, "Fast Splittable Pseudorandom Number
Generators", OOPSLA 2014; the constants are David Stafford's "Mix13" variant
of the MurmurHash3 64-bit finalizer):

```text
mix64(z):  z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9
           z = (z ^ (z >> 27)) * 0x94D049BB133111EB
           return z ^ (z >> 31)
absorb(h, w) = mix64((h ^ w) + 0x9E3779B97F4A7C15)     // wrapping u64

PixelStream::new(seed, x, y).state = absorb(absorb(0, seed), x | y << 32)
bits(sample, dimension)            = absorb(state, sample | dimension << 32)
```

Key fields are combined as three 64-bit words absorbed in a fixed order:
the seed, then the pixel word (`x` in bits 0 to 31, `y` in bits 32 to 63),
then the sample word (`sample` in bits 0 to 31, `dimension` in bits 32 to 63).

**Domain separation.** Each `mix64` step is a bijection of `u64` (an
xor-shift by at least one bit and a multiplication by an odd constant are each
invertible), so `absorb(h, w)` is a bijection of `w` for fixed `h`, and of `h`
for fixed `w`. Therefore:

- for a fixed seed and sample word, two different pixels give different
  states after the pixel word and so different bits; `(x, y) = (1, 0)` and
  `(0, 1)` are different pixel words and never collide;
- for a fixed seed and pixel, two different (sample, dimension) pairs never
  give the same bits; nor do swapped sample and dimension.

Keys that differ in both the pixel and the (sample, dimension) word can give
equal bits: 192 key bits cannot map injectively to 64. Nothing beyond the
construction says how often; for a well-mixed function it is about `2^-64` per
pair of keys.

The first absorption, `absorb(0, seed) = mix64(seed + 0x9E3779B97F4A7C15)`, is
exactly the first SplitMix64 output for the state `seed`, which ties the
implementation to SplitMix64's published reference outputs (see the test plan).
Holding a `PixelStream` saves two of the three absorptions per value: a
renderer builds one per pixel.

### Fractions

`fraction_from_bits(bits)` is the Q32.32 value with raw `bits >> 32`: the top
32 bits, as a fraction of `2^32`. The raw value is at most `2^32 - 1 =
ONE_RAW - 1`, so the fraction is in `[0, 1 - 2^-32]`; it is never negative and
never 1.0. Each of the `2^32` possible fractions comes from exactly `2^32`
inputs. `PixelStream::fraction(sample, dimension)` is
`fraction_from_bits(bits(sample, dimension))`.

### Pixel Sampler

`PixelSampler::point(sample, pair)` returns a point `[u, v]` in `[0, 1)^2`.
`bunny-sobol-owen/1` is the first two dimensions of the Sobol sequence, a
(0, 2)-sequence in base 2, with the sample index shuffled and each coordinate
scrambled by a hash-based nested uniform (Owen) scramble, in the arrangement
of Burley ("Practical Hash-based Owen Scrambling", Journal of Computer
Graphics Techniques 9(4), 2020):

```text
PixelSampler::new(seed, x, y).state = absorb(absorb(0x534F424F4C5F4F31, seed), x | y << 32)

point(sample, pair):
  p0 = absorb(state, pair)               // pair as a u64
  p1 = mix64(p0 + 0x9E3779B97F4A7C15)
  i  = owen(sample, low32(p0))           // shuffled index
  u  = owen(reverse32(i), high32(p0))    // Sobol dimension 0: van der Corput
  v  = owen(sobol1(i), high32(p1))       // Sobol dimension 1
  return [u * 2^-32, v * 2^-32]

owen(v, s) = reverse32(lk(reverse32(v), s))
lk(x, s):  x += s
           x ^= x * 0x6C50B47C;  x ^= x * 0xB82F1E52
           x ^= x * 0xC7AFE638;  x ^= x * 0x8D22F6E6        // wrapping u32
sobol1(i)  = xor of V_b over the set bits b of i,
             V_0 = 2^31, V_(b+1) = V_b ^ (V_b >> 1)
```

`reverse32` reverses the 32 bits of a word. `V_b` has bit `31 - r` set exactly
when the binomial coefficient `C(b, r)` is odd: the generator matrix of the
second Sobol dimension (primitive polynomial `x + 1`) is the Pascal matrix
modulo 2. The sampler's domain constant (the ASCII bytes `SOBOL_O1`) differs
from the hash's zero, so for the same seed the two never share a state.

Each coordinate is a `u32` shown as raw Q32.32, so it is in `[0, 1 - 2^-32]`,
never negative and never 1.0.

`PixelSampler::camera_sample(sample)` is `PixelSample::new(x, y, u, v)` for
pair 0: `u` is the rightward sub-pixel offset and `v` the downward one, so it
can be passed straight to `PinholeCamera::ray`.

### Stratification

**The property.** For every seed, pixel and pair, every `k` in `0..=32` and
every block `m`, the `2^k` points of samples `m * 2^k` to `(m + 1) * 2^k - 1`
form a (0, k, 2)-net in base 2: for each `p` in `0..=k`, divide the unit
square into `2^p` columns and `2^(k - p)` rows of equal dyadic cells; every
cell holds exactly one point. For `k = 4` (16 samples) that is one point in
each cell of the 16 by 1, 8 by 2, 4 by 4, 2 by 8 and 1 by 16 grids, which
includes classic jittered stratification (4 by 4) and both one-dimensional
stratifications.

**Why it holds.**

1. *The unscrambled points.* `(reverse32(i), sobol1(i))` is the first two
   dimensions of the Sobol sequence, which is a (0, 2)-sequence in base 2
   (Sobol 1967; Niederreiter, *Random Number Generation and Quasi-Monte Carlo
   Methods*, 1992): every aligned block of `2^k` consecutive indices is a
   (0, k, 2)-net, at the full 32 bits of precision.
2. *The shuffle maps an aligned block onto an aligned block.* In `lk`, the
   addition changes bit `j` by the carry from bits below `j`, and `x ^= x * c`
   with an even `c` changes bit `j` by a function of bits below `j` only. So
   `lk` keeps each bit and flips it by a function of lower bits, and `owen`,
   which works on reversed bits, flips each bit of the index by a function of
   higher bits. Samples `m * 2^k ..` share their bits above `k - 1`, so their
   shuffled indices share those bits too, and the low `k` bits are permuted
   among themselves: the block maps onto another aligned block of `2^k`
   indices, which by step 1 is a (0, k, 2)-net.
3. *The scramble keeps nets.* On a coordinate, `owen` flips each bit by a
   function of the more significant bits, which maps every dyadic interval of
   length `2^-p` onto a dyadic interval of the same length, one to one. A
   dyadic cell with one point is mapped to a dyadic cell with one point.

The test plan checks the property by counting, for `k` up to 10 (1 024
samples), and shows that it fails when the second dimension, the scramble or
the shuffle is broken.

**Decorrelation between pixels.** Every pixel has its own shuffle and
scramble seeds from the hash, so neighbouring pixels do not repeat a pattern.
This is decorrelation by hashing, not a proven property; the test plan has a
sanity guard (no neighbouring pixels share a point).

### Dimension Pairs

An integrator draws each two-dimensional decision from its own pair: pair 0 is
the sub-pixel offset, and pairs 1, 2, ... are free for the integrator to assign
(for example, one pair per bounce direction and one per light sample). Each
pair has its own shuffle and scrambles, derived from `(seed, pixel, pair)`.
So, within a pixel:

- **within a pair**, across sample indices, the points are stratified as
  stated above;
- **across pairs**, the points of different pairs are decorrelated by
  independent hash seeds (padding, in Burley's terms). The joint distribution
  of two pairs is not a net and is not claimed to be independent; the test
  plan has a sanity guard on one projection, not a proof.

A one-dimensional decision can use one coordinate of a pair, or
`PixelStream::fraction` with its own dimension index.

### Determinism

Every value is a pure function of its key, so an image does not depend on the
order in which pixels or samples are computed, on tile size or on the number
of threads. `PixelStream` and `PixelSampler` are small `Copy` values holding
one precomputed state; reusing one gives the same values as building it again.

### Statistical Limits

No statistical test suite (such as PractRand or TestU01) has been run on
`bunny-sample-hash/1` or `bunny-sobol-owen/1`. Their quality claims are those
of the constructions: SplitMix64's output function is a well-studied 64-bit
mixer; the chained absorption is this crate's arrangement of it; the
Laine-Karras style hash is an approximation of a random Owen scramble whose
nesting (and so the stratification property) is exact but whose randomness
is only what Burley reports for these constants. The avalanche and
decorrelation tests in the test plan are regression guards on the mixing,
with generous thresholds, not certification.

### Sequential Generator

`Xoroshiro128Plus` is the `xoroshiro128+` generator of Blackman and Vigna
(2016 parameters `a = 55`, `b = 14`, `c = 36`), seeded and drawn exactly as
Echo's `warp-math` `Prng`, so Echo can move to it without changing a bit of
its timelines:

- `from_seed(s0, s1)` uses the state `[s0, s1]`, replacing all zeros with
  `[0x9E3779B97F4A7C15, 0]`;
- `from_seed_u64(seed)` uses the first two SplitMix64 outputs for `seed`;
- `next_u64` returns `s0 + s1` and steps the state;
- `next_int(min, max)` keeps the low bits of one draw for a power-of-two span
  and rejects draws at or above the largest multiple of the span otherwise; it
  returns `None`, drawing nothing, when `min > max`, where Echo panics;
- `next_unit` returns `(next_u64 >> 41) * 2^-23` as Q32.32, which is exactly
  the value of Echo's `next_f32` (a 23-bit fraction is exact in both); Bunny
  has no float version.

`state` returns the state for checkpointing. Each value depends on every draw
before it, so this generator is for code that draws in one fixed order, not
for rendering. Echo's golden vectors replay exactly (see the test plan).

## Diffuse Shading

Between a trace hit and a radiance sample, a path tracer with diffuse
(Lambertian) surfaces, one directional light, a uniform sky and at most one
bounce takes a fixed set of steps. Each step is a pure integer function in
`crates/bunny-optics/src/shading/`, with golden vectors and a stated bound, so
that a renderer only wires them together and two renderers that wire them the
same way produce the same bits.

**What Bunny owns.** The triangle's normal and frame, the facing rule, where
secondary rays start, the bounce direction for a sampler point, the light's
term and its shadow ray, the sky's radiance, the diffuse reflection, and the
reference order in which they combine. **What the renderer owns.** Scene
loading and vertex transforms, the acceleration structure and the
closest-hit and any-hit queries (`bunny-broadphase`), which sampler pair each
decision uses, the loop over pixels and samples, and the pixel value
([Pixel Values](#pixel-values)). Materials other than diffuse, other lights,
textures, reflection and refraction are not defined here. Per-vertex normals
for a mesh are ([Vertex Normals](#vertex-normals)), and so is their
interpolation at a hit ([Smooth Normal](#smooth-normal)) and a hit shaded with
it ([Smooth Shading](#smooth-shading)), including its bounce.

### Shading Identity

| Identity | Constant | Names |
| --- | --- | --- |
| `bunny-diffuse-shading/1` | `SHADING_ID` | `TriangleSurface::new`, `ShadingFrame::new`, `ShadingFrame::flipped`, `ShadingFrame::cosine_direction`, `SurfaceHit::new`, `SurfaceHit::bounce_ray`, `SurfaceHit::light_sample`, `DirectionalLight::cosine`, `DirectionalLight::reflected_radiance`, `LightSample::radiance`, `Albedo::reflect`, `SkyLight::radiance`, the constants below, and the [order of operations](#order-of-operations). |

Any change to an output bit of any of these is a new version (`/2`) with new
functions or types; under the Fidelity Rule the existing ones keep their
output. The golden vectors in the [test plan](test-plan.md) define version 1.

| Constant | Raw value | Meaning |
| --- | --- | --- |
| `PI` | 13 493 037 705 | The nearest raw value to `pi * 2^32` (13 493 037 704.522), 0.478 raw units above pi. |
| `FRAC_1_PI` | 1 367 130 551 | The nearest raw value to `2^32 / pi` (1 367 130 551.153), 0.153 raw units below. The light's term uses it. |
| `MAX_LIGHT_RAW` | `2^48` (65 536.0) | The largest light irradiance or sky radiance per channel. |
| `SHADOW_T_MAX` | `2^41` (512.0) | The distance limit of a shadow ray's any-hit query. |
| `MIN_BOUNCE_COSINE_RAW` | `2^15` | Every bounce direction's dot product with the normal is at least this. |

### Geometric Normal

`TriangleSurface::new(&TraceTriangle)` prepares a triangle once, before any
hit: its exact plane, unit normal and frame.

- **Plane.** `N = (v1 - v0) x (v2 - v0)` in exact `i128` (edges are below
  `2^41` raw, so `N` is below `2^83`), and `N . v0`. `N` is the right-handed
  normal of the [coordinate law](../coordinate-law/README.md): it points to
  the triangle's front face, where `TriangleHit::front_face` is `true`.
- **Unit normal.** `N` scaled by the power of two that puts its largest
  component magnitude in `[2^61, 2^62)` (the camera basis's prescale), then
  `FixedUnitVec3::new`. Each component is within one raw unit of the exact
  `N / |N|`.
- **Degenerate triangles.** A zero-area triangle (duplicate or collinear
  vertices) has `N = 0` exactly and returns `None`. The exact triangle test
  never reports a hit on one, so a renderer never needs its surface.

**Unit, and per triangle.** The cosine terms need a consistent normalization,
and a unit normal gives one with no length to carry. It costs two square
roots and three divisions, and the frame three more divisions, so the
renderer builds a `TriangleSurface` per triangle, not per hit (84 ns each, see
[Accuracy And Cost](#accuracy-and-cost)); a hit then needs no square root or
division for its normal. The geometric normal of a two-sided triangle is used
for shading; [vertex normals](#vertex-normals) are a separate identity that
no step of version 1 uses.

### Vertex Normals

`VertexNormals::new(&positions, &triangles)` gives each vertex of an indexed
triangle mesh one unit normal from the faces around it: the first half of
smooth shading. The mesh is a slice of `FixedVec3` positions and a slice of
`[u32; 3]` triangles, each three indices into the positions, as Lumen's PLY
loader and `bunny-mesh::Triangle32` hold them. The rule has its own identity,
`bunny-vertex-normals/1` (`VERTEX_NORMALS_ID`); it is not part of
`bunny-diffuse-shading/1`, whose steps all use the geometric normal.

**Area weighting.** Each triangle `[a, b, c]` contributes its exact `i128`
cross product `N = (p[b] - p[a]) x (p[c] - p[a])` to each of its three
vertices, and a vertex's normal is the unit vector along the sum `S` of its
faces' `N`. `|N|` is twice the face's area, so the sum is already
area-weighted: no face needs a square root or a division, and listing a
face's corners from another one (a cyclic rotation) gives the same `N`. The
alternatives cost more and are not exact: equal weights need each face
normalized first (a square root per face), and angle weights need each
corner's angle (an arccosine or a two-argument arctangent). Area weighting is
the rule whose sum is exact in integers, and it lets a large face count for
more than a sliver at the same vertex.

**One rounding.** `S` is normalized exactly as `TriangleSurface::new`
normalizes `N`: scaled by the power of two that puts its largest component
magnitude in `[2^61, 2^62)` (exact when it scales up; an arithmetic shift,
floored, when it scales down), then `FixedUnitVec3::new` (the floor of the
integer square root of the sum of squares, and each component divided by it,
rounded to nearest, ties to even). Each component is within one raw unit of
the exact `S / |S|`. On a flat mesh every vertex therefore gets its plane's
unit normal to within that unit, the same bound as each face's
`TriangleSurface`. The flat meshes of the test plan (in an axis plane, and in
a plane with normal `(0, 3, 4) / 5`, where every prescale is exact) give the
same bits at every vertex, whatever its number of faces.

**Domain and widths.** Every position must be inside the trace domain (each
coordinate magnitude below `TRACE_COORD_LIMIT_RAW`, `2^40` raw), the domain of
the triangles a renderer traces. Then each edge component is below `2^41`,
each component of `N` below `2 * (2^41)^2 = 2^83`, and with at most
`MAX_NORMAL_TRIANGLES = 2^32` triangles a vertex's sum stays below `2^115`,
well inside `i128`. After the prescale each component is below `2^62`, so the
sum of squares (below `3 * 2^124`) fits `u128` and each division's numerator
(below `2^94`) fits `i128`.

**Order.** The sums are exact integer additions, which are commutative and
associative when nothing overflows, so `S` is a function of the set of faces
at the vertex: the order of the triangles, which corner each lists first, and
how a caller might split the work cannot change a bit. Relabelling the
vertices relabels the normals. Reversing a face's winding negates its `N`, so
a mesh must be wound consistently: faces wound against their neighbours
cancel at the vertices they share.

**No normal.** A vertex that no triangle lists, or whose `S` is exactly zero
(only zero-area faces, or faces that cancel, such as one triangle listed with
both windings), has `None`, not a substitute vector; the caller decides what
to shade it with, for example the geometric normal.

**No welding.** Vertices are identified by index, not position. Two vertices
at the same position (scanned meshes such as the Stanford models list some
positions more than once) each get the normal of only the faces that list
them, so a seam can show where a surface was split. Welding is not provided.

| Input | Accepted | Refusal (`VertexNormalError`, checked in this order) |
| --- | --- | --- |
| Triangle count | At most `MAX_NORMAL_TRIANGLES` (`2^32`) | `TooManyTriangles { count }` |
| Position | Each coordinate magnitude below `2^40` raw | `PositionOutsideTraceDomain { vertex, position }`, the first such vertex |
| Index | Below the number of positions | `IndexOutOfRange { triangle, index, vertex_count }`, the first such triangle and its first such corner |

There is one form, returning `Result`, because a build runs once per mesh and
its refusals need their reason; there is no `Option` fast path.
`VertexNormalError` is `#[non_exhaustive]`.

**Cost.** One cross product (six `i128` multiplications) and nine `i128`
additions per triangle, one normalization (two integer square roots and
three divisions) per vertex, and two allocations: the sums (48 bytes per
vertex) and the normals. The measured cost per vertex is under
[Accuracy And Cost](#accuracy-and-cost).

### Smooth Normal

`smooth_normal(normals, weights)` interpolates a triangle's three vertex unit
normals (in the order `v0`, `v1`, `v2`) at a point given by its exact
barycentric numerators, and returns the unit shading normal there. The weights
come from `TriangleHit::barycentric_numerators` (see the
[ray-tracing chapter](../ray-tracing/README.md#exact-triangle-test)): the
integers `u`, `v` and `det > 0` the exact triangle test decided the hit with,
so the weights of the vertices are `w = (det - u - v, u, v) / det` with no
rounding. The rule has its own identity, `bunny-smooth-normal/1`
(`SMOOTH_NORMAL_ID`); no step of `bunny-diffuse-shading/1` uses it.

1. **Shared normal.** When every vertex with a nonzero weight has the same
   normal, that normal is returned unchanged. A hit on a corner, on an edge
   between equal normals, or on a face whose three normals are equal (a flat
   part of a mesh) gets its vertex normal bit for bit. Without this step the
   normal would be normalized again, which can move a component by a raw
   unit: `FixedUnitVec3` holds vectors whose length is within one raw unit of
   1.0, not exactly 1.0. The step is part of the identity, and so is its cost:
   at a point next to a corner (or next to an edge between equal normals), where
   another weight is nonzero, the normal is normalized again, so it can differ
   from the corner's by up to one raw unit per component more than the
   interpolation itself moves.
2. **Shift.** Each exact `w_i` is shifted right, floored, by the same `k`: the
   smallest that puts `det >> k` below `2^62` (zero when `det` already is).
   `det` reaches about `2^117` (`3 * 2^115`), and a weight times a normal
   component (up to `2^32 + 1` raw) would need about `2^150`; after the shift
   each product is below `2^95`. Each weight is shifted on its own, so the
   shifted weights need not sum to `det >> k`.
3. **Combine.** `m = sum (w_i >> k) n_i`, exact in `i128`. Nothing divides by
   `det`: the normalization makes any positive scale cancel.
4. **Normalize.** `m` is normalized exactly as `TriangleSurface::new`
   normalizes a face normal: a power-of-two prescale to a largest component in
   `[2^61, 2^62)` (exact when it scales up, floored when it scales down), the
   floor of the integer square root of the sum of squares, and each component
   divided by it, rounded to nearest with ties to even.

`None` means `m` is exactly zero: vertex normals that cancel at this point,
such as opposite normals at the midpoint of their edge. The caller chooses
what to shade with then, for example the geometric normal.

**Bound.** Let `rho = |sum w_i n_i| / (2^32 det)` be the length, in units, of
the exact interpolation of the given raw normals (1.0 where they agree, less
where they spread apart, 0 where they cancel). Each component is within
`1 + 2^-26 / rho` raw units of the exact unit vector along `sum w_i n_i`, and
within 1 raw unit when `k = 0`:

- one raw unit from step 4, which is the bound of the face normal;
- the floors of step 2 change `m` by `e = sum f_i n_i` with each `f_i` in
  `[0, 1)`, so `|e| < 3 (2^32 + 2)`; with `k > 0`, `det >> k` is at least
  `2^61`, so `|m|` is at least `rho 2^93`, and the unit vector moves by at
  most `2 |e| / |m| < 6 (2^32 + 2) / (rho 2^93)`: below `2^-26 / rho` raw
  units.

For `rho >= 2^-26` that is within 2 raw units. For nearly cancelling normals
the direction itself is ill-conditioned, and the bound grows as `rho` falls.

**Every rounding is pinned.** The rule rounds in four places: the weight shift
(floor), the prescale (floor when it scales down), the length (floor of the
square root) and the division (to nearest, ties to even). The test plan has an
input for each that sits on its boundary, with a neighbour on either side
where one applies: a tied weight that the floor sends to zero, a component that
is exactly half a raw unit after the division, a squared length above
`(L + 1/2)^2` where a rounded square root would differ, and a prescale whose
dropped bits are above one half. Changing any of the four rules fails a test.

**Cost.** One comparison of up to three normals, three shifts, nine `i128`
multiplications and one normalization (two integer square roots and three
divisions) per hit; no division by `det`. On the benchmark scene's sphere hits
it measures about 72 ns per hit. The alternative that floors two
`barycentrics` weights (two long divisions) and then takes the same steps takes
14 % to 18 % longer, has a looser bound (each component within
`1 + 8 (1 + 2^-32) / rho` raw units, from its two floors), and gives other bits
on 451 of those 1 024 hits, by one raw unit (447) or two (4) in a component
(OPTICS-TP-086). See [Accuracy And Cost](#accuracy-and-cost) and the
[performance ledger](../performance/README.md#13-smooth-normal-from-exact-barycentric-numerators-new-api).

### Smooth Shading

`SmoothHit::new(hit, weights, normals)` shades a `SurfaceHit` with the
interpolated normal of its triangle's three vertex normals
(`[Option<FixedUnitVec3>; 3]`, in vertex order) at the hit's exact weights
(`TriangleHit::barycentric_numerators`). Its identity is
`bunny-smooth-shading/1` (`SMOOTH_SHADING_ID`); nothing in
`bunny-diffuse-shading/1` changes, and the `SurfaceHit` it holds keeps its
geometric frame, its start point and its exact plane (`surface_hit`).

**The frame.** Let `n = smooth_normal(normals, weights)` and `s = n . N`, the
exact `i128` dot product with the exact normal `N` of the face the ray arrived
on: `(v1 - v0) x (v2 - v0)`, negated for a back-face hit, so chosen by the
kernel's `front_face`, the same plane the start-point rule uses (each product
is below `2^116`, the sum below `2^118`).

| Case | Shading frame |
| --- | --- |
| `s > 0` | `ShadingFrame::new(n)` |
| `s < 0`: the normals point to the other face (a back-face hit on a mesh wound for its front) | `ShadingFrame::new(n).flipped()`, normal `-n` |
| `s = 0`, a vertex normal is `None`, or `smooth_normal` returns `None` | `hit.frame()`, the geometric frame |

The shading normal is therefore always strictly on the arrival side of the
exact plane. Every side decision uses `front_face` and the exact plane, never
a dot product with the rounded unit geometric normal: that normal is within a
raw unit of the exact one per component, and for a shading normal nearly
along the surface the two can disagree in sign (the test plan has such a
case, found by search). A missing vertex normal falls back even when its
weight is zero, so the rule depends only on which normals exist.

**The light.** `light_sample` keeps the geometric test of
`SurfaceHit::light_sample`: no shadow ray and no light unless the light is
strictly on the arrival side of the exact plane. When it is, the shadow ray is
the geometric one and the radiance is
`light.reflected_radiance(light.cosine(n'))` with the shading normal `n'`,
whose cosine is clamped at zero: a light above the surface but behind the
shading normal keeps its shadow ray and contributes nothing.

**The bounce.** `bounce(sample)` takes the cosine-weighted direction `d` of the
shading frame for the sampler point (`ShadingFrame::cosine_direction`, so
`None` for a sample outside the unit square). When `is_above_surface(d)`, that
is `d . N > 0` exact (each product below `2^115`), it returns
`SmoothBounce::Ray`, the ray from the surface hit's start point along `d`, to
be queried with a minimum distance of zero as before. Otherwise it returns
`SmoothBounce::BelowSurface`: no ray is traced and the bounce contributes
zero. A direction along the plane (`d . N = 0`) is below. The reason is the
start-point rule: the start point is only one or two raw units above the
plane, so a ray along or under it would hit its own triangle or a neighbour at
once, a self-shadowing artifact rather than light.

**Estimator.** Against `bunny-diffuse-shading/1`, in its
[order of operations](#order-of-operations), three things change and nothing
else does:

- the direct term's cosine uses the shading normal (visibility, the start point
  and the shadow ray do not change);
- the bounce direction is sampled around the shading normal;
- a bounce at or below the geometric surface contributes `B = 0` instead of the
  sky's or a second hit's radiance, and no ray is traced for it.

The cosine-weighted sampling still cancels the cosine against its density, so
an accepted bounce has weight `albedo`, as before. Rejection loses energy, on
purpose: where the shading normal tilts away from the geometric one, the part
of its hemisphere below the surface is dark, so grazing and strongly tilted
shading normals reflect less of the bounce light than a flat shading normal
would: with a shading normal 53 degrees from the geometric one, 49 of a 16 by
16 grid of sampler points are rejected in the test plan's case, and none with
the shading normal along the geometric one. That is the documented,
deterministic choice of this identity.

**Cost.** A whole smooth-shaded hit (`SurfaceHit::new`, `SmoothHit::new`,
`light_sample`, `bounce` and the reflection) measures about 171 ns on the
benchmark scene's sphere hits, against about 67 ns for the flat step on the
same hits: about 104 ns more, 2.5 times the flat cost. Besides `smooth_normal`
(about 72 ns), `SmoothHit::new` builds a frame around the interpolated normal
(three rounded `i128` divisions, which the flat step takes from the prepared
`TriangleSurface`); see [Accuracy And Cost](#accuracy-and-cost).

### Orthonormal Frame
### Orthonormal Frame

`ShadingFrame::new(normal)` builds the tangent and bitangent of Duff,
Burgess, Christensen, Hery, Kensler, Liani and Villemin ("Building an
Orthonormal Basis, Revisited", JCGT 6(1), 2017), the branch-free frame whose
only case split is the sign of `z`. With `(x, y, z)` the unit normal's raw
components, `s = 1` when `z >= 0` and `-1` otherwise, and `q(a) = a / (s + z)`
rounded to nearest, ties to even, in raw units:

```text
tangent   = (1 - s q(x x), -s q(x y), -s x)
bitangent = (-q(x y), s - q(y y), -y)
```

`|s + z|` is at least 1.0, so the quotients always exist. Each component is
the formula evaluated exactly on the given normal and rounded once, so within
half a raw unit of it. Exact ties occur: with `z = 0` the divisor is 1.0 and
`q(x y)` is a tie whenever the 2-adic valuations of `x` and `y` add up to 31,
which unit normals of real triangles reach (the test plan has four). Not
every such normal comes from a triangle: a triangle's unit normal is
normalized after the prescale to `[2^61, 2^62)`, which keeps it within
`2^-29` raw units of the exact unit circle before each component is rounded,
so one with `x = 2^31` and `z = 0` has `y = 3 719 550 786` or
`3 719 550 787` (a tie, which a sliver in the test plan reaches). The tie at
`(2^31, 3 719 550 789, 0)` reaches `ShadingFrame::new` only from a unit vector
built another way, such as `FixedUnitVec3::try_from_unit`.
The frame is right-handed (`tangent x bitangent = normal`). Because the given
normal is unit length only to within a raw unit, the exact formula is
orthonormal only to within a few raw units: the test plan checks every
pairwise dot product, every squared length minus one and every component of
`tangent x bitangent - normal` within 8 raw units (`2^-29`) over 4 000 frames
of each orientation, with largest values of 4.3, 3.9 and 4.4 raw units in a
400 000-frame measurement.

`flipped()` is `(bitangent, tangent, -normal)`: the same frame seen from the
other side, still right-handed, with no arithmetic but negation.

### Facing And Secondary Rays

`SurfaceHit::new(&surface, &ray, &hit)` prepares one hit, where `hit` is the
`TriangleHit` that `ray.hit_triangle` returned for the surface's triangle.

**Facing.** The hit's frame is `surface.facing_frame(hit.front_face)`: the
surface frame for a front-face hit and its flipped form for a back-face hit.
So the normal always points to the side the ray arrived from, and
`ray.direction() . normal < 0` for every hit (the exact test reports no hit
on a ray parallel to the plane).

**Start point (the self-intersection rule).** Every secondary ray (the
shadow ray and the bounce ray) starts at the same point:

1. `p = point_at(ray, hit.distance)`;
2. let `k` be the axis of the plane normal's largest component magnitude (the
   lowest axis on a tie), and `e` one raw unit along `k`, toward the arrival
   side (the sign of the facing normal's component `k`);
3. the start point is the first of `p`, `p + e`, `p + 2e` that is strictly on
   the arrival side of the triangle's exact plane: `N' . point > N' . v0` in
   exact `i128`, with `N'` the plane normal negated for a back-face hit.

Two steps always suffice: the floored distance leaves the exact point at
`hit.distance` on the arrival side or on the plane, `point_at` moves each
coordinate by at most half a raw unit, so `N' . p - N' . v0 >= -|N'|_1 / 2`,
and each step adds `|N'_k| >= |N'|_1 / 3`.

Two steps do occur. They are needed when the rounding of `point_at` leaves
the point at least `|N'_k|` behind the plane, which takes the rounding of
several coordinates against the plane. That is most likely when the normal's
components are equal, as on a regular octahedron's faces, the worst case of
the proof. With a direction component of exactly 1.0 that coordinate's
product is exact, and two steps then need equal components, a distance with
nothing floored and two products that are exact half-unit ties. On the
octahedron face `(-1, 0, 0),
(0, -1, 0), (0, 0, -1)` a hit can land at plane value `-2^64`: the first step
lands exactly on the plane and the second is strictly in front; a general
triangle can need two steps too (both are golden cases in the test plan).
Over the 99 616 hits the trace kernel reported for the test plan's generated
case (its fixed seed; ray directions with a largest component from 0.5 to
1.0), on octahedron faces 18 653 needed no step, 29 508 one and 1 800 two, and
on general triangles 36 687, 12 960 and 8; none needed three and none was
refused.

`SurfaceHit::new` returns `None` when the point is not representable or the step leaves the
trace domain (possible only for a point on its boundary), and for a hit that
the trace kernel did not report for this ray and face, as when a caller
changes `hit.distance` or `hit.front_face`. `SurfaceHit::try_new`, the
diagnostic path, gives the reason; see [Inputs And Refusals](#inputs-and-refusals). A hit point
more than two raw units outside the trace domain is refused before the plane
test, which keeps that test inside `i128`. The bound is exact and refuses no
hit that could start inside: two steps take a coordinate one raw unit outside
to `TRACE_COORD_LIMIT_RAW - 1`, inside, but take one two raw units outside only
to `TRACE_COORD_LIMIT_RAW`, outside.

Secondary rays are queried with a minimum distance of zero and no primitive
excluded. Because the start point is strictly on the arrival side and every
secondary ray leaves into that side (bounce directions by construction, the
shadow ray only when the light is strictly on that side), a secondary ray
never meets the plane of the triangle it leaves. Therefore it never hits:

- its own triangle;
- a coplanar neighbour, including across a shared edge or vertex;
- an exactly coincident duplicate, with either winding;
- a neighbour across a convex edge, which lies behind the plane.

**What it can miss or leak.** The start point is within 1.5 raw units of the
exact hit point on each axis, plus at most two along axis `k`: at most 3.5 raw
units (about `8e-10` units) on any axis. A surface closer than that on the
arrival side may be skipped, and at a concave corner the start point may move
up to that far through the other wall; the test plan shows a surface 4 raw
units away is found. There is no distance epsilon: a cover `2^-20` units above
a surface still casts its shadow, which a minimum distance of `2^-10` would
not.

**Alternatives not chosen.** *Excluding the primitive just hit*, with no
other rule, starts rays at `p`, which is on the plane or behind it for about
four hits in ten. Accepting such a start point (the test plan's mutation)
makes bounce and shadow rays hit their own triangle or a coplanar neighbour at
distance zero in the shared-edge scenes; excluding the primitive just hit
removes the first failure but not the second, nor a coincident duplicate.
*A minimum distance* `t_min` depends on the direction's length (trace
directions are not unit length) and misses nearby geometry. *An offset along
the normal* by a fixed length depends on the scene's scale. The chosen rule is
exact, scale-free and has no constant.

### Cosine-Weighted Hemisphere

`ShadingFrame::cosine_direction([u, v])` maps a unit-square point (a sampler
pair: `PixelSampler::point(sample, pair)`) to a direction on the normal's
side, distributed with density `cos(theta) / pi`: the polar disk point
`(sqrt(u) cos(2 pi v), sqrt(u) sin(2 pi v))` lifted to the hemisphere
(Malley's method).

```text
radius = floor_sqrt(u)              // sin(theta), in [0, 1)
height = floor_sqrt(1 - u)          // cos(theta), in [2^-16, 1]
(sin, cos) = FixedAngle::from_turns(v).sin_cos()
d_i = round((radius cos t_i + radius sin b_i + height n_i) / 2^64)   // exact i128
d_i = clamp(d_i, -1.0, 1.0)
```

`floor_sqrt` is `FixedQ32_32::sqrt`, the exact integer square root, floored.
`v` turns is an exact `FixedAngle`; its sine and cosine are within one raw
unit. The three products and their sum are exact in `i128` (below `2^98`),
rounded once per component to nearest with ties to even. The polar map
preserves area, so with exact arithmetic the direction would be exactly
cosine-weighted: `P(cos(theta) <= c) = c^2`.

Guarantees, each checked by the test plan:

- **Range.** Each component is in `-1.0..=1.0` (the clamp), so the direction
  is a valid trace direction as returned. `None` when `u` or `v` is outside
  `[0, 1)` (`try_cosine_direction` returns `SampleOutsideUnitInterval`).
- **Above the surface.** `u <= 1 - 2^-32`, so `height >= sqrt(2^-32) = 2^-16`:
  raw 65 536. The frame's dot products and the rounding change `d . n` by
  fewer than 50 raw units, so `d . n >= 2^15` raw units
  (`MIN_BOUNCE_COSINE_RAW`, an elevation above about `7.6e-6` radians). No
  direction is below or along the surface. In the ideal map only the last
  `u` value, a fraction `2^-32` of samples, is that low.
- **Accuracy.** Each component is within 4 raw units of the ideal map on the
  same frame (`sqrt(u) cos(2 pi v) t + sqrt(u) sin(2 pi v) b +
  sqrt(1 - u) n` in exact arithmetic): the square roots are within one raw
  unit, the sine and cosine within one, and the frame rows are unit to a few
  raw units. Largest value measured: 2.1 raw units.
- **Length.** Within 8 raw units of 1.0 (measured: 3).

The distribution is checked as a regression guard with stated thresholds, not
a statistical test: over 4 096 stratified sampler points the mean of
`cos(theta)` is within `2^-12` of 2/3; over 1 024 points (a (0, 10, 2)-net,
whose `u` and `v` are each stratified in 1 024 intervals) the cosine bands
`[0, 1/4)`, `[1/4, 1/2)`, `[1/2, 3/4)`, `[3/4, 1]` hold within one point of
64, 192, 320 and 448, and each quarter turn of azimuth within two of 256.

**Cost.** Two integer square roots, one table sine and cosine, nine `i128`
multiplications: 27 ns per direction on the benchmark machine. Two square
roots are inherent to this map; a concentric disk map would trade one of them
for a division (the angle is a ratio of the disk coordinates). That has not
been measured, and nothing here is optimized.

### Radiometric Convention

**A directional light's value is irradiance.** `DirectionalLight::new(
toward_light, irradiance)` takes, per channel, the irradiance the light
delivers to a surface perpendicular to it. A diffuse surface of albedo `a` lit
at cosine `c` then reflects radiance `a * E * c / pi` toward every direction.
This is the convention of pbrt's distant light and of Mitsuba's directional
emitter, whose parameter is named `irradiance`: a light with no angular size
has no finite radiance, so its value scales irradiance. Under it a white surface lit straight from
above by `E = pi` reflects radiance 1.0; a light given as 3.0 lights a 0.7
surface to about 0.67.

**The cosine.** `DirectionalLight::cosine(normal)` is
`normal . unit(toward_light)`, exact in `i128`, rounded once to nearest (ties
to even) and clamped to `0..=1.0`. `unit(toward_light)` is normalized once,
when the light is built, by the same prescale and `FixedUnitVec3::new`, to
within one raw unit per component. The cosine is within 3 raw units of the
exact cosine with the given normal (measured: 1.2), and so within about 5 of
the cosine with the triangle's exact normal.

**The term.** `DirectionalLight::reflected_radiance(cosine)` is the radiance a
white surface reflects:

```text
weight = cosine * FRAC_1_PI           // one Q32.32 product, rounded to nearest, ties to even
term_c = irradiance_c * weight        // one product per channel, same rounding
```

Proven infallible: the weight is at most `1 / pi` and each irradiance at most
`2^48` raw. The albedo multiplies later, in the
[diffuse reflection](#diffuse-reflection). Choosing the convention without
`1 / pi` instead would multiply every directly lit pixel by pi; it is a
contract choice of version 1.

### Directional Light

`toward_light` is the direction from a surface to the light. Its largest
component magnitude must be exactly 1.0 (raw `2^32`), so it is a valid trace
direction as given, with no division at run time.

`SurfaceHit::light_sample(&light)` returns a `LightSample`:

- when `toward_light . N' <= 0` exactly (the light is along or behind the
  surface; `N'` the facing plane normal), there is no shadow ray and the
  radiance is zero;
- otherwise the shadow ray runs from the start point along `toward_light`,
  and the radiance is `light.reflected_radiance(light.cosine(normal))` with
  the hit frame's normal. The cosine may round to zero for a light within
  `2^-32` of the horizon: a shadow ray, but no light.

The renderer queries the shadow ray with any-hit, minimum distance zero and
`SHADOW_T_MAX`, and passes the answer to `LightSample::radiance(visible)`,
which returns the term when visible and zero otherwise.

**Why `SHADOW_T_MAX` suffices.** Along the axis of `toward_light`'s 1.0
component, a distance `t` moves exactly `t` units. The start point and every
triangle lie inside the trace domain (below 256 units on every axis), so they
differ by less than 512 units on that axis, and every occluder is at a
distance below 512: raw `2^41`. Every limit of at least `2^41` gives the same
answer, so the limit is not pixel-affecting; the test plan checks an occluder
on the far face of the domain.

### Sky

`SkyLight::new(radiance)` is the constant radiance of every ray that leaves
the scene: a camera ray that hits nothing, or a bounce ray that hits nothing.
`SkyLight::radiance()` is that value; it names the step.

### Diffuse Reflection

`Albedo::reflect(direct, indirect)` is the radiance a diffuse surface sends
back along the incoming ray:

```text
out_c = albedo_c * (direct_c + indirect_c)    // exact sum, one product rounded to nearest, ties to even
```

`direct` is the light's term (`LightSample::radiance`), `indirect` the
radiance arriving along the bounce ray. **The bounce has no pi.** The
Lambertian BRDF is `albedo / pi`, the bounce direction's density is
`cos(theta) / pi`, so the one-sample estimate
`(albedo / pi) * L * cos(theta) / (cos(theta) / pi)` is `albedo * L`: the
cosine and `1 / pi` cancel exactly, and the function does not compute them.
Because each albedo channel is at most 1.0, the product is never larger than
the sum; the sum is checked, and `None` when it is not representable.

### Order Of Operations

Fixed-point multiplication is not associative, so the order is part of the
contract. For one sample of one pixel, with `t_max = i64::MAX` raw for camera
and bounce rays:

```text
1. camera ray:  ray0 = camera.ray(sampler.camera_sample(i))                 // pair 0
2. primary:     closest_hit(ray0); none -> L = sky.radiance(); done
3. hit1:        h1 = SurfaceHit::new(&surface[p1], &ray0, &hit1)
4. direct 1:    s1 = h1.light_sample(&light)
                D1 = s1.radiance(s1.shadow_ray() is Some and any_hit(it, SHADOW_T_MAX) is false)
5. bounce:      ray1 = h1.bounce_ray(sampler.point(i, 1))                   // pair 1
                closest_hit(ray1); none -> B = sky.radiance()
                else h2 = SurfaceHit::new(&surface[p2], &ray1, &hit2)
                     D2 = direct at h2, as in step 4
                     B  = albedo[p2].reflect(D2, 0)
                (no bounce: B = 0)
6. sample:      L = albedo[p1].reflect(D1, B)
```

Every multiplication rounds to nearest with ties to even; every addition is
exact. Per channel the products are, in order: `cosine * FRAC_1_PI`, then
`irradiance * weight` (inside `reflected_radiance`), then
`albedo * (direct + indirect)` at the second hit and at the first. The
directional light uses no sampler dimension. The sample then goes to
`PixelAccumulator::add_sample`: with albedo at most 1, the light at most
65 536 and one bounce, `L <= 65 536 / pi + 65 536 < 2^17` units, far inside
the accumulator's `2^52` raw limit.

The test plan locks the radiance bits of composed paths: one hand computed
(the sun straight above, the bounce seeing the sky, two ties to even) and
three from the reference model (a bounce onto a lit wall, the same path in
shadow, and a camera ray that sees the sky).

### Inputs And Refusals

| Input | Accepted | Refusal |
| --- | --- | --- |
| `toward_light` | Largest component magnitude exactly 1.0 | `ShadingError::LightNotNormalized` |
| Light irradiance, sky radiance | Each channel in `0..=MAX_LIGHT_RAW` | `NegativeRadiance` when one is negative, else `RadianceTooLarge` |
| Albedo | Each channel in `0..=1.0` | `ShadingError::AlbedoOutOfRange` |
| Triangle | Nonzero area | `TriangleSurface::new` returns `None` |
| Mesh for vertex normals | See [Vertex Normals](#vertex-normals) | `VertexNormalError` |
| Sampler point | Each coordinate in `[0, 1)` | `cosine_direction` and `bounce_ray` return `None`; `SampleOutsideUnitInterval` |
| Hit point | `point_at(ray, hit.distance)` representable | `SurfaceHit::new` returns `None`; `HitPointUnrepresentable` |
| Hit point | Each coordinate below `TRACE_COORD_LIMIT_RAW + 2` raw units in magnitude | `None`; `HitPointOutsideTraceDomain` |
| Hit point | Within two raw-unit steps of the arrival side | `None`; `NotOnArrivalSide` |
| Start point | Inside the trace domain after its step | `None`; `StartOutsideTraceDomain` |

**Fast path and diagnostic path.** Each of the three secondary-ray steps
comes in two forms. The `Option` forms, `SurfaceHit::new`,
`SurfaceHit::bounce_ray` and `ShadingFrame::cosine_direction`, are the fast
path: the 0.7.1 code, plus in `SurfaceHit::new` one comparison per coordinate
that refuses a hit point beyond reach of the domain before the plane test (so
that test stays inside `i128`). They say only whether an input was refused.
The `try_` forms, `SurfaceHit::try_new`, `SurfaceHit::try_bounce_ray` and
`ShadingFrame::try_cosine_direction`, are the diagnostic path: they return
`Result<_, SecondaryRayError>` and say why. A renderer calls the fast form per
hit and calls the `try_` form only after a `None`:

```rust
let Some(start) = SurfaceHit::new(&surface, &ray, &hit) else {
    let why = SurfaceHit::try_new(&surface, &ray, &hit).err();
    return Err(RenderError::Shading(why));
};
```

The two forms are separate code, so that richer errors cost nothing on the
success path: routing the fast forms through the `try_` forms made the
per-hit step about 4 % slower, even when the shared code carried only a
one-byte refusal code (see the
[performance chapter](../performance/README.md#typed-shading-refusals-flyingrobotsbunny280)).
Their agreement is a tested contract (`OPTICS-REQ-043`): over generated
inputs from a fixed and a fresh seed, and on every constructed refusal, the
fast form returns `Some(x)` exactly when the `try_` form returns `Ok(x)`, with
equal values, and both match the 0.7.1 code retained in the tests.

Each variant carries the values the check saw: the hit distance, the hit point
before any step, the stepped start point, or the sample as given. `try_new`
checks in the order of the table: representable, near the domain, arrival
side, inside the domain. A hit the trace kernel reported for a ray and
triangle inside the domain can be refused only by the last check, on the
domain's boundary; the other three mean the hit's distance or face does not
belong to that ray and triangle. `try_bounce_ray` refuses a sample as
`try_cosine_direction` does; its `BounceRayRefused` variant is defensive
(`TraceRay::new` refusing the ray), and no input is known to reach it, because
the start point is inside the domain and every bounce direction is nonzero
with components in `-1.0..=1.0`. `SecondaryRayError` is `#[non_exhaustive]`.

### Accuracy And Cost

| Step | Bound | Measured largest |
| --- | --- | --- |
| Unit normal | Each component within 1 raw unit of the exact unit normal | — |
| Frame | Within half a raw unit of the formula; orthonormal to 8 raw units | 4.4 raw units |
| Start point | At most 2 raw-unit steps along one axis from `point_at` | 2 steps (1 808 of 99 616 generated hits) |
| Bounce direction | Within 4 raw units of the ideal map; length within 8; `d . n >= 2^15` | 2.1; 3 |
| Light cosine | Within 3 raw units, clamped to `0..=1.0` | 1.2 |
| Vertex normal | Each component within 1 raw unit of the exact unit vector along the face sum | — |
| Smooth normal | Each component within `1 + 2^-26 / rho` raw units of the exact interpolation's unit vector (1 when `det < 2^62`); a shared normal exactly | — |

Timings on an Apple M5 Pro in the `rust-dev:1.96.0` container, run alone: the
`shade/` cases are the p50 across 11 interleaved rounds of `cargo run --locked
-p xtask -- perf --base origin/main --rounds 11 --align-code --filter shade/`
(base `7fdd2e4`, where they are new); the sampler and traced-ray rows are the
p50 of one direct harness run (`bunny-perf --filter`) in the same session, for
scale. A reading, not a speed claim:

| Case | ns per call |
| --- | ---: |
| `shade/surface_new` (per triangle, once) | 84.1 |
| `shade/bounce_direction` | 26.9 |
| `shade/hit_shading` (start point, light sample and shadow ray, bounce ray, reflection) | 73.7 |
| `shade/vertex_normals` (per vertex of a whole-mesh build; base `3c395f9`, where it is new) | 68.1 |
| `shade/smooth_normal` (per sphere hit of the benchmark scene; run 1 of ledger entry 13, base `e7a4fab`) | 72.4 |
| `shade/smooth_hit_shading` (the whole smooth-shaded step on the same hits) | 171.2 |
| `shade/flat_hit_shading_mesh` (the flat step of `shade/hit_shading` on the same hits) | 67.5 |
| `sample/sampler_point` | 19.7 |
| `trace/closest_hit` (one camera ray through the benchmark scene) | 226.8 |

A one-bounce path traces up to four rays (two closest-hit, two shadow) and
shades up to two hits, so shading is a small part of a sample's cost. The
per-hit step includes the bounce direction (27 ns of its 74 when timed alone)
and building two `TraceRay`s, which compute three reciprocals each.

## Pixel Values

A renderer adds the linear radiance of each sample to its pixel, resolves the
mean, maps it through a tone curve to a display-referred value and quantizes
that to an output code. Each step is a pure integer function in
`crates/bunny-optics/src/pixel/`: no floating point, no allocation, no I/O and
no image type. The caller owns every buffer; an accumulator is a small value
the caller stores wherever it keeps per-pixel state, and `write_rgba8` writes
into a slice the caller provides.

**What a sample is.** A sample is one sample index of one pixel: the
radiance gathered along the ray of `PixelSampler::camera_sample(sample)`, with
every later random decision of that path drawn under the same index from its
own dimension pair. `PixelAccumulator` takes at most 1 024 samples per pixel
(`MAX_SAMPLES_PER_PIXEL`), sample indices 0 to 1 023. The sampler stratifies
every aligned power-of-two block of indices and its test plan counts that up
to 1 024; so a pixel with a power-of-two count up to 1 024, indices from 0,
gets stratified positions, and its mean is the rounded shift described under
[Resolve](#resolve). Other counts up to 1 024 are accepted and resolved by the same
rule, but their positions are a partial block. Both the sample values and the
sums are independent of evaluation order, so tiles and threads cannot change a
pixel.

Radiance is linear light per channel (red, green, blue, in that order) as a
`FixedQ32_32`. Display values are `FixedQ32_32` in `[0, 1]`. Every rounding is
to nearest with ties to even, the default of the
[numeric constitution](../../NUMERIC_CONSTITUTION.md), except the floor inside
the sRGB table interpolation.

### Accumulation

`PixelAccumulator` holds three `i64` channel sums of raw Q32.32 units and a
sample count.

| Limit | Value | Refusal |
| --- | --- | --- |
| Sample channel | `0..=2^52` raw (0 to 1 048 576.0) | `NegativeRadiance` below zero, `RadianceTooLarge` above |
| Samples per pixel | `1..=1024` (`MAX_SAMPLES_PER_PIXEL`) | `TooManySamples` |

**Bit budget.** A sum of at most `2^10` samples of at most `2^52` raw each is
at most `2^62` raw, below `i64::MAX`, so no addition overflows and no wider
accumulator is needed. A radiance of 65 536 per channel is `2^48` raw; 1 024
such samples sum to `2^58` raw, with 32 times headroom left. The `2^52` limit
accepts samples up to 1 048 576.0 per channel and still leaves one bit spare.

`add_sample` checks the count first, then the channels in order (red, green,
blue), and returns the first error. `merge` adds another accumulator's sums and
count, refusing with `TooManySamples` when the two together hold more than
1 024 samples. On any error the accumulator is unchanged: it never saturates
and never wraps.

**Order independence.** The sums are exact integer sums, and integer addition
is associative and commutative, so the sums and the count are identical
whatever order the samples arrive in and however they are split between
accumulators that are then merged (for example, one per thread or tile). This
is a property of the representation, not of a careful reduction order.

### Resolve

`resolve` returns, per channel, the sum divided by the count, rounded to
nearest with ties to even: `round_half_even(sum / count)`. For a power-of-two
count `2^k` this equals the sum shifted right by `k` with the same rounding
(the discarded `k` bits compared with one half, an exact half going to the even
neighbour); the implementation uses one integer division for every count. The
mean of accepted samples is at most `2^52` raw. An empty accumulator returns
`NoSamples`. This is a box reconstruction filter: every sample of a pixel has
equal weight.

### Tone Curves

A `ToneCurve` maps one linear channel to a display-referred value in `[0, 1]`
(raw `0..=2^32`). Inputs at or below zero give 0 and inputs at or above 1.0
give 1.0 for both curves: radiance above 1.0 is clipped, not compressed.

**`LinearClampV1`** returns the input clamped to `[0, 1]`.

**`SrgbV1`** is the sRGB transfer function of IEC 61966-2-1: `12.92 x` below
0.0031308 and `1.055 x^(1/2.4) - 0.055` above. The power cannot be computed
exactly in fixed point, so the upper segment is a table:

```text
x <= 0                        0
x >= 1.0                      1.0 (raw 2^32)
x <  TOE_END = 13 446 684     round_half_even(1292 * x / 100)
otherwise                     max(interpolate(x), 173 731 144)
```

with all values raw Q32.32. `TOE_END` is the first raw input at or above
0.0031308 (`ceil(0.0031308 * 2^32)`).

- **Knots.** 1 153 knots cover `[2^-9, 1]`: 128 equal segments per octave
  for the nine octaves `[2^(-e-1), 2^-e)`, `e = 0..=8`, plus a last knot at 1.0.
  Knot `i` sits at raw `2^(31 - e) + j * 2^(24 - e)` with `e = 8 - i / 128`
  and `j = i % 128`; its value is `1.055 x^(1/2.4) - 0.055` rounded to the
  nearest raw unit (computed with 60-digit arithmetic; no knot is within
  `2.7 * 10^-4` raw units of a tie). Knots below `TOE_END` other than the one
  that starts its segment are never read; they keep the indexing uniform.
- **Interpolation.** For `x` in octave `e`, segment width `w = 2^(24 - e)` is a
  power of two, so `interpolate(x) = K[i] + floor((K[i + 1] - K[i]) * f / w)`
  with `f` the offset of `x` in its segment: one multiplication and one shift.
- **The hold at the toe end.** The standard's two pieces do not meet: at
  0.0031308 the toe gives 0.040449936 and the power curve 0.040449907. Raw
  input `TOE_END - 1` gives 173 731 144 through the toe, while the table gives
  173 730 860 at `TOE_END`. The power segment is therefore held at the toe's
  last value, 173 731 144, until it rises past it, which takes 23 inputs. This
  keeps the curve monotone and changes no 8-bit or 16-bit code.

**Bound.** For every input, `|SrgbV1(x) - srgb(x)|` is at most 6 400 raw units
(`1.49 * 10^-6`) of display value: the chord of a concave curve lies below it
by at most 6 256 raw units (in the top octave), knot rounding adds half a raw
unit and the floor one. That is at most 0.098 of a 16-bit code and 0.00038 of
an 8-bit code. The table output is never more than half a raw unit above the
exact curve (only knot rounding can put it above); the hold is at most 110 raw
units above it.

**Monotone.** Within a segment the value is the floor of an affine function
with a non-negative slope; consecutive knots increase; at each knot the value
equals the knot, which is at least the value one input before it; and the hold
joins the toe at an equal value. So `SrgbV1` never decreases.

A filmic or Reinhard-style operator, or an exposure control, is not provided:
the two curves above cover the current consumers, and a new operator would be
a new `ToneCurve` variant with its own identity.

### Output Codes

`quantize_8(v)` and `quantize_16(v)` clamp `v` to `[0, 1]` and return
`round_half_even(255 * v)` and `round_half_even(65535 * v)`, from raw values:
`round_half_even(raw * 255 / 2^32)`. Both are monotone. Because 255 and 65 535
are odd, the only display value in `[0, 1]` that lands exactly on a half code
is 0.5 (127.5 and 32 767.5), which goes to the even codes 128 and 32 768.

`ToneCurve::encode_8` and `encode_16` are `quantize_8(apply(x))` and
`quantize_16(apply(x))`.

`write_rgba8(out, curve, [r, g, b])` writes `[encode_8(r), encode_8(g),
encode_8(b), 255]` into `out`, which must be exactly four bytes long (one pixel
of a caller's row-major RGBA8 buffer, such as one item of
`chunks_exact_mut(4)`). Any other length returns
`WrongSliceLength { expected: 4, actual }` and leaves `out` unchanged.

### Accuracy Of The Codes

The codes of `SrgbV1` are not always the correctly rounded sRGB codes
`round(255 * srgb(x))` and `round(65535 * srgb(x))`: near each code boundary
the table's small error can put an input on the other side. The test plan finds
exactly where each code begins. With a 60-digit reference over every raw input
in `[0, 1]`:

| Output | Codes that begin late | Most inputs late | Inputs that begin early | Inputs differing in `[0, 1]` |
| --- | --- | --- | --- | --- |
| 8-bit | 245 of 255 | 9 447 (`2.2 * 10^-6` linear) | none | 486 760 of `2^32` (0.011 %) |
| 16-bit | 62 847 of 65 535 | 9 522 | 4 codes, 1 input each (in the toe) | 125 352 888 (2.9 %) |

Every such input is within one code of the correctly rounded code, and only
in the direction stated. The codes in the toe are exact for 8-bit; the four
16-bit toe codes that begin one input early do so because the toe's display
value is rounded before quantization. The test plan asserts the stated bounds
(at most 9 600 inputs late and 2 early) against an `f64` reference.

A curve whose 8-bit codes are exactly the correctly rounded ones is possible
(a table of the 255 input thresholds and a binary search), but it would not be
the composition of a tone curve and `quantize_8`; it would be a new identity.

### Versioned Identities

These names are contracts. Each one names exact output bits for every input,
locked by golden vectors; any change to any output is a new version with a new
name, and the old one keeps its bits.

| Identity | Function |
| --- | --- |
| `bunny-tone-srgb/1` | `ToneCurve::SrgbV1`, as defined above, including the knot table, the floor and the hold. |
| `bunny-tone-linear-clamp/1` | `ToneCurve::LinearClampV1`. |
| `bunny-unorm8/1` (`UNORM8_IDENTITY`) | `quantize_8`. |
| `bunny-unorm16/1` (`UNORM16_IDENTITY`) | `quantize_16`. |
| `bunny-rgba8-opaque/1` (`RGBA8_OPAQUE_IDENTITY`) | `write_rgba8`'s bytes: red, green, blue `bunny-unorm8/1` codes of the toned channels, then alpha 255. |

The accumulation limits and the mean's rounding are part of
`PixelAccumulator`'s contract and change only with a new type or function.
