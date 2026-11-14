# Optics

This chapter defines Bunny's current contract for camera, ray-generation,
sampling and pixel-value math in `bunny-optics`: a deterministic pinhole
camera that turns pixel samples into [trace rays](../ray-tracing/README.md),
the deterministic sample hash and pixel sampler that choose those samples and
every later random decision, and the functions that turn the radiance those
rays gather into 8-bit and 16-bit output codes.

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
  `UNORM16_IDENTITY` and `RGBA8_OPAQUE_IDENTITY`.

Evidence is listed in the [test plan](test-plan.md). Mapping unit-square
points to directions on a hemisphere, and shading (#160), are not part of this
chapter. The camera uses the same conventions as `FixedMat4::try_look_at`
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
