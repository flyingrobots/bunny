# Fixed-Point Rounding

This chapter says where `bunny-num` rounds a Q32.32 result and by which rule. The law is
[`NUMERIC_CONSTITUTION.md`](../../NUMERIC_CONSTITUTION.md#rounding-policy): ties to even.
The [test plan](test-plan.md) pins each site with exact values. It answers
flyingrobots/bunny#287.

## Rounding Sites

| Site | Exact value rounded | Rule |
| --- | --- | --- |
| `FixedQ32_32::checked_mul`, `Mul` | `lhs * rhs / 2^32`, from the exact `i128` product | ties to even |
| `FixedQ32_32::checked_div`, `Div` | `dividend * 2^32 / divisor`, from the exact `i128` quotient and remainder | ties to even |
| `FixedDivisor::checked_div` | the same quotient as `checked_div`, by a prepared reciprocal | ties to even, the same bits |
| `from_f32`, `try_from_f32` | the float's exact value times `2^32` | ties to even |
| `to_f32` | the raw value to a 24-bit significand | ties to even |

Each site rounds the magnitude and then applies the sign. Because being even does not
depend on the sign, this is ties to even on the signed value: `-2.5` becomes `-2` and
`-1.5` becomes `-2`, and `round(-x) == -round(x)` always.

Overflow is decided after rounding. A product whose exact value is `2^63 - 1/2` ties to
`2^63`, which does not fit: `checked_mul` returns `None` and `Mul` saturates to `i64::MAX`.
Negated, the tie goes to `-2^63`, which is `i64::MIN` and fits.

## Not Ties To Even

- `FixedQ32_32::sqrt` returns the floor of the exact square root.
- `FixedAngle::from_degrees` and `from_radians` round half up on the magnitude, and
  `sin_cos` rounds a 62-bit intermediate (see
  [angles and trigonometry](../angle-trigonometry/README.md)).
