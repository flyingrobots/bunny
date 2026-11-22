# Unsigned 32-Bit Ties-to-Even Division Test Plan

Verification plan for [`bunny-u32-div-round-even/1`](README.md).

## Requirements

| ID | Requirement | Current Source |
| --- | --- | --- |
| UDRE-REQ-001 | CPU reference computes exact ties-to-even `n / d` across `0..=u32::MAX` without `2 * r` overflow and returns `DivisionByZero` / `[0, 1]` on `d == 0`. | `README.md#normative-contract` |
| UDRE-REQ-002 | Checked-in ordered fixture contains the 15 core golden vectors, boundary grid, large ties, and seeded tail matching CPU reference and exact `u128` oracle. | `README.md#ownership--scope` |
| UDRE-REQ-003 | Portable WGSL helper executed via `wgpu` matches CPU reference canonical 8-byte LE output bit-for-bit, or honestly reports `Outcome::Unavailable` when no adapter exists. | `README.md#conformance-evidence` |

```toml
[[requirement]]
id = "UDRE-REQ-001"
summary = "CPU reference computes exact ties-to-even u32 division over 0..=u32::MAX without overflow and returns [0, 1] on d == 0."
status = "active"

[[requirement]]
id = "UDRE-REQ-002"
summary = "Ordered vector fixture contains the 15 core golden vectors plus boundary, large-tie, and seeded cases matching CPU reference."
status = "active"

[[requirement]]
id = "UDRE-REQ-003"
summary = "WGSL compute shader executed via wgpu matches CPU canonical output bytes bit-for-bit, with deterministic manifest digests and honest Unavailable reporting."
status = "active"
```

## Test Cases

| ID | Requirements | Oracle | Test |
| --- | --- | --- | --- |
| UDRE-TP-001 | UDRE-REQ-001 | All 15 core golden vectors checked against `u128` oracle, `Result`, `[value, status]`, and 8-byte LE wire format. | `crates/bunny-num/tests/u32_div_round_even_tests.rs::udre_tp_001_golden_vectors_match_contract` |
| UDRE-TP-002 | UDRE-REQ-001 | Division by zero across representative `n` values returns `Err(DivisionByZero)` and `[0, 1]`. | `crates/bunny-num/tests/u32_div_round_even_tests.rs::udre_tp_002_division_by_zero_returns_canonical_error` |
| UDRE-TP-003 | UDRE-REQ-001 | 19x19 boundary grid and large even-denominator ties (where `2 * r` overflows `u32`) match `u128` oracle. | `crates/bunny-num/tests/u32_div_round_even_tests.rs::udre_tp_003_boundary_grid_and_large_ties_match_u128_oracle` |
| UDRE-TP-004 | UDRE-REQ-001 | 50,000 seeded random pairs match `u128` oracle. | `crates/bunny-num/tests/u32_div_round_even_tests.rs::udre_tp_004_seeded_property_sweep_matches_u128_oracle` |
| UDRE-TP-005 | UDRE-REQ-002 | Checked-in 1,720-vector JSON fixture matches CPU reference and pins the `3,000,000,000 / 4,000,000,000` counterexample. | `crates/bunny-gpu/tests/u32_div_round_even_gpu.rs::udre_gpu_001_fixture_matches_cpu_reference_and_15_core_vectors` |
| UDRE-TP-006 | UDRE-REQ-003 | Real `wgpu` compute execution of assembled WGSL matches CPU canonical output digest bit-for-bit (or reports `Unavailable` honestly). | `crates/bunny-gpu/tests/u32_div_round_even_gpu.rs::udre_gpu_002_wgsl_compute_matches_cpu_bit_for_bit` |
| UDRE-TP-007 | UDRE-REQ-003 | Content-addressed manifest and artifact SHA-256 digests are deterministic 64-hex values. | `crates/bunny-gpu/tests/u32_div_round_even_gpu.rs::udre_gpu_003_manifest_and_receipt_identities_are_deterministic` |

```toml
[[case]]
id = "UDRE-TP-001"
requirements = ["UDRE-REQ-001"]
evidence = "test"
test = "crates/bunny-num/tests/u32_div_round_even_tests.rs::udre_tp_001_golden_vectors_match_contract"
oracle = "15 core golden vectors checked against independent u128 exact rounding oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "UDRE-TP-002"
requirements = ["UDRE-REQ-001"]
evidence = "test"
test = "crates/bunny-num/tests/u32_div_round_even_tests.rs::udre_tp_002_division_by_zero_returns_canonical_error"
oracle = "Zero divisor always produces Err(DivisionByZero), [0, 1], and 8-byte LE [0,0,0,0,1,0,0,0]."
tier = "fast"
status = "implemented"

[[case]]
id = "UDRE-TP-003"
requirements = ["UDRE-REQ-001"]
evidence = "test"
test = "crates/bunny-num/tests/u32_div_round_even_tests.rs::udre_tp_003_boundary_grid_and_large_ties_match_u128_oracle"
oracle = "19x19 boundary grid and large ties where 2*r wraps u32 match u128 oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "UDRE-TP-004"
requirements = ["UDRE-REQ-001"]
evidence = "test"
test = "crates/bunny-num/tests/u32_div_round_even_tests.rs::udre_tp_004_seeded_property_sweep_matches_u128_oracle"
oracle = "50,000 reproducible seeded random pairs match u128 oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "UDRE-TP-005"
requirements = ["UDRE-REQ-002"]
evidence = "test"
test = "crates/bunny-gpu/tests/u32_div_round_even_gpu.rs::udre_gpu_001_fixture_matches_cpu_reference_and_15_core_vectors"
oracle = "Checked-in 1,720-vector fixture matches CPU reference and exact rational expectations."
tier = "fast"
status = "implemented"

[[case]]
id = "UDRE-TP-006"
requirements = ["UDRE-REQ-003"]
evidence = "test"
test = "crates/bunny-gpu/tests/u32_div_round_even_gpu.rs::udre_gpu_002_wgsl_compute_matches_cpu_bit_for_bit"
oracle = "Executed WGSL compute shader output bytes match CPU canonical bytes SHA-256 bit-for-bit."
tier = "fast"
status = "implemented"

[[case]]
id = "UDRE-TP-007"
requirements = ["UDRE-REQ-003"]
evidence = "test"
test = "crates/bunny-gpu/tests/u32_div_round_even_gpu.rs::udre_gpu_003_manifest_and_receipt_identities_are_deterministic"
oracle = "ImplementationManifest digests are deterministic 64-hex SHA-256 values."
tier = "fast"
status = "implemented"
```
