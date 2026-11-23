# Exact Unsigned Ellipse Classification Test Plan

Verification plan for [`bunny-u32-ellipse-classify/1`](README.md).

## Requirements

| ID | Requirement | Current Source |
| --- | --- | --- |
| UEC-REQ-001 | CPU reference computes exact unsigned 64-bit ellipse trichotomy over `0..=46,340`, enforces the explicit zero-semiaxis branch before the raw polynomial, and returns `OutOfDomain` / `[0, 1]` above `46,340`. | `README.md#normative-contract` |
| UEC-REQ-002 | Checked-in ordered fixture contains the 24 core golden vectors (including raw-polynomial zero-semiaxis traps and max-domain carry cases), scaled ties, and seeded tail matching CPU reference. | `README.md#ownership--scope` |
| UEC-REQ-003 | Portable 2-limb `u32` WGSL helper executed via `wgpu` matches CPU reference canonical 8-byte LE output bit-for-bit, with deterministic manifest digests, `NotRun` on empty slices, error-scope `Fail` on invalid WGSL, and honest `Unavailable` reporting. | `README.md#conformance-evidence` |

```toml
[[requirement]]
id = "UEC-REQ-001"
summary = "CPU reference computes exact unsigned 64-bit ellipse trichotomy over 0..=46,340 with explicit zero-semiaxis branch and OutOfDomain refusal."
status = "active"

[[requirement]]
id = "UEC-REQ-002"
summary = "Checked-in 1,444-vector fixture contains the 24 core golden vectors (including zero-semiaxis traps), scaled ties, and seeded random tuples matching CPU reference."
status = "active"

[[requirement]]
id = "UEC-REQ-003"
summary = "Portable 2-limb u32 WGSL helper executed via wgpu matches CPU canonical output bytes bit-for-bit with deterministic manifest and honest failure/empty/unavailable reporting."
status = "active"
```

## Test Cases

| ID | Requirements | Oracle | Test |
| --- | --- | --- | --- |
| UEC-TP-001 | UEC-REQ-001 | Pythagorean exact boundary ties, off-by-1 neighbors, non-circular ellipses, cardinal endpoints, and scaled max-domain ties match `u128` oracle, `Result`, `[class, status]`, and 8-byte LE bytes. | `crates/bunny-num/tests/u32_ellipse_classify_tests.rs::uec_tp_001_golden_pythagorean_ties_and_neighbors_are_exact` |
| UEC-TP-002 | UEC-REQ-001 | Explicit `semi_x == 0 \|\| semi_y == 0` branch precedes raw polynomial so `(0,0)` is `OnBoundary` and `(0,1,0,5)`, `(1,0,5,0)`, etc. are `Outside`. | `crates/bunny-num/tests/u32_ellipse_classify_tests.rs::uec_tp_002_zero_semiaxis_branch_precedes_raw_polynomial` |
| UEC-TP-003 | UEC-REQ-001 | Any operand `> 46,340` returns `Err(EllipseDomainError)`, `[0, 1]`, and `[0,0,0,0,1,0,0,0]`. | `crates/bunny-num/tests/u32_ellipse_classify_tests.rs::uec_tp_003_out_of_domain_returns_canonical_error` |
| UEC-TP-004 | UEC-REQ-001 | 50,000 reproducible seeded random tuples in `0..=46,340` match independent `u128` oracle. | `crates/bunny-num/tests/u32_ellipse_classify_tests.rs::uec_tp_004_seeded_property_sweep_matches_u128_oracle` |
| UEC-TP-005 | UEC-REQ-002 | Checked-in 1,444-vector fixture matches CPU reference and pins the zero-semiaxis traps `(0,0,0,5)`, `(0,1,0,5)`, and `(1,0,5,0)`. | `crates/bunny-gpu/tests/u32_ellipse_classify_gpu.rs::uec_gpu_001_fixture_matches_cpu_reference_and_zero_axis_traps` |
| UEC-TP-006 | UEC-REQ-003 | Real `wgpu` compute execution of assembled 2-limb WGSL matches CPU canonical output digest bit-for-bit (or reports `Unavailable` honestly). | `crates/bunny-gpu/tests/u32_ellipse_classify_gpu.rs::uec_gpu_002_wgsl_2limb_compute_matches_cpu_bit_for_bit` |
| UEC-TP-007 | UEC-REQ-003 | Deterministic `ImplementationManifest` (`CRATE_VERSION`), `Outcome::NotRun` on empty slices, and error-scoped `Outcome::Fail` on invalid WGSL. | `crates/bunny-gpu/tests/u32_ellipse_classify_gpu.rs::uec_gpu_003_manifest_empty_and_invalid_wgsl_behave_honestly` |

```toml
[[case]]
id = "UEC-TP-001"
requirements = ["UEC-REQ-001"]
evidence = "test"
test = "crates/bunny-num/tests/u32_ellipse_classify_tests.rs::uec_tp_001_golden_pythagorean_ties_and_neighbors_are_exact"
oracle = "Golden Pythagorean, non-circular, cardinal, and scaled max-domain ties match independent u128 oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "UEC-TP-002"
requirements = ["UEC-REQ-001"]
evidence = "test"
test = "crates/bunny-num/tests/u32_ellipse_classify_tests.rs::uec_tp_002_zero_semiaxis_branch_precedes_raw_polynomial"
oracle = "Degenerate zero-semiaxis branch precedes raw polynomial so non-origin points classify as Outside."
tier = "fast"
status = "implemented"

[[case]]
id = "UEC-TP-003"
requirements = ["UEC-REQ-001"]
evidence = "test"
test = "crates/bunny-num/tests/u32_ellipse_classify_tests.rs::uec_tp_003_out_of_domain_returns_canonical_error"
oracle = "Any operand above 46,340 returns Err(EllipseDomainError), [0, 1], and canonical LE error bytes."
tier = "fast"
status = "implemented"

[[case]]
id = "UEC-TP-004"
requirements = ["UEC-REQ-001"]
evidence = "test"
test = "crates/bunny-num/tests/u32_ellipse_classify_tests.rs::uec_tp_004_seeded_property_sweep_matches_u128_oracle"
oracle = "50,000 seeded random tuples in 0..=46,340 match independent u128 oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "UEC-TP-005"
requirements = ["UEC-REQ-002"]
evidence = "test"
test = "crates/bunny-gpu/tests/u32_ellipse_classify_gpu.rs::uec_gpu_001_fixture_matches_cpu_reference_and_zero_axis_traps"
oracle = "Checked-in 1,444-vector fixture matches CPU reference and pins the zero-semiaxis traps."
tier = "fast"
status = "implemented"

[[case]]
id = "UEC-TP-006"
requirements = ["UEC-REQ-003"]
evidence = "test"
test = "crates/bunny-gpu/tests/u32_ellipse_classify_gpu.rs::uec_gpu_002_wgsl_2limb_compute_matches_cpu_bit_for_bit"
oracle = "Executed 2-limb u32 WGSL compute shader output bytes match CPU canonical bytes SHA-256 bit-for-bit."
tier = "fast"
status = "implemented"

[[case]]
id = "UEC-TP-007"
requirements = ["UEC-REQ-003"]
evidence = "test"
test = "crates/bunny-gpu/tests/u32_ellipse_classify_gpu.rs::uec_gpu_003_manifest_empty_and_invalid_wgsl_behave_honestly"
oracle = "Manifest is deterministic with dynamic CRATE_VERSION, empty slices return NotRun, invalid WGSL returns Fail without panicking."
tier = "fast"
status = "implemented"
```
