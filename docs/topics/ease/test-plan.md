# Power Ease and Steps Test Plan

This document defines how Bunny proves [`bunny-ease/1`](README.md), part 1: the power eases
`power_in`, `power_out` and `power_in_out` for `n` in `2..=5`, and `steps`. It answers
flyingrobots/bunny#304.

## Scope

- `bunny_num::ease::{power_in, power_out, power_in_out, steps}` and `EASE_CONTRACT_ID`
- `bunny_num::ease::{sine_in, sine_out, sine_in_out}` (part 3, flyingrobots/bunny#306)

Out of scope: back, circ and bounce (flyingrobots/bunny#305), which extend the same module.

## Requirements

| ID | Requirement | Current Source |
| --- | --- | --- |
| EASE-REQ-001 | Every ease is `None` for `t` outside `[0, 1]` (raw outside `0..=2^32`); the power eases are `None` for `n` outside `2..=5` and `steps` for `n = 0`. | `README.md#domain` |
| EASE-REQ-002 | `power_in` is `t^n`, `power_out` is `1 - (1 - t)^n`, and `power_in_out` is `2^(n-1) t^n` below `1/2` and `1 - 2^(n-1) (1 - t)^n` from `1/2`, each its exact rational value rounded once to the nearest raw unit, ties to even: within half a raw unit. | `README.md#power-eases` |
| EASE-REQ-003 | Every power ease is exactly `0` at `t = 0` and `1.0` at `t = 1`, non-decreasing on `[0, 1]`; `power_out(n, t) = 1 - power_in(n, 1 - t)` and `power_in_out(n, 1 - t) = 1 - power_in_out(n, t)` bit for bit; `power_in_out(n, 1/2) = 1/2`, and one raw unit either side of `1/2` it moves by at most `n` raw units. | `README.md#properties` |
| EASE-REQ-004 | `steps(n, t)` is `k / n` rounded once to nearest (never a tie), for `k = floor(n t)`: `k / n` on `[k/n, (k+1)/n)` and `1.0` at `t = 1`. | `README.md#steps` |
| EASE-REQ-020 | `sine_in`, `sine_out` and `sine_in_out` are `None` for `t` outside `[0, 1]` (raw outside `0..=2^32`). | `README.md#sine-eases` |
| EASE-REQ-021 | `sine_in` is `U - cos(r 2^30)`, `sine_out` is `U - sine_in(1 - t)` (equal to `sin(r 2^30)`), and `sine_in_out` is `round_half_even((U - cos(r 2^31)) / 2)` below `1/2` and its reflection from `1/2`, reading `FixedAngle::sin_cos` at an exact angle; each is within one raw unit of `1 - cos(t pi/2)`, `sin(t pi/2)` and `(1 - cos(t pi)) / 2`. | `README.md#sine-eases` |
| EASE-REQ-022 | Each sine ease is exactly `0` at `t = 0` and `1.0` at `t = 1`, `sine_in_out(1/2) = 1/2`; `sine_out(t) = 1 - sine_in(1 - t)` and `sine_in_out(1 - t) = 1 - sine_in_out(t)` bit for bit, and both equal the direct formulas; non-decreasing over the tested points. | `README.md#sine-eases` |

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

[[requirement]]
id = "EASE-REQ-020"
summary = "The sine eases are None for t outside [0, 1]."
status = "active"

[[requirement]]
id = "EASE-REQ-021"
summary = "The sine eases read sin_cos at an exact angle, halve in-out once ties to even, and are within one raw unit."
status = "active"

[[requirement]]
id = "EASE-REQ-022"
summary = "Sine eases: exact ends and 1/2, bit-exact reflections equal to the direct formulas, monotone over the tested points."
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

The sine eases have their own oracle in `crates/bunny-num/tests/ease_sine_tests.rs`:
`oracle_sin` is the exact `sin(pi r / 2^33)` in unsigned fixed point with 124 fractional bits,
from `pi` by Machin's formula and the Taylor series of `sin`, and gives every exact sine ease
(`1 - sin((1 - t) pi/2)`, `sin(t pi/2)`, `sin(t pi/2)^2`) to about `2^-90` raw units. It shares
nothing with `sin_cos`'s table and interpolation, and it is written there rather than taken
from `tests/angle_oracle`, which has no sine. A separate Python model, posted on the pull
request, ports `sin_cos` and evaluates `decimal` sine series at 40 digits: it printed the two
pinned digests and the golden raw values, and measured the same worst cases as EASE-TP-042.

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
| EASE-TP-040 | EASE-REQ-021 | The identity string; 12 golden `t` (the ends, `1/2` and one raw unit either side, one raw unit from each end, `1/4`, `3/4`, three full-resolution values) for all three eases, each from the Python model and within one raw unit of `oracle_sin`; four halving ties, `U - cos` of 1, 3, 5 and 7, going to 0, 2, 2 and 4. | `crates/bunny-num/tests/ease_sine_tests.rs::ease_tp_040_golden_vectors_and_the_halving_tie_go_to_even` |
| EASE-TP-041 | EASE-REQ-020, EASE-REQ-022 | `sin_cos` exact at 0, a quarter and a half turn; `f(0) = 0` and `f(1) = 1.0` for each ease, `sine_in_out(1/2) = 1/2`; `None` at raw `-1`, `2^32 + 1`, `-2^32`, `2^33` and the ends of `i64`. | `crates/bunny-num/tests/ease_sine_tests.rs::ease_tp_041_the_domain_is_refused_outside_and_the_ends_and_middle_are_exact` |
| EASE-TP-042 | EASE-REQ-021 | Every `t = k / 2^20` (1 048 577 points, 3 145 731 cases) within one raw unit of `oracle_sin`, each ease's largest error and its `k` printed; the outputs' digest equals the Python model's. | `crates/bunny-num/tests/ease_sine_tests.rs::ease_tp_042_the_sweep_over_every_k_over_2_20_is_within_one_raw_unit` |
| EASE-TP-043 | EASE-REQ-021 | 100 000 seeded full-resolution `t` (seed printed) and every raw `t` within 4 096 of 0, `1/2` and 1 (16 387 points) within one raw unit of `oracle_sin`; the seeded outputs' digest equals the Python model's. | `crates/bunny-num/tests/ease_sine_tests.rs::ease_tp_043_seeded_and_boundary_points_are_within_one_raw_unit` |
| EASE-TP-044 | EASE-REQ-022 | Non-decreasing over every `t = k / 2^20` and over every `t = k / 2^16` and one raw unit either side (196 609 points); equal grid neighbours only within 128 grid steps of an end, counted and printed. | `crates/bunny-num/tests/ease_sine_tests.rs::ease_tp_044_every_sine_ease_is_monotone_non_decreasing` |
| EASE-TP-045 | EASE-REQ-022 | At 169 634 grid, seeded and near-zero points, both reflections hold bit for bit and each ease equals its direct formula read from `sin_cos` (`sin(r 2^30)` for out, the lower in-out formula on the whole interval). | `crates/bunny-num/tests/ease_sine_tests.rs::ease_tp_045_reflections_and_the_direct_formulas_agree_bit_for_bit` |
| EASE-TP-046 | EASE-REQ-020, EASE-REQ-021, EASE-REQ-022 | Each sine rule broken on purpose fails at least one case above. | [Sine mutation checks](#sine-mutation-checks) |

```toml
[[case]]
id = "EASE-TP-001"
requirements = ["EASE-REQ-002"]
evidence = "test"
test = "crates/bunny-num/tests/ease_tests.rs::ease_tp_001_hand_derived_eases_and_ties_round_to_even"
oracle = "Hand-derived raw golden values, each also checked against the independent Nat oracle and the Python Fraction model."
tier = "fast"
status = "implemented"

[[case]]
id = "EASE-TP-002"
requirements = ["EASE-REQ-001", "EASE-REQ-003", "EASE-REQ-004"]
evidence = "test"
test = "crates/bunny-num/tests/ease_tests.rs::ease_tp_002_the_domain_is_refused_outside_and_the_ends_are_exact"
oracle = "Exact ends and refusals on and one raw unit past each end of the domain."
tier = "fast"
status = "implemented"

[[case]]
id = "EASE-TP-003"
requirements = ["EASE-REQ-002"]
evidence = "test"
test = "crates/bunny-num/tests/ease_tests.rs::ease_tp_003_the_grid_matches_the_oracle_exhaustively"
oracle = "Independent Nat oracle at all 65 537 grid points for 12 eases, and the Python model's digest."
tier = "fast"
status = "implemented"

[[case]]
id = "EASE-TP-004"
requirements = ["EASE-REQ-002"]
evidence = "test"
test = "crates/bunny-num/tests/ease_tests.rs::ease_tp_004_seeded_and_boundary_points_match_the_oracle"
oracle = "Independent Nat oracle at 100 000 seeded and 16 387 boundary points, and the Python model's digest."
tier = "fast"
status = "implemented"

[[case]]
id = "EASE-TP-005"
requirements = ["EASE-REQ-003"]
evidence = "test"
test = "crates/bunny-num/tests/ease_tests.rs::ease_tp_005_every_ease_is_monotone_non_decreasing"
oracle = "Order of consecutive outputs over the grid and one raw unit either side of each point."
tier = "fast"
status = "implemented"

[[case]]
id = "EASE-TP-006"
requirements = ["EASE-REQ-003"]
evidence = "test"
test = "crates/bunny-num/tests/ease_tests.rs::ease_tp_006_reflections_symmetry_and_the_join_are_exact"
oracle = "Bit-exact reflection and symmetry identities at grid and seeded points; the value at and beside 1/2."
tier = "fast"
status = "implemented"

[[case]]
id = "EASE-TP-007"
requirements = ["EASE-REQ-004"]
evidence = "test"
test = "crates/bunny-num/tests/ease_tests.rs::ease_tp_007_steps_are_k_over_n_on_each_interval"
oracle = "Hand-derived steps and an independent u128 oracle over 100 000 seeded pairs."
tier = "fast"
status = "implemented"

[[case]]
id = "EASE-TP-008"
requirements = ["EASE-REQ-001", "EASE-REQ-002", "EASE-REQ-003", "EASE-REQ-004"]
evidence = "manual-audit"
artifact = "docs/topics/ease/test-plan.md#mutation-checks"
oracle = "Each rule broken on purpose makes at least one implemented case fail."
tier = "manual"
status = "implemented"

[[case]]
id = "EASE-TP-040"
requirements = ["EASE-REQ-021"]
evidence = "test"
test = "crates/bunny-num/tests/ease_sine_tests.rs::ease_tp_040_golden_vectors_and_the_halving_tie_go_to_even"
oracle = "Golden raw values from the Python model, each within one raw unit of the independent series oracle; halving ties."
tier = "fast"
status = "planned"

[[case]]
id = "EASE-TP-041"
requirements = ["EASE-REQ-020", "EASE-REQ-022"]
evidence = "test"
test = "crates/bunny-num/tests/ease_sine_tests.rs::ease_tp_041_the_domain_is_refused_outside_and_the_ends_and_middle_are_exact"
oracle = "Exact ends and 1/2; refusals one raw unit past each end of the domain."
tier = "fast"
status = "planned"

[[case]]
id = "EASE-TP-042"
requirements = ["EASE-REQ-021"]
evidence = "test"
test = "crates/bunny-num/tests/ease_sine_tests.rs::ease_tp_042_the_sweep_over_every_k_over_2_20_is_within_one_raw_unit"
oracle = "Independent Machin and sine-series oracle at all 1 048 577 points of k / 2^20, and the Python model's digest."
tier = "fast"
status = "planned"

[[case]]
id = "EASE-TP-043"
requirements = ["EASE-REQ-021"]
evidence = "test"
test = "crates/bunny-num/tests/ease_sine_tests.rs::ease_tp_043_seeded_and_boundary_points_are_within_one_raw_unit"
oracle = "Independent series oracle at 100 000 seeded and 16 387 boundary points, and the Python model's digest."
tier = "fast"
status = "planned"

[[case]]
id = "EASE-TP-044"
requirements = ["EASE-REQ-022"]
evidence = "test"
test = "crates/bunny-num/tests/ease_sine_tests.rs::ease_tp_044_every_sine_ease_is_monotone_non_decreasing"
oracle = "Order of consecutive outputs over the 2^20 grid and one raw unit either side of the 2^16 grid."
tier = "fast"
status = "planned"

[[case]]
id = "EASE-TP-045"
requirements = ["EASE-REQ-022"]
evidence = "test"
test = "crates/bunny-num/tests/ease_sine_tests.rs::ease_tp_045_reflections_and_the_direct_formulas_agree_bit_for_bit"
oracle = "Bit-exact reflections and agreement with the direct formulas read from sin_cos."
tier = "fast"
status = "planned"

[[case]]
id = "EASE-TP-046"
requirements = ["EASE-REQ-020", "EASE-REQ-021", "EASE-REQ-022"]
evidence = "manual-audit"
artifact = "docs/topics/ease/test-plan.md#sine-mutation-checks"
oracle = "Each sine rule broken on purpose makes at least one implemented case fail."
tier = "manual"
status = "planned"
```

## Mutation Checks

Each rule below was broken on purpose in a local copy of `crates/bunny-num/src/ease.rs`, and
`cargo test --locked -q -p bunny-num --no-fail-fast --test ease_tests` was run; the source was
restored after each. The unmutated source passed every case. "Half up" means a tie goes toward
positive infinity.

| Mutation | Rule broken | Cases that fail |
| --- | --- | --- |
| The final rounding truncates (the issue's mutation) | Ties to even | 001 (the tie `power_in(3, 3 2^21 raw) = 13.5`, which truncates to 13), 003, 004 |
| A tie goes half up | Ties to even | 001 (`power_in(3, 2^21 raw) = 0.5`), 003, 004 |
| A tie goes to the odd neighbour | Ties to even | 001, 003, 004 |
| A tie goes down | Ties to even | 001, 003 |
| In-out divides by `2^(32(n-1))`, dropping `2^(n-1)` | The in-out polynomial | 001, 003, 004, 006 |
| In-out takes its lower half below `1/2 - 2^20` raw | The join at `1/2` | 003, 004, 005, 006 |
| Out is `t^n`, not the reflection | The out polynomial | 001, 003, 004, 005, 006 |
| `t` above `1.0` accepted | `None` outside `[0, 1]` | 002 |
| `n = 1` accepted | `None` outside `2..=5` | 002 |
| `r^n` built with one factor too few | The power | 001, 002, 003, 004, 006 |
| The high 128 bits dropped before the shift | The 256-bit intermediate | 001, 002, 003, 004, 005, 006 |
| `steps` rounds `k` up | `k = floor(n t)` | 002, 007 |
| `steps(0, t)` returns `0` | `None` for `n = 0` | 002 |
| In-out takes its lower half at `t <= 1/2` | none, as proved: both halves are exactly `1/2` there ([properties](README.md#properties)) | none |
| In-out takes its lower half below `1/2 - 1` raw | none observed: at one raw unit from `1/2` the two formulas differ by about `2 n (n - 1) 2^-64`, far below a raw unit, and no tested point rounds differently | none |
