# Power Ease and Steps Test Plan

This document defines how Bunny proves [`bunny-ease/1`](README.md), part 1: the power eases
`power_in`, `power_out` and `power_in_out` for `n` in `2..=5`, and `steps`. It answers
flyingrobots/bunny#304.

## Scope

- `bunny_num::ease::{power_in, power_out, power_in_out, steps}` and `EASE_CONTRACT_ID`

Out of scope: back, circ and bounce (flyingrobots/bunny#305) and the sine eases
(flyingrobots/bunny#306), which extend the same module.

## Requirements

| ID | Requirement | Current Source |
| --- | --- | --- |
| EASE-REQ-001 | Every ease is `None` for `t` outside `[0, 1]` (raw outside `0..=2^32`); the power eases are `None` for `n` outside `2..=5` and `steps` for `n = 0`. | `README.md#domain` |
| EASE-REQ-002 | `power_in` is `t^n`, `power_out` is `1 - (1 - t)^n`, and `power_in_out` is `2^(n-1) t^n` below `1/2` and `1 - 2^(n-1) (1 - t)^n` from `1/2`, each its exact rational value rounded once to the nearest raw unit, ties to even: within half a raw unit. | `README.md#power-eases` |
| EASE-REQ-003 | Every power ease is exactly `0` at `t = 0` and `1.0` at `t = 1`, non-decreasing on `[0, 1]`; `power_out(n, t) = 1 - power_in(n, 1 - t)` and `power_in_out(n, 1 - t) = 1 - power_in_out(n, t)` bit for bit; `power_in_out(n, 1/2) = 1/2`, and one raw unit either side of `1/2` it moves by at most `n` raw units. | `README.md#properties` |
| EASE-REQ-004 | `steps(n, t)` is `k / n` rounded once to nearest (never a tie), for `k = floor(n t)`: `k / n` on `[k/n, (k+1)/n)` and `1.0` at `t = 1`. | `README.md#steps` |

```toml
[[requirement]]
id = "EASE-REQ-001"
summary = "Every ease is None for t outside [0, 1]; power eases None for n outside 2..=5, steps None for n = 0."
status = "active"

[[requirement]]
id = "EASE-REQ-002"
summary = "power_in, power_out and power_in_out are the issue's polynomials, exact and rounded once, ties to even."
status = "active"

[[requirement]]
id = "EASE-REQ-003"
summary = "Exact ends, monotone, bit-exact out reflection and in-out symmetry, exact 1/2, a join step of at most n raw units."
status = "active"

[[requirement]]
id = "EASE-REQ-004"
summary = "steps(n, t) is round(floor(n t) / n), 1.0 at t = 1."
status = "active"
```

## Oracles

`crates/bunny-num/tests/ease_tests.rs` writes its own exact reference and shares no code with
the kernel. `oracle_ease` evaluates each ease from the issue's definition as one numerator over
`2^(32(n-1))` in a base-`2^32` natural number (`Nat`, with its own multiplication and
subtraction), including the subtractions of "out" and the upper half of "in-out" that the
kernel replaces by reflection, and rounds by `floor((2v + 2^s) / 2^(s+1))` with an exact tie
moved back to even. `oracle_steps` computes `k` and `round(k / n)` in `u128`. The kernel holds
`r^n` in a `u128` pair, divides "in-out" by `2^(31(n-1))`, and rounds by a shift and a mask.

A third model, a Python `fractions.Fraction` script posted on the pull request, evaluates the
same definitions with Python's `round` (ties to even). It printed the two pinned FNV-1a
digests (every grid output and every seeded output, function by function) and the golden raw
values. Every test is a `wasm_bindgen_test`, so the WebAssembly job runs the golden vectors
and digests too.

## Test Cases

| ID | Requirements | Oracle | Test |
| --- | --- | --- | --- |
| EASE-TP-001 | EASE-REQ-002 | The identity string; hand-derived golden raw values, each also equal to `oracle_ease` and the Python model: `power_in(2, 0.5) = 0.25`, `power_out(2, 0.25) = 0.4375`, `power_in_out(3, 0.5) = 0.5` from both branches; the ties `power_in(3, 3 2^21 raw) = 13.5 -> 14` and `2^21 raw -> 0`, `power_out(3, 3 2^21 raw)` and `power_in_out(2, 3 2^15 raw) = 4.5 -> 4`; one raw unit from each end and from `1/2`; three full-resolution values. | `crates/bunny-num/tests/ease_tests.rs::ease_tp_001_hand_derived_eases_and_ties_round_to_even` |
| EASE-TP-002 | EASE-REQ-001, EASE-REQ-003, EASE-REQ-004 | `f(0) = 0` and `f(1) = 1.0` for all 12 power eases and `steps` at five `n`; `None` at raw `-1`, `2^32 + 1`, `-2^32` and the ends of `i64`, for `n` of 0, 1, 6 and `u32::MAX`, and for `steps(0, t)`. | `crates/bunny-num/tests/ease_tests.rs::ease_tp_002_the_domain_is_refused_outside_and_the_ends_are_exact` |
| EASE-TP-003 | EASE-REQ-002 | All 12 power eases at every `t = k / 2^16` (65 537 points, 786 444 cases) equal `oracle_ease`, and the outputs' digest equals the Python model's. | `crates/bunny-num/tests/ease_tests.rs::ease_tp_003_the_grid_matches_the_oracle_exhaustively` |
| EASE-TP-004 | EASE-REQ-002 | 100 000 seeded full-resolution `t` (seed printed) and every raw `t` within 4 096 raw units of 0, `1/2` and 1 (16 387 points), for all 12 eases, equal `oracle_ease`; the seeded outputs' digest equals the Python model's. | `crates/bunny-num/tests/ease_tests.rs::ease_tp_004_seeded_and_boundary_points_match_the_oracle` |
| EASE-TP-005 | EASE-REQ-003 | Over every grid point and one raw unit either side (196 609 points), each ease's outputs are non-decreasing; `power_in(5, 0) == power_in(5, 1 raw)` and `power_out(2, 1 - 1 raw) == 1.0` show it is not strict. | `crates/bunny-num/tests/ease_tests.rs::ease_tp_005_every_ease_is_monotone_non_decreasing` |
| EASE-TP-006 | EASE-REQ-003 | At every grid and seeded point, `power_out(n, t) == 1 - power_in(n, 1 - t)` and `power_in_out(n, 1 - t) == 1 - power_in_out(n, t)` exactly; `power_in_out(n, 1/2) = 1/2`; at `1/2` plus and minus one raw unit the value moves by 1 to `n` raw units. | `crates/bunny-num/tests/ease_tests.rs::ease_tp_006_reflections_symmetry_and_the_join_are_exact` |
| EASE-TP-007 | EASE-REQ-004 | Hand-derived steps one raw unit either side of `k / n` for `n` of 1, 2, 3, 4 and `u32::MAX`, each equal to `oracle_steps`; 100 000 seeded `(n, t)` (seed printed, `n` of every magnitude) equal `oracle_steps` and are non-decreasing one raw unit on. | `crates/bunny-num/tests/ease_tests.rs::ease_tp_007_steps_are_k_over_n_on_each_interval` |
| EASE-TP-008 | EASE-REQ-001, EASE-REQ-002, EASE-REQ-003, EASE-REQ-004 | Each rule broken on purpose fails at least one case above. | [Mutation checks](#mutation-checks) |

```toml
[[case]]
id = "EASE-TP-001"
requirements = ["EASE-REQ-002"]
evidence = "test"
test = "crates/bunny-num/tests/ease_tests.rs::ease_tp_001_hand_derived_eases_and_ties_round_to_even"
oracle = "Hand-derived raw golden values, each also checked against the independent Nat oracle and the Python Fraction model."
tier = "fast"
status = "planned"

[[case]]
id = "EASE-TP-002"
requirements = ["EASE-REQ-001", "EASE-REQ-003", "EASE-REQ-004"]
evidence = "test"
test = "crates/bunny-num/tests/ease_tests.rs::ease_tp_002_the_domain_is_refused_outside_and_the_ends_are_exact"
oracle = "Exact ends and refusals on and one raw unit past each end of the domain."
tier = "fast"
status = "planned"

[[case]]
id = "EASE-TP-003"
requirements = ["EASE-REQ-002"]
evidence = "test"
test = "crates/bunny-num/tests/ease_tests.rs::ease_tp_003_the_grid_matches_the_oracle_exhaustively"
oracle = "Independent Nat oracle at all 65 537 grid points for 12 eases, and the Python model's digest."
tier = "fast"
status = "planned"

[[case]]
id = "EASE-TP-004"
requirements = ["EASE-REQ-002"]
evidence = "test"
test = "crates/bunny-num/tests/ease_tests.rs::ease_tp_004_seeded_and_boundary_points_match_the_oracle"
oracle = "Independent Nat oracle at 100 000 seeded and 16 387 boundary points, and the Python model's digest."
tier = "fast"
status = "planned"

[[case]]
id = "EASE-TP-005"
requirements = ["EASE-REQ-003"]
evidence = "test"
test = "crates/bunny-num/tests/ease_tests.rs::ease_tp_005_every_ease_is_monotone_non_decreasing"
oracle = "Order of consecutive outputs over the grid and one raw unit either side of each point."
tier = "fast"
status = "planned"

[[case]]
id = "EASE-TP-006"
requirements = ["EASE-REQ-003"]
evidence = "test"
test = "crates/bunny-num/tests/ease_tests.rs::ease_tp_006_reflections_symmetry_and_the_join_are_exact"
oracle = "Bit-exact reflection and symmetry identities at grid and seeded points; the value at and beside 1/2."
tier = "fast"
status = "planned"

[[case]]
id = "EASE-TP-007"
requirements = ["EASE-REQ-004"]
evidence = "test"
test = "crates/bunny-num/tests/ease_tests.rs::ease_tp_007_steps_are_k_over_n_on_each_interval"
oracle = "Hand-derived steps and an independent u128 oracle over 100 000 seeded pairs."
tier = "fast"
status = "planned"

[[case]]
id = "EASE-TP-008"
requirements = ["EASE-REQ-001", "EASE-REQ-002", "EASE-REQ-003", "EASE-REQ-004"]
evidence = "manual-audit"
artifact = "docs/topics/ease/test-plan.md#mutation-checks"
oracle = "Each rule broken on purpose makes at least one implemented case fail."
tier = "manual"
status = "planned"
```

## Mutation Checks

Planned: each rule in [the chapter](README.md) broken on purpose in a local copy of
`crates/bunny-num/src/ease.rs`, including the issue's listed mutation (the final rounding
truncates instead of rounding ties to even, which must fail a tie vector), with the cases that
fail recorded here.
