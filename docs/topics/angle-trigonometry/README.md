# Angles And Trigonometry

This chapter is the current reference for how Bunny represents angles, evaluates sine
and cosine, converts angles out to turns, degrees and radians, and finds the angle of a
direction with `atan2`. The [test plan](test-plan.md) says how it is verified. It answers issue #110.

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

## Conversions Out

Each conversion multiplies the raw angle by a scale exactly in `u128` and rounds once to
Q32.32, to nearest with ties to even. The results are non-negative: an angle is read as a
fraction of a turn in `[0, 1)`, counter-clockwise.

| Method | Exact value | Result | Bound |
| --- | --- | --- | --- |
| `to_turns()` | `raw / 2^64` turn | `[0, 1]` | correctly rounded: within 1/2 raw unit |
| `to_degrees()` | `raw × 360 / 2^64` | `[0, 360]` | correctly rounded: within 1/2 raw unit |
| `to_radians()` | `raw × 2π / 2^64` | `[0, 2π]` | within `1/2 + 2^-30` raw units |

- `to_turns` is `raw / 2^32` rounded and `to_degrees` is `raw × 45 / 2^29` rounded, so both
  are the nearest Q32.32 value to the exact one. Exact ties occur and go to even:
  `to_turns(2^31)` is `0` and `to_turns(3 × 2^31)` is `2` raw; `to_degrees(2^28)` is `22`
  raw (exactly 22.5) and `to_degrees(3 × 2^28)` is `68` (exactly 67.5).
- Angles within half a raw unit of a full turn round up to exactly `1.0` turn or 360
  degrees. Every multiple of a quarter turn converts exactly to turns and degrees.
- `to_radians` multiplies by `round(2π × 2^61)`, the most precise `2π` a `u64` holds, and
  rounds the product. The constant is within one half of `2π × 2^61`, which moves the
  product by less than `2^64 × 2^-1 / 2^93 = 2^-30` raw units. The result is therefore one
  of the two Q32.32 values either side of the exact one; it is not always the nearest.

Round trips:

- `to_turns(from_turns(t))` is exactly the fractional part of `t`, `t.rem_euclid(1)`, for
  every `t`, because `from_turns` keeps the 32 fractional bits and drops whole turns.
- `from_turns(to_turns(a))` is `a` rounded to the nearest multiple of `2^32` units (ties to
  even), wrapping a full turn to zero.
- The degree and radian round trips are not exact: each direction rounds.

## Atan2

`FixedAngle::atan2(y, x)` is the angle of the direction `(x, y)`, counter-clockwise from
`+x`. It returns `None` only at the origin, which has no direction; every other pair of
Q32.32 values, including `i64::MIN` components, has an angle.

**Error bound: within `1/2 + 2^-9` angle units (`2^-64` turn) of the exact angle.** The
result is the nearest angle, or its other neighbour when the exact angle lies within
`2^-9` units of a midpoint. In radians that is about `1.7 × 10⁻¹⁹`, far below the
resolution of the Q32.32 inputs, so for any practical input the angle is as exact as the
direction it was given.

Range and branch: the result is any `FixedAngle`, `[0, 1)` turn. `+x` gives `ZERO`, `+y`
`QUARTER_TURN`, and the negative x axis exactly `HALF_TURN`. A direction just above `-x`
is at or just below a half turn and one just below it at or just above, so the result
never jumps there. It can round onto the half turn itself: for raw `x = i64::MIN` and
`y = ±1` the exact angle is about `1/π` of an angle unit from a half turn, and the result
is `HALF_TURN` (AT-TP-014). Read as a signed `i64`, the raw value is in `[-2^63, 2^63)`, which is the half-open
turn `[-1/2, 1/2)` with `-x` at `-1/2`.

Exact by construction:

- The axes and the diagonals give exact multiples of an eighth turn.
- `atan2(-y, x)` is exactly `atan2(y, x).wrapping_neg()`, `atan2(y, -x)` is exactly
  `HALF_TURN - atan2(y, x)` and `atan2(x, y)` is exactly `QUARTER_TURN - atan2(y, x)`,
  all wrapping.
- Doubling both inputs, where both still fit, gives the same bits: the inputs are scaled
  to a fixed width first.

### How atan2 is computed

1. The signs of `x` and `y` and whether `|y| > |x|` select one of eight octants; the
   magnitudes, with the smaller as `y`, give an angle in `[0, 1/8]` turn. The octant is
   rebuilt exactly with the symmetries above.
2. Both magnitudes are shifted left so the larger has its top bit at bit 123.
3. **CORDIC**: 72 steps rotate the vector toward `+x` by `atan(2^-i)` for `i = 0..72`, in
   the direction the sign of `y` picks, using only shifts and additions on `i128`, and add
   up the signed step angles. After the last step the angle left over is at most
   `atan(2^-71)` radians, about `0.0013` angle units.
4. The step angles are held in units of `2^-96` turn, so in the sum their rounding adds up
   to under `10⁻⁸` units. The sum is rounded once to the nearest angle unit, ties to even.

The step angles are computed at compile time by an integer Taylor series of
`atan(2^-i)`, scaled by `round(2^127 / 2π)`; `atan(1)` is exactly an eighth turn. There is
no generator script or checked-in table, and no floating point or division at run time.
The truncated shifts move the vector by at most one unit of a value of at least `2^123`
per step, an angle under `10⁻¹⁶` units over all 72 steps. The bound adds these to the
final half unit: `1/2 + 0.0013 + 10⁻⁸ < 1/2 + 2^-9`. AT-TP-013 checks the bound against
an independent oracle; it does not measure a tighter one, and none is claimed.

## Sine And Cosine

`sin`, `cos` and `sin_cos` return Q32.32 values in `[-1, 1]`.

**Error bound: at most one raw Q32.32 unit (2⁻³² ≈ 2.3 × 10⁻¹⁰) from the exact value of the
angle.** Final rounding to Q32.32 contributes up to 0.5 raw units and interpolation
a small fraction of a unit more. The bound is checked in tests (see the test plan) against
`f64` trigonometry, whose own error is about a million times smaller than one raw unit, and
by a float-free `sin^2 + cos^2` identity check.

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
4. The result, held with 62 fractional bits, is rounded to Q32.32 (ties to even). Exact
   ties occur: AT-TP-009 pins six positions where the value is exactly half a raw unit
   above an integer, three of each parity.

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

Measured cost (Apple M5 Pro, inside the rust-dev container, best p50 of 11 rounds, two
runs): `sin_cos` about 10.4 to 11.1 ns per call, `from_radians` about 0.9 ns, and
`f64::sin_cos` on the same angles in the same harness about 4.2 ns, a float routine that is
not deterministic across targets. Method, command and caveats are in the
[performance ledger](../performance/README.md#optimization-ledger).

## Non-Goals

Not provided yet: tangent, the inverse functions `asin`, `acos` and single-ratio `atan`,
signed conversions out (radians or degrees in `(-π, π]`), angle interpolation (#111),
and rotation types (#109). Each needs its own error bound and golden vectors when it
lands (flyingrobots/bunny#244). Rotation matrices from a `FixedAngle` are in
[Rotation](../rotation/README.md).
