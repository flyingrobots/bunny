# Fixed-Point Rounding Test Plan

This document defines how Bunny proves the [rounding sites](README.md) round ties to even.

## Scope

- `checked_mul` and `Mul`; `checked_div`, `Div` and `FixedDivisor::checked_div`
- `from_f32` and `try_from_f32` (ingress) and `to_f32` (egress)

Out of scope: `sqrt` (a floor), `FixedAngle` (the angle chapter), and the trace kernels'
floor rounding (flyingrobots/bunny#295).

## Requirements

| ID | Requirement | Current Source |
| --- | --- | --- |
| FPR-REQ-001 | Multiplication rounds the exact product to Q32.32 with ties to even, the same for both signs, before the overflow policy. | `README.md#rounding-sites` |
| FPR-REQ-002 | Division and the prepared divisor round the exact quotient with ties to even, the same for both signs. | `README.md#rounding-sites` |
| FPR-REQ-003 | Float ingress and egress round with ties to even. | `README.md#rounding-sites` |

```toml
[[requirement]]
id = "FPR-REQ-001"
summary = "Multiplication rounds the exact product with ties to even for both signs, before the overflow policy."
status = "active"

[[requirement]]
id = "FPR-REQ-002"
summary = "Division and the prepared divisor round the exact quotient with ties to even for both signs."
status = "active"

[[requirement]]
id = "FPR-REQ-003"
summary = "Float ingress and egress round with ties to even."
status = "active"
```

## Fixtures

Every vector is inline in `crates/bunny-num/tests/tie_rounding_tests.rs`, with the exact
rational, its two neighbours and the chosen one in a comment. They were derived by hand and
checked with exact rational arithmetic in a separate Python model before being committed.
Each multiplication vector runs through `checked_mul` and `Mul`; each division vector
through `checked_div`, `Div` and `FixedDivisor`; each float through both signs.

## Test Cases

| ID | Requirements | Oracle | Test |
| --- | --- | --- | --- |
| FPR-TP-001 | FPR-REQ-001 | Products just below half (one raw unit under) keep the truncation, both signs. | `tie_rounding_tests.rs::fpr_tp_001_products_below_half_keep_the_truncation` |
| FPR-TP-002 | FPR-REQ-001 | Products just above half move away from the truncation, both signs. | `tie_rounding_tests.rs::fpr_tp_002_products_above_half_leave_the_truncation` |
| FPR-TP-003 | FPR-REQ-001 | Exact-half products whose truncation is even keep it, both signs. | `tie_rounding_tests.rs::fpr_tp_003_product_ties_keep_an_even_truncation` |
| FPR-TP-004 | FPR-REQ-001 | Exact-half products whose truncation is odd move to the even neighbour, both signs. | `tie_rounding_tests.rs::fpr_tp_004_product_ties_move_an_odd_truncation_to_even` |
| FPR-TP-005 | FPR-REQ-001 | Ties with products near 2^95, both neighbour parities and signs; `2^63 - 1/2` rounds out of range and its negation to `i64::MIN`. | `tie_rounding_tests.rs::fpr_tp_005_product_ties_at_the_widest_intermediates` |
| FPR-TP-006 | FPR-REQ-002 | Quotients below half (including 1/256 under) keep the truncation, both signs. | `tie_rounding_tests.rs::fpr_tp_006_quotients_below_half_keep_the_truncation` |
| FPR-TP-007 | FPR-REQ-002 | Quotients above half (including 1/256 over) move away from the truncation, both signs. | `tie_rounding_tests.rs::fpr_tp_007_quotients_above_half_leave_the_truncation` |
| FPR-TP-008 | FPR-REQ-002 | Exact-half quotients whose truncation is even keep it, for every sign pair. | `tie_rounding_tests.rs::fpr_tp_008_quotient_ties_keep_an_even_truncation` |
| FPR-TP-009 | FPR-REQ-002 | Exact-half quotients whose truncation is odd move to the even neighbour, for every sign pair. | `tie_rounding_tests.rs::fpr_tp_009_quotient_ties_move_an_odd_truncation_to_even` |
| FPR-TP-010 | FPR-REQ-002 | Ties with numerators near 2^95 and with the divisor `i64::MIN`; an odd divisor just above a half. | `tie_rounding_tests.rs::fpr_tp_010_quotient_ties_at_the_widest_operands` |
| FPR-TP-011 | FPR-REQ-003 | Floats of `n * 2^-34`: below, above and exact halves of both parities, both signs. | `tie_rounding_tests.rs::fpr_tp_011_float_ingress_rounds_ties_to_even` |
| FPR-TP-012 | FPR-REQ-003 | Raw values losing one or two bits to the significand: ties of both parities, below and above half, both signs. | `tie_rounding_tests.rs::fpr_tp_012_float_egress_rounds_ties_to_even` |
| FPR-TP-013 | FPR-REQ-001, FPR-REQ-002, FPR-REQ-003 | Each wrong rounding rule at each site, put in on purpose, makes a case above fail. | [Mutation checks](#mutation-checks) |

The test file is `crates/bunny-num/tests/tie_rounding_tests.rs`.

```toml
[[case]]
id = "FPR-TP-001"
requirements = ["FPR-REQ-001"]
evidence = "test"
test = "crates/bunny-num/tests/tie_rounding_tests.rs::fpr_tp_001_products_below_half_keep_the_truncation"
oracle = "Hand-derived exact raw products just below half, both signs."
tier = "fast"
status = "implemented"

[[case]]
id = "FPR-TP-002"
requirements = ["FPR-REQ-001"]
evidence = "test"
test = "crates/bunny-num/tests/tie_rounding_tests.rs::fpr_tp_002_products_above_half_leave_the_truncation"
oracle = "Hand-derived exact raw products just above half, both signs."
tier = "fast"
status = "implemented"

[[case]]
id = "FPR-TP-003"
requirements = ["FPR-REQ-001"]
evidence = "test"
test = "crates/bunny-num/tests/tie_rounding_tests.rs::fpr_tp_003_product_ties_keep_an_even_truncation"
oracle = "Hand-derived exact-half products with an even truncation, both signs."
tier = "fast"
status = "implemented"

[[case]]
id = "FPR-TP-004"
requirements = ["FPR-REQ-001"]
evidence = "test"
test = "crates/bunny-num/tests/tie_rounding_tests.rs::fpr_tp_004_product_ties_move_an_odd_truncation_to_even"
oracle = "Hand-derived exact-half products with an odd truncation, both signs."
tier = "fast"
status = "implemented"

[[case]]
id = "FPR-TP-005"
requirements = ["FPR-REQ-001"]
evidence = "test"
test = "crates/bunny-num/tests/tie_rounding_tests.rs::fpr_tp_005_product_ties_at_the_widest_intermediates"
oracle = "Hand-derived ties with products near 2^95, including one that rounds out of range."
tier = "fast"
status = "implemented"

[[case]]
id = "FPR-TP-006"
requirements = ["FPR-REQ-002"]
evidence = "test"
test = "crates/bunny-num/tests/tie_rounding_tests.rs::fpr_tp_006_quotients_below_half_keep_the_truncation"
oracle = "Hand-derived exact raw quotients below half, both signs, three division paths."
tier = "fast"
status = "implemented"

[[case]]
id = "FPR-TP-007"
requirements = ["FPR-REQ-002"]
evidence = "test"
test = "crates/bunny-num/tests/tie_rounding_tests.rs::fpr_tp_007_quotients_above_half_leave_the_truncation"
oracle = "Hand-derived exact raw quotients above half, both signs, three division paths."
tier = "fast"
status = "implemented"

[[case]]
id = "FPR-TP-008"
requirements = ["FPR-REQ-002"]
evidence = "test"
test = "crates/bunny-num/tests/tie_rounding_tests.rs::fpr_tp_008_quotient_ties_keep_an_even_truncation"
oracle = "Hand-derived exact-half quotients with an even truncation, every sign pair."
tier = "fast"
status = "implemented"

[[case]]
id = "FPR-TP-009"
requirements = ["FPR-REQ-002"]
evidence = "test"
test = "crates/bunny-num/tests/tie_rounding_tests.rs::fpr_tp_009_quotient_ties_move_an_odd_truncation_to_even"
oracle = "Hand-derived exact-half quotients with an odd truncation, every sign pair."
tier = "fast"
status = "implemented"

[[case]]
id = "FPR-TP-010"
requirements = ["FPR-REQ-002"]
evidence = "test"
test = "crates/bunny-num/tests/tie_rounding_tests.rs::fpr_tp_010_quotient_ties_at_the_widest_operands"
oracle = "Hand-derived ties with numerators near 2^95 and the divisor i64::MIN."
tier = "fast"
status = "implemented"

[[case]]
id = "FPR-TP-011"
requirements = ["FPR-REQ-003"]
evidence = "test"
test = "crates/bunny-num/tests/tie_rounding_tests.rs::fpr_tp_011_float_ingress_rounds_ties_to_even"
oracle = "Hand-derived raw values of exact floats n * 2^-34."
tier = "fast"
status = "implemented"

[[case]]
id = "FPR-TP-012"
requirements = ["FPR-REQ-003"]
evidence = "test"
test = "crates/bunny-num/tests/tie_rounding_tests.rs::fpr_tp_012_float_egress_rounds_ties_to_even"
oracle = "Hand-derived f32 bit patterns for raw values that lose one or two bits."
tier = "fast"
status = "implemented"

[[case]]
id = "FPR-TP-013"
requirements = ["FPR-REQ-001", "FPR-REQ-002", "FPR-REQ-003"]
evidence = "manual-audit"
artifact = "docs/topics/fixed-point-rounding/test-plan.md#mutation-checks"
oracle = "Each wrong rounding rule at each site, put in on purpose, makes at least one implemented case fail."
tier = "manual"
status = "implemented"
```

## The Constitution's Required Tests

[`NUMERIC_CONSTITUTION.md`](../../NUMERIC_CONSTITUTION.md#rounding-policy) lists eight
required tests. "Exact-half to even" is read as a tie whose truncation is already even (it is
kept); "to odd" as a tie whose truncation is odd (it moves to the even neighbour).

| Required test | Multiplication | Division | Floats |
| --- | --- | --- | --- |
| positive / negative less-than-half | FPR-TP-001 | FPR-TP-006 | FPR-TP-011, FPR-TP-012 |
| positive / negative greater-than-half | FPR-TP-002 | FPR-TP-007 | FPR-TP-011, FPR-TP-012 |
| positive / negative exact-half to even | FPR-TP-003 | FPR-TP-008 | FPR-TP-011, FPR-TP-012 |
| positive / negative exact-half to odd | FPR-TP-004 | FPR-TP-009 | FPR-TP-011, FPR-TP-012 |

## Mutation Checks

Each rule was put into a container copy of the source, the sources were touched, and
`timeout 120 cargo test --locked --no-fail-fast -p bunny-num` was run. "Toward zero" means
ties toward zero on the magnitude; "half up" means ties toward positive infinity. The
unmutated source passed every test; every mutant failed at least one FPR case.

| Site (mutated call) | Ties away | Ties toward zero | Half up | Truncation |
| --- | --- | --- | --- | --- |
| `checked_mul` and `Mul` | 003, 005 | 004, 005 | 003, 004, 005 | 002, 004, 005 |
| `checked_div` (and `Div`) | 008, 010 | 009, 010 | 008, 009, 010 | 007, 009, 010 |
| `FixedDivisor::checked_div` | 008, 010 | 009, 010 | 008, 009, 010 | 007, 009, 010 |
| `from_f32` ingress shift | 011 | — | — | 011 |
| `to_f32` egress shift | 012 | — | — | 012 |

Before these cases, no `bunny-num` test failed for any multiplication mutant or any float
mutant, and the division mutants failed only the agreement tests between `checked_div` and
`FixedDivisor` (PERF-TP-034, -035, -037, and -036 for truncation), which compare two
implementations rather than a value.

## Gaps

- FPR-TP-001 and FPR-TP-006 fail none of these mutants; they catch rounding that always goes
  up.
- `FixedAngle`'s conversions round half up on the magnitude, as documented in its chapter;
  this plan does not cover them.
