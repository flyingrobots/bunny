# Exact Lerp and Ratios (`bunny-lerp/1`)

This chapter states how Bunny interpolates between two Q32.32 values and how it turns a
ratio of two integers into one. Each function computes the exact rational value in a wide
integer and rounds it once, to nearest with ties to even. The identity string is
`bunny-lerp/1` (`bunny_num::LERP_CONTRACT_ID`). The [test plan](test-plan.md) holds the
oracles, golden vectors and mutation checks. It answers flyingrobots/bunny#303 and
flyingrobots/bunny#111.

| Function | Crate | Exact value rounded | Refused (`None`) |
| --- | --- | --- | --- |
| `FixedQ32_32::lerp(a, b, t)` | `bunny-num` | the step `(b - a) t` | a result outside Q32.32 |
| `FixedQ32_32::from_ratio(n, d)` | `bunny-num` | `n / d` | `d = 0`; a result of `2^31` or more |
| `FixedQ32_32::inverse_lerp(a, b, x)` | `bunny-num` | `t = (x - a) / (b - a)` | `a = b`; a result outside Q32.32 |
| `FixedQ32_32::remap(x, [in_lo, in_hi], [out_lo, out_hi])` | `bunny-num` | the step `(x - in_lo) (out_hi - out_lo) / (in_hi - in_lo)` | `in_lo = in_hi`; a result outside Q32.32 |
| `x.checked_clamp(lo, hi)` | `bunny-num` | nothing: exact | `lo > hi` |
| `FixedVec3::lerp(a, b, t)` | `bunny-linalg` | each component's step | any component refused |
| `lerp_channels(a, b, t)` | `bunny-linalg` | each channel's step | any channel refused |

## Lerp

On raw values, with `U = 2^32` for `1.0`:

```text
lerp(a, b, t) = a + round_half_even((b - a) * t / U)
```

The step from `a` is rounded, then added to `a`, which is already a whole raw value. So
the result is within half a raw unit of the exact `a + (b - a) t`, and moving both ends by
the same amount moves the result by exactly that amount:
`lerp(a + c, b + c, t) == lerp(a, b, t) + c`. On a tie the even choice is made for the step,
not for the sum: `lerp(5, 6, 0.5)` is `5` raw, where rounding the exact `5.5` would give `6`.

- **Domain.** Every `t` is accepted. Inside `[0, 1]` the result lies between `a` and `b`
  and is never refused; outside it the line is extrapolated (overshooting eases need this).
- **Ends.** `t = 0` returns exactly `a` and `t = 1.0` exactly `b`, for every `a` and `b`,
  because the step is then `0` or exactly `b - a`.
- **Order.** For fixed `a` and `b` the result is non-decreasing in `t` when `b >= a` and
  non-increasing when `b < a`, because the exact step is monotone and rounding preserves
  order. It is strictly monotone when `|b - a| >= 1.0`: one raw unit of `t` then moves the
  exact step by at least one raw unit, and `round(x + 1) = round(x) + 1`. Below that,
  neighbouring `t` can give the same result (`lerp(0, 1 raw, 0)` and `lerp(0, 1 raw, 1 raw)`
  are both `0`).
- **Overflow.** `None` only when the rounded result is outside `i64`. `b - a` itself never
  refuses: it is held in `i128`, so `lerp(i64::MIN, i64::MAX, 1.0)` is `i64::MAX`.

The intermediate widths:

| Quantity | Bound on its magnitude | Held in |
| --- | --- | --- |
| `b - a` | `2^64 - 1` | `i128` |
| `(b - a) t` | `(2^64 - 1) 2^63 < 2^127` | `i128`, exact |
| the rounded step | `2^95` | `i128` |
| `a` plus the step | `2^63 + 2^95` | `i128`, then checked against `i64` |

The step is rounded by an arithmetic shift (the floor) and a mask (the remainder in
`[0, 2^32)`): above half a unit rounds up, below rounds down, and exactly half rounds to the
even floor or floor plus one. As in the [rounding chapter](../fixed-point-rounding/README.md),
this is ties to even on the signed value: a step of `-1/2` raw gives `0` and `-3/2` gives `-2`.

## From Ratio

```text
from_ratio(n, d) = round_half_even(n * U / d)      n, d: u32
```

`n * 2^32` is below `2^64`, so the quotient and remainder are exact in `u64`. A tie cannot
occur: `n 2^32 / d = q + 1/2` would mean `n 2^33 = (2q + 1) d`, so `2^33` would divide `d`,
which is below `2^32`. The result is therefore always the unique nearest raw value. It is
`None` for `d = 0` (including `0 / 0`) and when it is `2^31` or more (raw `2^63`); the
largest value reached is `from_ratio(2^32 - 1, 2) = 2^31 - 1/2`. `from_ratio(d, d)` is
exactly `ONE` for every `d > 0`.

## Vectors and Channels

`FixedVec3::lerp` and `lerp_channels` (for `[FixedQ32_32; 3]`) apply the scalar `lerp` to
each component with the same `t`. Each component is rounded once on its own, so the ends are
exact and the result is within half a raw unit per component. If any component is refused
the whole result is `None`. `lerp_channels` interpolates the channels it is given and assumes
no colour space: sRGB-encoded values are interpolated as encoded, so decode them to linear
first (for example with `bunny-optics`'s `srgb8_to_linear`) to interpolate light.

## Clamp, Min and Max

`x.checked_clamp(lo, hi)` returns `lo` when `x < lo`, `hi` when `x > hi` and `x` otherwise.
It is exact: the result is always one of its three arguments, so nothing is rounded and
nothing overflows. `lo == hi` returns `lo`. An inverted range, `lo > hi`, is refused with
`None`; the bounds are never swapped. That is the one difference from `Ord::clamp`, which
`FixedQ32_32` also has (it derives `Ord`) and which panics on an inverted range. The
checked form has its own name so that existing calls to `clamp` keep their meaning.

Min and max need no new function: `FixedQ32_32` orders by its raw `i64`, which is the order
of the values it stands for, so `Ord::min` and `Ord::max` are exact for every pair.

## Inverse Lerp

```text
inverse_lerp(a, b, x) = round_half_even((x - a) * U / (b - a))
```

This is the `t` at which the line from `a` to `b` passes through `x`, computed exactly and
rounded once to the nearest raw unit of `t`, an exact half going to the even value. So the
result is within half a raw unit (`2^-33`) of the exact `(x - a) / (b - a)`.

- **Degenerate range.** `a == b` is refused with `None`: there is no division by zero and
  no silent zero.
- **Domain.** Every `x` is accepted. Between `a` and `b` the result is in `[0, 1]`; outside
  it `t` is extrapolated, as `lerp` extrapolates for `t` outside `[0, 1]`. Either order of
  `a` and `b` is allowed: `inverse_lerp(1.0, 0, 0.25)` is `0.75`.
- **Ends.** `x = a` returns exactly `0` and `x = b` exactly `1.0`.
- **Overflow.** `None` when the rounded `t` is outside `i64`. With `b - a = 1.0`, `t` is
  exactly `x - a` raw, so `inverse_lerp(0, 1.0, i64::MAX raw)` is `i64::MAX` raw and moving
  `a` down one raw unit refuses it. A tie can cross the edge: with `b - a = 2.0` and
  `x - a = 2^64 - 1` raw, the exact `t` is `2^63 - 1/2` raw, which rounds to the even
  `2^63`, outside `i64`.

It is `remap(x, [a, b], [0, 1.0])` bit for bit, and its widths are that function's with
`out_hi - out_lo = 2^32`: `|x - a|` and `|b - a|` are at most `2^64 - 1`, and
`(x - a) 2^32` is below `2^96`.

### Round Trips

Fixed point cannot make `inverse_lerp` and `lerp` exact inverses of each other, because
one direction always loses resolution: with `|b - a|` below `1.0` many raw `t` give one
`x` (`lerp(0, 1 raw, 0)` and `lerp(0, 1 raw, 1 raw)` are both `0`), and with it above `1.0`
many raw `x` give one `t`. What holds, with `d = |b - a|` in raw units and both calls
defined:

| Round trip | Exact when | Otherwise within (raw units) |
| --- | --- | --- |
| `lerp(a, b, inverse_lerp(a, b, x))` against `x` | `d <= 2^32` (`\|b - a\| <= 1.0`) | `floor(1/2 + d / 2^33)`, at most `2^31` (`0.5`) |
| `inverse_lerp(a, b, lerp(a, b, t))` against `t` | `d >= 2^32` (`\|b - a\| >= 1.0`) | `floor(1/2 + 2^31 / d)`, at most `2^31` (`0.5`) |

Proof of the first. Let `t* = (x - a) U / (b - a)` exactly and `t` its rounding, so
`|t - t*| <= 1/2`. `lerp` rounds `(b - a) t / U = (x - a) + (b - a)(t - t*) / U`, which is
within `d / 2U` of `x - a`, and rounding adds at most `1/2`. The result and `x` are whole
raw values, so their difference is a whole number at most `1/2 + d / 2^33`. When
`d < 2^32` that is below 1, so the difference is 0; when `d = 2^32`, `t*` is whole and
nothing is rounded.

Proof of the second. `lerp` gives `a + s` with `s = (b - a) t / U + e`, `|e| <= 1/2`.
`inverse_lerp` then rounds `s U / (b - a) = t + e U / (b - a)`, within `2^31 / d` of `t`,
and rounding adds at most `1/2`. When `d > 2^32` the whole difference is below 1, so it
is 0; when `d = 2^32`, `e = 0`.

The bounds are for the case where both calls are defined. Near the ends of the raw range
the second call can be refused when the first is not, because its exact result may lie up
to the bound beyond `x` (or `t`), outside Q32.32.

## Remap

```text
remap(x, [in_lo, in_hi], [out_lo, out_hi])
    = out_lo + round_half_even((x - in_lo) (out_hi - out_lo) / (in_hi - in_lo))
```

The step from `out_lo` is computed exactly and rounded once, then added to `out_lo`, as in
`lerp`. So the result is within half a raw unit of the exact value **whatever the widths of
the two ranges**: composing `inverse_lerp` and `lerp` instead would round twice, with an
error that grows with `|out_hi - out_lo| / |in_hi - in_lo|`. As in `lerp`, a tie goes to
the even step, not the even sum: `remap(1 raw, [0, 2 raw], [5 raw, 6 raw])` is `5` raw.

- **Degenerate input range.** `in_lo == in_hi` is refused with `None`. A degenerate output
  range is allowed: `out_lo == out_hi` maps every `x` to `out_lo`.
- **Domain.** Every `x` is accepted and extrapolated outside the input range. Either range
  may be reversed.
- **Ends.** `x = in_lo` returns exactly `out_lo` and `x = in_hi` exactly `out_hi`, for
  every pair of ranges, because the step is then `0` or exactly `out_hi - out_lo`.
- **Order.** For fixed ranges the result is monotone in `x`: non-decreasing when the two
  ranges run the same way and non-increasing otherwise.
- **Lerp and inverse lerp.** `remap(t, [0, 1.0], [a, b])` is `lerp(a, b, t)` and
  `remap(x, [a, b], [0, 1.0])` is `inverse_lerp(a, b, x)`, bit for bit, refusals included:
  each is the same exact step rounded by the same rule.
- **Overflow.** `None` only when the rounded result is outside `i64`.

The ranges are passed as `[lo, hi]` pairs because Bunny's core functions take at most four
parameters (`CODE_STANDARDS.md`, Rule 9).

The intermediate widths:

| Quantity | Bound on its magnitude | Held in |
| --- | --- | --- |
| `x - in_lo`, `out_hi - out_lo`, `in_hi - in_lo` | `2^64 - 1` | `i128` |
| the product of the first two | `(2^64 - 1)^2 < 2^128` | `u128`, exact (the sign is kept apart) |
| the quotient and remainder by the third | below `2^128` and `2^64` | `u128`, exact |
| the rounded step | refused above `2^64 - 1` | `u128`, then `i128` |
| `out_lo` plus the step | `2^63 + 2^64 - 1` | `i128`, then checked against `i64` |

The sign of the step is the parity of the negative signs among the three differences; the
magnitude is rounded, and since rounding to nearest with ties to even is symmetric about
zero, that is the same as rounding the signed step. A rounded step above `2^64 - 1` is
refused before the sum: no `out_lo` can bring it back into range, since `|out_lo|` is at
most `2^63`. The widest step that is accepted is exactly `2^64 - 1`, in
`remap(i64::MAX raw, [i64::MIN raw, i64::MAX raw], [i64::MIN raw, i64::MAX raw])`, which
is `i64::MAX` raw.
