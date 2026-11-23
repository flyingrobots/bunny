# Power Ease and Steps Test Plan

This document defines how Bunny proves [`bunny-ease/1`](README.md), part 1: the power eases
`power_in`, `power_out` and `power_in_out` for `n` in `2..=5`, and `steps`. It answers
flyingrobots/bunny#304.

## Scope

- `bunny_num::ease::{power_in, power_out, power_in_out, steps}` and `EASE_CONTRACT_ID`
- `bunny_num::ease::{back_in, back_out, back_in_out, circ_in, circ_out, circ_in_out,
  bounce_in, bounce_out, bounce_in_out}` and `BACK_DEFAULT_OVERSHOOT` (part 2,
  flyingrobots/bunny#305)

Out of scope: the sine eases (flyingrobots/bunny#306), which extend the same module.

## Requirements

| ID | Requirement | Current Source |
| --- | --- | --- |
| EASE-REQ-001 | Every ease is `None` for `t` outside `[0, 1]` (raw outside `0..=2^32`); the power eases are `None` for `n` outside `2..=5` and `steps` for `n = 0`. | `README.md#domain` |
| EASE-REQ-002 | `power_in` is `t^n`, `power_out` is `1 - (1 - t)^n`, and `power_in_out` is `2^(n-1) t^n` below `1/2` and `1 - 2^(n-1) (1 - t)^n` from `1/2`, each its exact rational value rounded once to the nearest raw unit, ties to even: within half a raw unit. | `README.md#power-eases` |
| EASE-REQ-003 | Every power ease is exactly `0` at `t = 0` and `1.0` at `t = 1`, non-decreasing on `[0, 1]`; `power_out(n, t) = 1 - power_in(n, 1 - t)` and `power_in_out(n, 1 - t) = 1 - power_in_out(n, t)` bit for bit; `power_in_out(n, 1/2) = 1/2`, and one raw unit either side of `1/2` it moves by at most `n` raw units. | `README.md#properties` |
| EASE-REQ-004 | `steps(n, t)` is `k / n` rounded once to nearest (never a tie), for `k = floor(n t)`: `k / n` on `[k/n, (k+1)/n)` and `1.0` at `t = 1`. | `README.md#steps` |
| EASE-REQ-010 | Every part-2 ease is `None` for `t` outside `[0, 1]` (raw outside `0..=2^32`) and refuses nothing else; back accepts every overshoot, and its value has magnitude at most `1 + 4 \|s\| / 27`; `BACK_DEFAULT_OVERSHOOT` is raw `7 308 230 452`, the nearest raw value to `85 079 / 50 000`. | `README.md#part-2-domain` |
| EASE-REQ-011 | `back_in` is `t^2 ((s + 1) t - s)`, `back_out` is `(t - 1)^2 ((s + 1)(t - 1) + s) + 1`, and `back_in_out` is `back_in(s, 2t) / 2` below `1/2` and `1 - back_in(s, 2 - 2t) / 2` from it, each its exact rational value rounded once to nearest, ties to even; for `s > 0` the minimum of `back_in` is `-4 s^3 / (27 (s + 1)^2)` at `2s / (3 (s + 1))`, and the least raw value there is within `1/2 + s 2^-34` raw units of it. | `README.md#back` |
| EASE-REQ-012 | `circ_in` is `2^32 - isqrt(2^64 - r^2)`, `circ_out` is `isqrt(2^64 - (2^32 - r)^2)`, and `circ_in_out` is `2^31 - isqrt(2^62 - r^2)` below `1/2` and its reflection from it, with nothing rounded before the floor root: `circ_in` and the lower half of `circ_in_out` are in `[exact, exact + 1)`, and `circ_out` and the upper half in `(exact - 1, exact]`. | `README.md#circ` |
| EASE-REQ-013 | `bounce_out` is GSAP's four pieces with the breakpoints `1/2.75`, `2/2.75` and `2.5/2.75`, each owned by the piece on its right, and `bounce_in` and `bounce_in_out` are GSAP's `1 - bounce_out(1 - t)` and `(1 -/+ bounce_out(1 -/+ 2t)) / 2`, each its exact value rounded once to nearest, ties to even; the pieces meet with the value 1, and either side of a breakpoint the result is within one raw unit of 1 carried along that side's slope. | `README.md#bounce` |
| EASE-REQ-014 | Every part-2 ease is exactly `0` at `t = 0` and `1.0` at `t = 1` for every overshoot; `f_out(t) == 1 - f_in(1 - t)` and `f_in_out(1 - t) == 1 - f_in_out(t)` bit for bit and `f_in_out(1/2) = 1/2`; circ is non-decreasing, and back and bounce are not monotone. | `README.md#part-2-properties` |

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
id = "EASE-REQ-010"
summary = "Part-2 eases are None for t outside [0, 1] only; every overshoot fits; the default overshoot is raw 7308230452."
status = "active"

[[requirement]]
id = "EASE-REQ-011"
summary = "back_in, back_out and back_in_out are GSAP's back, exact and rounded once, ties to even, with the closed-form minimum."
status = "active"

[[requirement]]
id = "EASE-REQ-012"
summary = "circ is the floor of the exact 128-bit root subtracted exactly, within one raw unit on the stated side."
status = "active"

[[requirement]]
id = "EASE-REQ-013"
summary = "bounce is GSAP's four pieces at 1/2.75, 2/2.75 and 2.5/2.75, each rounded once, continuous at each breakpoint."
status = "active"

[[requirement]]
id = "EASE-REQ-014"
summary = "Exact ends, bit-exact reflections and in-out symmetry, exact 1/2; circ monotone, back and bounce not."
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

### Part 2 Oracles

`crates/bunny-num/tests/ease_back_circ_bounce_tests.rs` writes its own references from GSAP's
definitions in flyingrobots/bunny#305 and shares no code with the kernel. Back is one signed
numerator over `2^96`, built from the unexpanded forms `t^2 ((s + 1) t - s)` and
`(t - 1)^2 ((s + 1)(t - 1) + s) + 1` (in-out from `(1 -/+ back_out(1 -/+ 2t)) / 2`), multiplied
out in base-`2^32` digits and rounded on those digits. The kernel uses the expanded inner
factor, a reflection, `u128` halves and a shift and mask. Bounce is one `u128` fraction per
piece, `121/16 (t - a/b)^2 + c/64`, chosen by `275 t` against `100`, `200` and `250`, and rounded
by a division. Circ is the issue's rule with the test's own bit-by-bit integer square root,
written from different radicands (`2 U r - r^2` for "out", `U^2 - 4 r^2` halved after the root
for in-out). The kernel uses core's `u128::isqrt`. `ease_tp_026` proves the circ bound by squaring,
so it needs no square root at all.

The Python model, posted on the pull request, evaluates back and bounce as `fractions.Fraction`
values rounded by Python's `round` (ties to even) and circ by the rule with `math.isqrt`. It
printed the three pinned FNV-1a digests (grid, seeded and overshoot pairs) and the golden raw
values. It also measured each circ result against a 50-digit `decimal` square root, giving the
maxima recorded in the [chapter](README.md#circ).

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
| EASE-TP-020 | EASE-REQ-010, EASE-REQ-011, EASE-REQ-012, EASE-REQ-013 | `BACK_DEFAULT_OVERSHOOT` is raw `7 308 230 452`, within half a raw unit of `85 079 2^32 / 50 000`; hand-derived raw values, each also equal to the oracle and the Python model: `bounce_out(0.5) = 49/64`, `circ_in(0.5) = 575 416 510` (its root's squares checked), `circ_in` at raw `4 294 920 955` and `2^32 - 1`, `circ_out(1 raw)`, `circ_in_out(1/2 - 1 raw)`, the ties `bounce_in_out(1/2 - 2^17 raw) = 1/2 - 60.5 -> 1/2 - 60` and `back_in(s0, 1/2) = -376 657 894.5 -> even`, and three full-resolution values. | `crates/bunny-num/tests/ease_back_circ_bounce_tests.rs::ease_tp_020_hand_derived_eases_and_the_default_overshoot` |
| EASE-TP-021 | EASE-REQ-010, EASE-REQ-014 | For all nine eases and eight overshoots (the default, 0, 1, -1, -1.0, 10.0 and the ends of `i64`): `f(0) = 0`, `f(1) = 1.0`, and `None` at raw `-1`, `2^32 + 1`, `-2^32` and the ends of `i64`. | `crates/bunny-num/tests/ease_back_circ_bounce_tests.rs::ease_tp_021_the_domain_is_refused_outside_and_the_ends_are_exact` |
| EASE-TP-022 | EASE-REQ-011, EASE-REQ-012, EASE-REQ-013 | All nine eases (back at the default overshoot) at every `t = k / 2^16` (65 537 points, 589 833 cases) equal the oracle, and the outputs' digest equals the Python model's. | `crates/bunny-num/tests/ease_back_circ_bounce_tests.rs::ease_tp_022_the_grid_matches_the_oracle_exhaustively` |
| EASE-TP-023 | EASE-REQ-011, EASE-REQ-012, EASE-REQ-013 | Every raw `t` within `2^16` of 0 and of 1, and within 4 096 of `1/2` and of each bounce breakpoint, its reflection and its in-out images (237 583 points counted with overlaps, 2 138 247 cases), for all nine eases, equals the oracle. | `crates/bunny-num/tests/ease_back_circ_bounce_tests.rs::ease_tp_023_every_raw_t_near_the_ends_and_joins_matches_the_oracle` |
| EASE-TP-024 | EASE-REQ-011, EASE-REQ-012, EASE-REQ-013 | Raw `4 294 920 955` and `2^20` seeded full-resolution `t` (seed printed; 1 048 577 points, 9 437 193 cases), for all nine eases, equal the oracle; the outputs' digest equals the Python model's. | `crates/bunny-num/tests/ease_back_circ_bounce_tests.rs::ease_tp_024_seeded_full_resolution_t_match_the_oracle` |
| EASE-TP-025 | EASE-REQ-010, EASE-REQ-011 | `2^16` seeded `(s, t)` (seed printed; `s` of every magnitude from `2^16` to `2^63` raw, either sign) and the ends of `i64`, `+-1`, `+-1.0` and `0` as `s` at seven `t`: the three back eases equal the oracle and have magnitude at most `1 + 4 \|s\| / 27`; the pairs' digest equals the Python model's. | `crates/bunny-num/tests/ease_back_circ_bounce_tests.rs::ease_tp_025_back_matches_the_oracle_for_every_overshoot` |
| EASE-TP-026 | EASE-REQ-012 | At every grid point, every raw `t` within `2^16` of each end (so the last 65 536 raw units before 1, where the slope is unbounded) and every seeded point including raw `4 294 920 955`, squaring proves each circ result is the floor root of its exact radicand: `circ_in` in `[exact, exact + 1)`, `circ_out` in `(exact - 1, exact]`, and each half of `circ_in_out` on its side. | `crates/bunny-num/tests/ease_back_circ_bounce_tests.rs::ease_tp_026_circ_is_the_floor_root_within_one_raw_unit` |
| EASE-TP-027 | EASE-REQ-014 | At every grid point and `2^16` seeded points, for circ, bounce and back at seven overshoots: `f_out(t) == 1 - f_in(1 - t)` and `f_in_out(1 - t) == 1 - f_in_out(t)` exactly, and `f_in_out(1/2) = 1/2`. | `crates/bunny-num/tests/ease_back_circ_bounce_tests.rs::ease_tp_027_reflections_and_the_middle_are_exact` |
| EASE-TP-028 | EASE-REQ-014 | Over every grid point, one raw unit either side, and every raw `t` within `2^16` of each end (sorted, 327 675 points), each circ ease is non-decreasing; `back_in(s0, 1/4) < back_in(s0, 0)` and `bounce_out` falls from `4/11` to `1/2` show back and bounce are not monotone. | `crates/bunny-num/tests/ease_back_circ_bounce_tests.rs::ease_tp_028_circ_is_monotone_and_back_and_bounce_are_not` |
| EASE-TP-029 | EASE-REQ-013 | The oracle's adjacent pieces are exactly 1 at `4/11`, `8/11` and `10/11` (and at 1); one raw unit either side of each breakpoint, `bounce_out` is within one raw unit of 1 carried along that side's slope; over every raw `t` near each breakpoint and end, one raw unit moves each bounce ease by at most 6 raw units, and each grid step by at most `5.5 2^16 + 1`. | `crates/bunny-num/tests/ease_back_circ_bounce_tests.rs::ease_tp_029_bounce_is_continuous_at_each_breakpoint` |
| EASE-TP-030 | EASE-REQ-011, EASE-REQ-014 | For `s` of the default, 1, 3 and 10: the least `back_in` near `t* = 2s / (3(s + 1))`, the greatest `back_out` near `1 - t*` and the least `back_in_out` near `t*/2` are within `1/2 + 2^-16` raw units of `-k`, `1 + k` and `-k/2` for `k = 4 s^3 / (27 (s + 1)^2)`, checked in exact `i128`; `back_out(s, 1) = 1.0` exactly at five overshoots. | `crates/bunny-num/tests/ease_back_circ_bounce_tests.rs::ease_tp_030_back_overshoots_by_the_closed_form` |
| EASE-TP-031 | EASE-REQ-010, EASE-REQ-011, EASE-REQ-012, EASE-REQ-013, EASE-REQ-014 | Each part-2 rule broken on purpose fails at least one case above, the issue's wrong bounce breakpoint among them. | [Part 2 mutation checks](#part-2-mutation-checks) |

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
id = "EASE-TP-020"
requirements = ["EASE-REQ-010", "EASE-REQ-011", "EASE-REQ-012", "EASE-REQ-013"]
evidence = "test"
test = "crates/bunny-num/tests/ease_back_circ_bounce_tests.rs::ease_tp_020_hand_derived_eases_and_the_default_overshoot"
oracle = "Hand-derived raw golden values, each also checked against the independent oracle and the Python model."
tier = "fast"
status = "implemented"

[[case]]
id = "EASE-TP-021"
requirements = ["EASE-REQ-010", "EASE-REQ-014"]
evidence = "test"
test = "crates/bunny-num/tests/ease_back_circ_bounce_tests.rs::ease_tp_021_the_domain_is_refused_outside_and_the_ends_are_exact"
oracle = "Exact ends and refusals on and one raw unit past each end of the domain."
tier = "fast"
status = "implemented"

[[case]]
id = "EASE-TP-022"
requirements = ["EASE-REQ-011", "EASE-REQ-012", "EASE-REQ-013"]
evidence = "test"
test = "crates/bunny-num/tests/ease_back_circ_bounce_tests.rs::ease_tp_022_the_grid_matches_the_oracle_exhaustively"
oracle = "Independent oracle at all 65 537 grid points for 9 eases, and the Python model's digest."
tier = "fast"
status = "implemented"

[[case]]
id = "EASE-TP-023"
requirements = ["EASE-REQ-011", "EASE-REQ-012", "EASE-REQ-013"]
evidence = "test"
test = "crates/bunny-num/tests/ease_back_circ_bounce_tests.rs::ease_tp_023_every_raw_t_near_the_ends_and_joins_matches_the_oracle"
oracle = "Independent oracle at every raw t near each end, join and breakpoint."
tier = "fast"
status = "implemented"

[[case]]
id = "EASE-TP-024"
requirements = ["EASE-REQ-011", "EASE-REQ-012", "EASE-REQ-013"]
evidence = "test"
test = "crates/bunny-num/tests/ease_back_circ_bounce_tests.rs::ease_tp_024_seeded_full_resolution_t_match_the_oracle"
oracle = "Independent oracle at 1 048 577 seeded points, and the Python model's digest."
tier = "fast"
status = "implemented"

[[case]]
id = "EASE-TP-025"
requirements = ["EASE-REQ-010", "EASE-REQ-011"]
evidence = "test"
test = "crates/bunny-num/tests/ease_back_circ_bounce_tests.rs::ease_tp_025_back_matches_the_oracle_for_every_overshoot"
oracle = "Independent wide-integer oracle over 65 536 seeded and 63 constructed overshoot pairs, and the Python model's digest."
tier = "fast"
status = "implemented"

[[case]]
id = "EASE-TP-026"
requirements = ["EASE-REQ-012"]
evidence = "test"
test = "crates/bunny-num/tests/ease_back_circ_bounce_tests.rs::ease_tp_026_circ_is_the_floor_root_within_one_raw_unit"
oracle = "The floor property `y^2 <= X < (y + 1)^2` checked by squaring, with no square root; the Python model's 50-digit decimal measurement is in the chapter."
tier = "fast"
status = "implemented"

[[case]]
id = "EASE-TP-027"
requirements = ["EASE-REQ-014"]
evidence = "test"
test = "crates/bunny-num/tests/ease_back_circ_bounce_tests.rs::ease_tp_027_reflections_and_the_middle_are_exact"
oracle = "Bit-exact reflection and symmetry identities at grid and seeded points; the value at 1/2."
tier = "fast"
status = "implemented"

[[case]]
id = "EASE-TP-028"
requirements = ["EASE-REQ-014"]
evidence = "test"
test = "crates/bunny-num/tests/ease_back_circ_bounce_tests.rs::ease_tp_028_circ_is_monotone_and_back_and_bounce_are_not"
oracle = "Order of consecutive outputs; explicit counterexamples for the non-monotone families."
tier = "fast"
status = "implemented"

[[case]]
id = "EASE-TP-029"
requirements = ["EASE-REQ-013"]
evidence = "test"
test = "crates/bunny-num/tests/ease_back_circ_bounce_tests.rs::ease_tp_029_bounce_is_continuous_at_each_breakpoint"
oracle = "Exact piece values at the breakpoints and exact slope bounds on consecutive outputs."
tier = "fast"
status = "implemented"

[[case]]
id = "EASE-TP-030"
requirements = ["EASE-REQ-011", "EASE-REQ-014"]
evidence = "test"
test = "crates/bunny-num/tests/ease_back_circ_bounce_tests.rs::ease_tp_030_back_overshoots_by_the_closed_form"
oracle = "The closed-form extremes in exact integer arithmetic."
tier = "fast"
status = "implemented"

[[case]]
id = "EASE-TP-031"
requirements = ["EASE-REQ-010", "EASE-REQ-011", "EASE-REQ-012", "EASE-REQ-013", "EASE-REQ-014"]
evidence = "manual-audit"
artifact = "docs/topics/ease/test-plan.md#part-2-mutation-checks"
oracle = "Each part-2 rule broken on purpose makes at least one implemented case fail."
tier = "manual"
status = "implemented"
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

## Part 2 Mutation Checks

Each rule below was broken on purpose in a local copy of
`crates/bunny-num/src/ease/back_circ_bounce.rs`, or of `ease.rs` for the shared domain check, and
`cargo test --locked -q -p bunny-num --no-fail-fast --test ease_back_circ_bounce_tests` was run.
The source was restored after each. The unmutated source passed every case.

| Mutation | Rule broken | Cases that fail |
| --- | --- | --- |
| The last bounce breakpoint `2.5/2.75` read as `2.6/2.75` (the issue's mutation) | The breakpoints | 020, 022, 023, 024, 027, 029 (just above `10/11`, `bounce_out` is 1.25 raw units from the right-hand tangent, beyond the one-unit bound) |
| The last bounce level `63/64` read as `62/64` | The bounce coefficients | 020, 021, 022, 023, 024, 029 |
| Bounce in-out's upper half not reflected | The in-out construction | 021, 022, 023, 024, 027 |
| Bounce in-out halves after rounding the full value | One rounding | 020, 022, 023, 024, 027 |
| Back's final rounding truncates | Ties to even, one rounding | 022, 023, 024, 025, 030 |
| Back drops the high 128 bits of its 256-bit value | The wide intermediate | 021, 022, 023, 025, 027, 028, 030 |
| Back drops the sign of a negative value | Rounding a negative value | 020, 022, 023, 024, 025, 028, 030 |
| Back in-out scales the overshoot by Penner's `1.525` | GSAP's in-out, same overshoot | 020, 021, 022, 023, 024, 025, 027, 030 |
| The default overshoot truncated to raw `7 308 230 451` | Nearest raw value of `1.70158` | 020 |
| Circ rounds `1 - t^2` to Q32.32 before the root (the rule the issue corrected) | Nothing rounded before the root | 020, 023, 024, 026 |
| Circ rounds the root to nearest instead of taking the floor | The floor root | 020, 022, 023, 024, 026 |
| Circ in-out halves after the subtraction, `(1 - isqrt(2^64 - 4 r^2)) / 2` | Halving inside the root | 020, 022, 023, 024, 026, 027 |
| Circ out evaluated at `t`, not reflected at `1 - t` | The out construction | 020, 021, 022, 023, 024, 026, 027, 028 |
| `t` one raw unit above `1.0` accepted (shared domain check) | `None` outside `[0, 1]` | 021 |
| Each bounce breakpoint owned by the piece on its left (`<` read as `<=`) | none, as proved: no raw `t` lies on a breakpoint, because `11` does not divide `k 2^32` ([bounce](README.md#bounce)) | none |
