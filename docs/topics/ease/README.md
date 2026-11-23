# Power Eases and Steps (`bunny-ease/1`)

This chapter states how Bunny evaluates easing curves in Q32.32. Part 1 covers the power eases
(GSAP's power1 to power4, `t^2` to `t^5`) and `steps`. Each power ease is a polynomial with
rational coefficients, so its exact value is a rational with a power-of-two denominator: Bunny
computes it exactly and rounds it once, to nearest with ties to even. The identity string is
`bunny-ease/1` (`bunny_num::EASE_CONTRACT_ID`); the functions are in `bunny_num::ease`. The
[test plan](test-plan.md) holds the oracles, golden vectors and mutation checks. It answers
flyingrobots/bunny#304.

| Function | Exact value rounded | Refused (`None`) |
| --- | --- | --- |
| `power_in(n, t)` | `t^n` | `t` outside `[0, 1]`; `n` outside `2..=5` |
| `power_out(n, t)` | `1 - (1 - t)^n` | the same |
| `power_in_out(n, t)` | `2^(n-1) t^n` for `t < 1/2`, `1 - 2^(n-1) (1 - t)^n` from `1/2` | the same |
| `steps(n, t)` | `k / n` for `k = floor(n t)` | `t` outside `[0, 1]`; `n = 0` |

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
