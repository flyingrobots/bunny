# Exact Lerp and Ratios (`bunny-lerp/1`)

This chapter states how Bunny interpolates between two Q32.32 values and how it turns a
ratio of two integers into one. Each function computes the exact rational value in a wide
integer and rounds it once, to nearest with ties to even. The identity string is
`bunny-lerp/1` (`bunny_num::LERP_CONTRACT_ID`). The [test plan](test-plan.md) holds the
oracles, golden vectors and mutation checks. It answers flyingrobots/bunny#303.

| Function | Crate | Exact value rounded | Refused (`None`) |
| --- | --- | --- | --- |
| `FixedQ32_32::lerp(a, b, t)` | `bunny-num` | the step `(b - a) t` | a result outside Q32.32 |
| `FixedQ32_32::from_ratio(n, d)` | `bunny-num` | `n / d` | `d = 0`; a result of `2^31` or more |
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
