# Angles And Trigonometry

This chapter is the current reference for how Bunny represents angles and evaluates sine
and cosine. The [test plan](test-plan.md) says how it is verified. It answers issue #110.

## Canonical Angle

A canonical angle is a `FixedAngle`: a fraction of one full turn stored in a `u64`, with
`2^64` units per turn. One unit is about `3.4e-19` radians.

| Turn | Raw value |
| --- | --- |
| 0 | `0` |
| quarter turn (90°) | `1 << 62` (`FixedAngle::QUARTER_TURN`) |
| half turn (180°) | `1 << 63` (`FixedAngle::HALF_TURN`) |
| three quarters (270°) | `3 << 62` |

This is the binary angle measurement long used in game engines. Bunny chose it over radians,
degrees or a Q32.32 count of turns because:

- **Every `u64` is a valid angle, and there is nothing to normalize.** Adding, subtracting
  and negating wrap at a full turn exactly (`wrapping_add`, `wrapping_sub`, `wrapping_neg`).
  Whole turns disappear without rounding.
- **Range reduction is a bit mask.** The top two bits are the quadrant, the next ten pick a
  table segment, and the remaining 52 position the angle inside it. No `pi`, division or
  remainder is involved, so no reduction error can creep in for large angles.
- **Radians would need an irrational constant.** Any fixed-point `pi` is rounded, so a
  radian angle can never reach a quarter turn exactly; a binary angle reaches it by
  construction.

Positive angles turn counter-clockwise: the right-hand rule about `+Z`, as in the
[coordinate law](../coordinate-law/README.md).

`FixedAngle` deliberately has no ordering: "less than" is ambiguous on a circle.

## Conversions

| Constructor | Rule |
| --- | --- |
| `from_raw(u64)` | The raw turn units. |
| `from_turns(FixedQ32_32)` | Exact: the Q32.32 fraction becomes the top 32 bits; whole turns are discarded; negative turns wrap. |
| `from_degrees(FixedQ32_32)` | `degrees × 2^64 / 360`, rounded half up on the magnitude. Whole multiples of 90° are exact. |
| `from_radians(FixedQ32_32)` | `radians × 2^64 / 2π`, rounded half up on the magnitude. |

The scale factors are stored with 64 fractional bits beyond Q32.32 (`round(2^96 / 360)` and
`round(2^96 / 2π)`), so their own rounding adds less than one unit for any input below
about 2³¹ turns. Both conversions round the magnitude and then apply the sign, so
`from_radians(-x)` is exactly `from_radians(x).wrapping_neg()`, and likewise for degrees.

There are no conversions out of `FixedAngle` yet.

## Sine And Cosine

`sin`, `cos` and `sin_cos` return Q32.32 values in `[-1, 1]`.

**Error bound: at most one raw Q32.32 unit (2⁻³² ≈ 2.3 × 10⁻¹⁰) from the exact value of the
angle.** The largest error measured over 4 196 611 angles (four million seeded angles plus
a sweep of segment boundaries and their neighbours) was 0.5001 raw units: the 0.5 is the
final rounding to Q32.32, and interpolation adds about 0.0001. The bound is checked against
`f64` trigonometry, whose own error is about a million times smaller than one raw unit.

Exact by construction:

- `sin` and `cos` of every multiple of a quarter turn are exactly `0`, `1` or `-1`.
- `sin(-a)` is exactly `-sin(a)`, and `cos(-a)` is exactly `cos(a)`.

### How it is computed

1. The quadrant (top two bits) selects which symmetry rebuilds the full wave:
   `(sin, cos)` of the angle within its quadrant becomes `(s, c)`, `(c, -s)`, `(-s, -c)` or
   `(-c, s)`.
2. Within the quadrant, sine is read from a table of 1 025 samples of `sin` over
   `[0, π/2]` (1 024 segments). Cosine is sine of the complementary position, so it uses
   the same table.
3. Between samples the value is **cubic Hermite** interpolated. The slope of `sin` is `cos`,
   which is the same table read from the other end, so the interpolation knows the exact
   slope at both ends of each segment at no extra storage cost.
4. The result, held with 62 fractional bits, is rounded to Q32.32 (ties to even).

The table is computed at compile time by an integer Taylor series with 62 fractional bits,
starting from `round(π/2 × 2^62)`. No floating point is involved at build time or at run
time, and there is no generator script or checked-in table to keep in sync.

## Trade-offs

| Choice | Benefit | Cost |
| --- | --- | --- |
| Lookup table | Each call is a few multiplications; identical on every target. | 1 025 × 8 bytes = 8.2 KB of read-only data. |
| Table built at compile time | No generator to run or review; the table cannot drift from its definition. | A few milliseconds more const evaluation when `bunny-num` compiles. |
| Cubic Hermite rather than linear interpolation | Error under one raw unit. Linear interpolation over the same table would err by up to about 3 × 10⁻⁷ (about 1 260 raw units), as Echo's `f32` table does. | Four more multiplications per call than linear. |
| 62-bit table values | Interpolation error stays far below the output's resolution. | 64-bit entries rather than 32-bit. |
| Binary angles | Exact wraparound and quadrant boundaries; reduction is free. | Radians and degrees round once at the boundary; callers must convert explicitly. |

Measured cost: `sin_cos` takes about 10 ns per call on an Apple M5 Pro, against about 7 ns
for `f64::sin_cos` on the same machine. See the [performance ledger](../performance/README.md#optimization-ledger).

## Non-Goals

Not provided yet: tangent, inverse functions (`asin`, `acos`, `atan`, `atan2`), conversions
from `FixedAngle` back to radians or degrees, angle interpolation (#111), and rotation
types (#109). Each needs its own error bound and golden vectors when it lands.
