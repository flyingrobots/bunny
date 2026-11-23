# Power Eases and Steps (`bunny-ease/1`)

This chapter states how Bunny evaluates easing curves in Q32.32. Part 1 covers the power eases
(GSAP's power1 to power4, `t^2` to `t^5`) and `steps`. Each power ease is a polynomial with
rational coefficients, so its exact value is a rational with a power-of-two denominator: Bunny
computes it exactly and rounds it once, to nearest with ties to even. The identity string is
`bunny-ease/1` (`bunny_num::EASE_CONTRACT_ID`); the functions are in `bunny_num::ease`. The
[test plan](test-plan.md) holds the oracles, golden vectors and mutation checks. It answers
flyingrobots/bunny#304. Part 3, the [sine eases](#sine-eases), answers flyingrobots/bunny#306.

| Function | Exact value rounded | Refused (`None`) |
| --- | --- | --- |
| `power_in(n, t)` | `t^n` | `t` outside `[0, 1]`; `n` outside `2..=5` |
| `power_out(n, t)` | `1 - (1 - t)^n` | the same |
| `power_in_out(n, t)` | `2^(n-1) t^n` for `t < 1/2`, `1 - 2^(n-1) (1 - t)^n` from `1/2` | the same |
| `steps(n, t)` | `k / n` for `k = floor(n t)` | `t` outside `[0, 1]`; `n = 0` |
| `sine_in(t)` | `1 - cos(t pi/2)`, within one raw unit | `t` outside `[0, 1]` |
| `sine_out(t)` | `sin(t pi/2)`, within one raw unit | the same |
| `sine_in_out(t)` | `(1 - cos(t pi)) / 2`, within one raw unit | the same |

## Domain

`t` is a `FixedQ32_32` in `[0, 1]`: raw `0` to `2^32` inclusive. Any other `t` (raw `-1`,
raw `2^32 + 1`, and everything beyond) is `None`; nothing is clamped or extrapolated. The power
eases take `n` in `2..=5` (`n = 2` is GSAP's power1, `n = 5` its power4) and refuse every other
`n`, including `1`; `steps` takes any `n >= 1` and refuses `0`. Inside the domain no ease is
ever refused.

## Power Eases

On raw values, with `r = t_raw` and `U = 2^32` for `1.0`:

```text
power_in(n, t)      = round_half_even(r^n / 2^(32(n-1)))
power_out(n, t)     = U - power_in(n, 1 - t)
power_in_out(n, t)  = round_half_even(r^n / 2^(31(n-1)))      r < 2^31
                    = U - power_in_out(n, 1 - t)                r >= 2^31
```

The first line is `t^n` rounded once. The others are the issue's definitions rounded once,
written by reflection. `1 - x` rounded to even is `U - round_half_even(x)` exactly, because `U`
is even: a value that is not a tie has one nearest neighbour either way, and on a tie both
sides pick the even one. So `power_out` is `round_half_even(1 - (1 - t)^n)` and the upper half
of `power_in_out` is `round_half_even(1 - 2^(n-1) (1 - t)^n)`. Each result is therefore within half
a raw unit of the exact value, the bound the Fidelity Rule needs, with no further approximation.

Ties occur, and they round to even. `r^n / 2^s` is a tie when `r^n` has exactly `s - 1` factors
of two, so `n v = s - 1` for `v` the number of factors of two in `r`. For `power_in` that has a
solution only at `n = 3` (`v = 21`): `power_in(3, 3 2^21 raw)` is exactly `13.5` raw, rounded to
`14`. For the lower half of `power_in_out` it has solutions at `n = 2` (`v = 15`) and `n = 4`
(`v = 23`): `power_in_out(2, 3 2^15 raw)` is `4.5`, rounded to `4`.

The intermediate widths, at the extreme `r = 2^32`:

| Quantity | Bound | Held in |
| --- | --- | --- |
| `r^n`, `n <= 5` | `2^160` | `upper 2^64 + lower`: a `u128` and a `u64`, while built |
| `upper` before the last factor, `upper * r` | `2^64`, `2^96` | `u128`, exact |
| `r^n` for the shift | `2^160` | the high and low `u128` halves of a 256-bit value |
| the shift `s` | `31` to `128` bits | the quotient by a shift, the remainder by a mask |
| the rounded result | `2^32` | `u64`, then `i64` |

`r^n` is built by multiplying by `r` five times at most. After `k` factors it is at most
`2^(32k)`, so before the last factor (`k <= 4`) its part above `2^64` is at most `2^64`, and
times `r` at most `2^96`: the `u128` never overflows. The quotient `r^n / 2^s` is at most
`2^32`, the exact value of an ease at `t <= 1`.

## Properties

- **Ends.** Every power ease is exactly `0` at `t = 0` and exactly `1.0` at `t = 1`: `r^n` is
  then `0` or `2^(32n)`, which the shift divides exactly.
- **Order.** Each ease is non-decreasing on `[0, 1]`: the exact polynomial is, the piecewise
  in-out curve is continuous and non-decreasing across its join, and rounding preserves order.
  It is not strict. Where the slope is below one raw unit per raw unit of `t`, neighbouring
  inputs repeat a value: near `t = 0` for `power_in` (`power_in(5, 0) = power_in(5, 1 raw) =
  0`), near `t = 1` for `power_out` (`power_out(2, 1 - 1 raw) = 1.0`), and at both ends for
  `power_in_out`.
- **Reflection.** `power_out(n, t) == 1 - power_in(n, 1 - t)` and
  `power_in_out(n, 1 - t) == 1 - power_in_out(n, t)`, bit for bit, for every `t` in the domain.
- **The middle.** `power_in_out(n, 1/2) = 1/2` exactly: `2^(n-1) (1/2)^n` and
  `1 - 2^(n-1) (1/2)^n` are both `1/2`.
- **The join.** The two halves meet at `1/2` with the same value and the same slope `n`, so the
  curve is continuous there. One raw unit of `t` either side of `1/2` moves the result by at
  least 1 and at most `n` raw units (for example `power_in_out(4, 1/2 - 1 raw) = 1/2 - 4 raw`).

## Steps

```text
steps(n, t) = from_ratio(k, n)      k = floor(n r / 2^32)
```

`n r` is at most `(2^32 - 1) 2^32`, exact in `u64`, so `k` is the exact `floor(n t)`: `t` in
`[k/n, (k+1)/n)` gives `k / n`, and `t = 1` gives `n / n = 1.0`. `k / n` is rounded once by
[`FixedQ32_32::from_ratio`](../lerp/README.md#from-ratio), where a tie cannot occur. The rounded
value can sit just below the interval's start: `steps(3, t)` for the least `t` with `3 t >= 1`
(raw `1 431 655 766`) is `round(1/3)`, raw `1 431 655 765`. `steps` is non-decreasing in `t`
and constant on each interval.

## Sine Eases

The sine eases need cosine, which has no exact rational value, so they are not rounded once
from an exact value like the power eases. They read [`FixedAngle::sin_cos`](../angle-trigonometry/README.md#sine-and-cosine),
which is within one raw unit of the exact sine and cosine, and inherit that bound. On raw
values, with `r = t_raw`, `U = 2^32` and `cos(a)` the raw cosine `sin_cos` returns for the
angle `a` in `FixedAngle` units (`2^64` a turn):

```text
sine_in(t)      = U - cos(r 2^30)
sine_out(t)     = U - sine_in(1 - t)                              (= sin(r 2^30))
sine_in_out(t)  = round_half_even((U - cos(r 2^31)) / 2)         r < 2^31
                = U - sine_in_out(1 - t)                          r >= 2^31
```

**The angle is exact.** `t pi/2` is `t` of a quarter turn, `2^62` units, so it is `r 2^62 /
2^32 = r 2^30` units, and `t pi` is `r 2^31`. For `r <= 2^32` these are at most `2^62` and
`2^63`, exact in a `u64`; no step before `sin_cos` rounds.

**The rounding steps.** `sin_cos` rounds its own result to Q32.32 once, ties to even.
`sine_in` subtracts it from `U` exactly; `sine_out` is two exact subtractions. `sine_in_out`
halves `U - cos`, an integer in `[0, 2^32]`, and rounds the half to nearest, ties to even: an
odd `U - cos` is a tie (`U - cos = 3`, at raw `t = 49 152`, gives `2`, not `1`). Each angle
the formulas read is at most a quarter turn, where `sin_cos` returns a cosine in `[0, 1]`, so
every result is in `[0, 1]`.

**The bound.** With `sin_cos` within one raw unit, `sine_in` and `sine_out` are within one
raw unit of the exact ease, and `sine_in_out` within `1/2 + 1/2 = 1` (half of `sin_cos`'s
error, plus the halving's half unit). **One raw unit is the contract for all three.** It is
exactly as strong as `sin_cos`'s bound, which its own tests check against `f64` at sampled
angles (AT-TP-004) rather than prove. Measured against an exact oracle (Machin's `pi` and the
sine series, with 124 fractional bits) at every `t = k / 2^20`, 1 048 577 points per ease:

| Ease | Largest error | At |
| --- | --- | --- |
| `sine_in` | `0.500 047` raw | `k = 17 981` |
| `sine_out` | `0.500 047` raw | `k = 1 030 595`, the mirror of `sine_in`'s |
| `sine_in_out` | `0.750 004` raw | `k = 314 529` |

100 000 seeded full-resolution `t` and every raw `t` within 4 096 of `0`, `1/2` and `1` stay
below those figures. The sweep covers `2^20 + 1` of the `2^32 + 1` inputs, so the measured
figures describe it and are not promised; flyingrobots/bunny#306 expected about 2 raw units,
and the derivation gives 1.

**Ends and middle.** `sin_cos` is exact at every multiple of a quarter turn
([angles](../angle-trigonometry/README.md#sine-and-cosine), AT-TP-002), so
`sine_in(0) = U - cos(0) = 0`, `sine_in(1) = U - cos(quarter) = 1.0`, `sine_out` follows by
reflection, `sine_in_out(0) = round((U - U) / 2) = 0`, `sine_in_out(1/2) = (U - cos(quarter))
/ 2 = 1/2` exactly, and `sine_in_out(1) = U - sine_in_out(0) = 1.0`. One raw unit either side
of `1/2`, where the slope is `pi/2`, `sine_in_out` moves by two raw units.

**Reflection.** `sine_out(t) == 1 - sine_in(1 - t)` and `sine_in_out(1 - t) == 1 -
sine_in_out(t)` bit for bit, by definition. Neither changes the issue's formulas: `sin_cos`
reads cosine from the sine table at the complementary position and rebuilds other quadrants
by exact sign changes, so `cos(quarter - a) == sin(a)` and `cos(half - a) == -cos(a)` bit for
bit, and `U` is even. `sine_out` is therefore exactly `sin(r 2^30)` and the upper half of
`sine_in_out` exactly `round_half_even((U - cos(r 2^31)) / 2)`; EASE-TP-045 checks both.

**Order.** `sin_cos` documents no monotonicity, so this is measured: every ease is
non-decreasing over all 1 048 577 grid points and over every `t = k / 2^16` and one raw unit
either side. Equal grid neighbours occur only at the flat ends, where the slope is under one
raw unit per `2^12` raw units of `t`: 52 pairs for `sine_in`, all with `k <= 94`; 52 for
`sine_out`, all with `k >= 1 048 481`; 68 for `sine_in_out`, with `k <= 67` or
`k >= 1 048 508`. Between raw neighbours equal values are common: the slope is at most `pi/2`
raw units per raw unit.
