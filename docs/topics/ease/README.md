# Power Eases and Steps (`bunny-ease/1`)

This chapter states how Bunny evaluates easing curves in Q32.32. Part 1 covers the power eases
(GSAP's power1 to power4, `t^2` to `t^5`) and `steps`. Each power ease is a polynomial with
rational coefficients, so its exact value is a rational with a power-of-two denominator: Bunny
computes it exactly and rounds it once, to nearest with ties to even. The identity string is
`bunny-ease/1` (`bunny_num::EASE_CONTRACT_ID`); the functions are in `bunny_num::ease`. The
[test plan](test-plan.md) holds the oracles, golden vectors and mutation checks. It answers
flyingrobots/bunny#304. Part 2, [Back, Circ and Bounce](#back-circ-and-bounce), adds GSAP's back,
circ and bounce eases from flyingrobots/bunny#305. Part 3, the [sine eases](#sine-eases), answers
flyingrobots/bunny#306.

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

## Back, Circ and Bounce

Part 2 adds GSAP's back, circ and bounce eases, from flyingrobots/bunny#305, in the same module.
Back and bounce are polynomials with rational coefficients, each rounded once to nearest with
ties to even. Circ needs a square root. Its radicand is formed exactly and its root is a floor,
so circ has a one-sided bound of one raw unit, not half a unit.

| Function | Exact value | Result | Refused (`None`) |
| --- | --- | --- | --- |
| `back_in(s, t)` | `t^2 ((s + 1) t - s)` | rounded once | `t` outside `[0, 1]` |
| `back_out(s, t)` | `(t - 1)^2 ((s + 1)(t - 1) + s) + 1` | rounded once | the same |
| `back_in_out(s, t)` | `back_in(s, 2t) / 2` for `t < 1/2`, `1 - back_in(s, 2 - 2t) / 2` from `1/2` | rounded once | the same |
| `circ_in(t)` | `1 - sqrt(1 - t^2)` | `2^32 - isqrt(2^64 - r^2)` | the same |
| `circ_out(t)` | `sqrt(1 - (1 - t)^2)` | `isqrt(2^64 - (2^32 - r)^2)` | the same |
| `circ_in_out(t)` | `circ_in(2t) / 2` for `t < 1/2`, `1 - circ_in(2 - 2t) / 2` from `1/2` | `2^31 - isqrt(2^62 - r^2)`, reflected | the same |
| `bounce_out(t)` | GSAP's four pieces, [below](#bounce) | rounded once | the same |
| `bounce_in(t)` | `1 - bounce_out(1 - t)` | rounded once | the same |
| `bounce_in_out(t)` | `(1 - bounce_out(1 - 2t)) / 2` for `t < 1/2`, `(1 + bounce_out(2t - 1)) / 2` from `1/2` | rounded once | the same |

### Part 2 Domain

`t` has the same domain as in part 1: raw `0` to `2^32`, and anything else is `None`. The back
eases accept every `FixedQ32_32` overshoot `s`, negative ones included, and refuse nothing else.
Every value fits. `t^3` is in `[0, 1]` and `t^2 (1 - t)` is in `[0, 4/27]`, so
`back_in = t^3 - s t^2 (1 - t)` has magnitude at most `1 + 4 |s| / 27`, and so do `back_out` and
`back_in_out`. For `|s| < 2^31` that is below `3.2 10^8`. The raw value is then below `2^61`.

`BACK_DEFAULT_OVERSHOOT` is GSAP's `1.70158`, which is `85 079 / 50 000`. The contract rounds it
once to nearest, giving raw `7 308 230 452`. `85 079 2^32 / 50 000` is `7 308 230 451.53`, so the
overshoot actually used is exactly `7 308 230 452 / 2^32`, `0.47` raw units above GSAP's
decimal. The curve is then exact for that rational.

### Back

On raw values, with `r = t_raw`, `S = s_raw` and `U = 2^32`:

```text
back_in(s, t)      = round_half_even(r^2 (S (r - U) + U r) / 2^96)
back_out(s, t)     = U - back_in(s, 1 - t)
back_in_out(s, t)  = round_half_even((2r)^2 (S (2r - U) + U 2r) / 2^97)    r < 2^31
                   = U - back_in_out(s, 1 - t)                             r >= 2^31
```

`r^2 (S (r - U) + U r) / 2^96` is `t^2 ((s + 1) t - s)` in raw units, exactly. "Out" and the upper
half of "in-out" use the same reflection as [part 1](#power-eases), and it is exact for the same
reason: `U` is even, so `round_half_even(U - x) = U - round_half_even(x)`. A negative value
rounds as the negation of its magnitude, so a tie goes to even on either side of zero. Every
result is within half a raw unit of the exact value. In-out is GSAP's construction with the
same overshoot. It is not Penner's, which scales `s` by `1.525`.

Back is not monotone for `s > 0`, and it leaves `[0, 1]`. Write `k = 4 s^3 / (27 (s + 1)^2)`.
`back_in` falls to its minimum `-k` at `t* = 2s / (3 (s + 1))`, then rises to exactly `1.0` at
`t = 1`; it is at most `t^3 <= 1` elsewhere. `back_out` rises to its maximum `1 + k` at `1 - t*`,
and is never below `0`. `back_in_out` reaches `-k/2` at `t*/2` and `1 + k/2` at `1 - t*/2`. At the
default overshoot `k` is about `0.1`, so `back_in` dips about 10 % below zero. A raw `t` within
half a raw unit of `t*` gives a value within `1/2 + s 2^-34` raw units of the closed form: one
rounding, plus the curve's rise over that half unit, where its second derivative is `2s`.

| Quantity | Bound | Held in |
| --- | --- | --- |
| `S (r - U)` | `2^95` | `i128` |
| inner factor `S (r - U) + U r` | `2^95 + 2^64` | `i128`, then its magnitude in `u128` |
| magnitude times `r` | below `2^128` | `u128` |
| times `r` again | below `2^160` | the high and low `u128` halves of a 256-bit value |
| the rounded result | below `2^61` | `i64` |

### Circ

```text
circ_in(t)      = U - isqrt(2^64 - r^2)
circ_out(t)     = isqrt(2^64 - (U - r)^2)                   = U - circ_in(1 - t)
circ_in_out(t)  = 2^31 - isqrt(2^62 - r^2)                  r < 2^31
                = 2^31 + isqrt(2^62 - (U - r)^2)            r >= 2^31, = U - circ_in_out(1 - t)
```

`isqrt(n)` is the floor of the square root, `max { y : y^2 <= n }`, computed exactly by
`u128::isqrt`. Nothing is rounded before the root. `1 - t^2` in raw units squared is the exact
integer `2^64 - r^2`, at most `2^64`, so it fits a `u128`. In-out halves inside the root:
`sqrt(2^64 - 4 r^2) / 2 = sqrt(2^62 - r^2)`, so it needs no division after it. Each subtraction
is of integers, and exact.

The rule is a floor followed by an exact subtraction. That is not one rounding to nearest of
the exact value, so the bound comes from the rule. `isqrt(n)` is in `(sqrt(n) - 1, sqrt(n)]`.
So `circ_in` and the lower half of `circ_in_out` are at least the exact value and less than one
raw unit above it. `circ_out` and the upper half of `circ_in_out` are at most the exact value
and less than one raw unit below it. Each is exact where the radicand is a perfect square. The
bound was proved before it was measured. A Python model then compared the results with a
50-digit `decimal` square root over every tested point: the grid, every raw `t` within `2^16`
of each end, and the `2^20 + 1` seeded points. The measured `kernel - exact` was:

| Function | Least | Greatest |
| --- | --- | --- |
| `circ_in` | `0` | `0.999 999 999 884` |
| `circ_out` | `-0.999 999 999 884` | `0` |
| `circ_in_out` | `-0.999 999 999 767` | `0.999 999 999 767` |

Near `t = 1` the slope of `circ_in` is unbounded. At raw `U - k` the radicand is `2 k U - k^2`,
so the last raw unit before `1.0` gives `U - isqrt(2^33 - 1) = U - 92 681`, a step of 92 681 raw
units. That is why the radicand is never rounded. At raw `4 294 920 955` (`k = 46 341`) the
radicand is `398 064 011 439 591`, with root `19 951 541.58`. The floor gives
`circ_in = 4 275 015 755`, `0.58` raw units above the exact value. Rounding `1 - t^2` to Q32.32
first would err by `54.58` raw units there. Circ is non-decreasing (not strictly), because
`isqrt` is.

### Bounce

GSAP's bounce.out uses `n = 7.5625 = 121/16` and the breakpoints `1/2.75 = 4/11`,
`2/2.75 = 8/11` and `2.5/2.75 = 10/11`:

```text
bounce_out(t) = 121/16 t^2                          0      <= t < 4/11
              = 121/16 (t - 6/11)^2  + 3/4           4/11   <= t < 8/11      (1.5/2.75 = 6/11)
              = 121/16 (t - 9/11)^2  + 15/16         8/11   <= t < 10/11     (2.25/2.75 = 9/11)
              = 121/16 (t - 21/22)^2 + 63/64         10/11  <= t <= 1        (2.625/2.75 = 21/22)
```

Each breakpoint belongs to the piece on its right. No raw `t` sits on one, because `11` does
not divide `k 2^32`. The pieces meet there with the common value exactly `1`. With
`t = n/22`, a piece is `((n - p)^2 + q) / 64` for centre `p/22` and level `q/64`, and
`(8 - 0)^2 + 0 = (8 - 12)^2 + 48 = (16 - 18)^2 + 60 = (20 - 21)^2 + 63 = 64`. So bounce is
continuous but not smooth. Its slope jumps at each breakpoint: from `5.5` to `-2.75` at `4/11`,
from `2.75` to `-1.375` at `8/11`, and from `1.375` to `-0.6875` at `10/11`. Bounce is not
monotone. It is at most `1` everywhere, and exactly `1` at each breakpoint and at `t = 1`.

`121/16 (t - p/22)^2` is `(22 t - p)^2 / 64`, so on raw values each piece is

```text
bounce_out(t)      = round_half_even((22 r - p U)^2 / 2^38) + q 2^26
bounce_in(t)       = U - bounce_out(1 - t)
bounce_in_out(t)   = 2^31 - q 2^25 - round_half_even((22 u - p U)^2 / 2^39)      r < 2^31, u = U - 2r
                   = U - bounce_in_out(1 - t)                                    r >= 2^31
```

The piece is chosen by `22 r` against `8 U`, `16 U` and `20 U`. `|22 r - p U| < 2^37`, so the
square is exact in a `u128`, and each level is a whole number of raw units, so each result is
the exact value rounded once. `(22 r - p U)^2 / 2^38` is never a tie, because a square has an
even number of factors of two. Halved, it can be one: `bounce_in_out(1/2 - 2^17 raw)` is
exactly `1/2 - 60.5` raw, rounded to `1/2 - 60`. Either side of a breakpoint the result is
within one raw unit of the common value `1`, carried along that side's slope. The steepest
slope is `5.5`, so one raw unit of `t` moves any bounce ease by at most 6 raw units.

### Part 2 Properties

- **Ends.** Every part-2 ease is exactly `0` at `t = 0` and exactly `1.0` at `t = 1`, for every
  overshoot. `back_out(s, 1) = 1.0` because `back_in(s, 0) = 0` exactly.
- **Reflection.** `f_out(t) == 1 - f_in(1 - t)` and `f_in_out(1 - t) == 1 - f_in_out(t)` hold
  bit for bit for back (at every overshoot), circ and bounce. `f_in_out(1/2) = 1/2` exactly.
- **Order.** Circ is non-decreasing. Back for `s > 0` and bounce are not monotone, and no
  order property is claimed for them.
- **The circ join.** `circ_in_out` is continuous at `1/2`, but its slope there is unbounded:
  `circ_in_out(1/2 - 1 raw) = 1/2 - 65 535 raw`.

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
