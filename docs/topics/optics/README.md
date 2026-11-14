# Optics

This chapter defines Bunny's current contract for camera and ray-generation
math in `bunny-optics`: a deterministic pinhole camera that turns pixel
samples into [trace rays](../ray-tracing/README.md), and the deterministic
sample hash and pixel sampler that choose those samples and every later random
decision.

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
  compatible with Echo's.

Evidence is listed in the [test plan](test-plan.md). Mapping unit-square
points to directions on a hemisphere, and shading or pixel values (#222,
#160), are not part of this chapter. The camera uses the same conventions as `FixedMat4::try_look_at`
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
