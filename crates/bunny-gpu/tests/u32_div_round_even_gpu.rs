//! `wgpu` CPU-vs-GPU equivalence tests for `bunny-u32-div-round-even/1` (`flyingrobots/bunny#428`).

use bunny_gpu::{
    load_u32_div_round_even_fixture, run_u32_div_round_even_cpu, run_u32_div_round_even_gpu,
    run_u32_div_round_even_gpu_with_shader, u32_div_round_even_manifest, AdapterPolicy, DivVector,
    ExecutionClass, Outcome,
};
use bunny_num::{div_round_even_u32_words, CRATE_VERSION, U32_DIV_ROUND_EVEN_CONTRACT_ID};

const EXPECTED_CORE_15: &[(u32, u32, u32, u32)] = &[
    (0, 1, 0, 0),
    (u32::MAX, 1, u32::MAX, 0),
    (u32::MAX, u32::MAX, 1, 0),
    (1, 4, 0, 0),
    (3, 4, 1, 0),
    (1, 2, 0, 0),
    (3, 2, 2, 0),
    (5, 2, 2, 0),
    (7, 2, 4, 0),
    (u32::MAX, 2, 2_147_483_648, 0),
    (3_000_000_000, 4_000_000_000, 1, 0),
    (2_147_483_647, u32::MAX, 0, 0),
    (2_147_483_648, u32::MAX, 1, 0),
    (0, 0, 0, 1),
    (u32::MAX, 0, 0, 1),
];

#[test]
fn udre_gpu_001_fixture_matches_cpu_reference_and_15_core_vectors() {
    let fixture = load_u32_div_round_even_fixture().expect("fixture JSON should parse cleanly");
    assert_eq!(fixture.contract_id, U32_DIV_ROUND_EVEN_CONTRACT_ID);
    assert_eq!(fixture.core_count, 15);
    assert!(fixture.vectors.len() >= 1_500, "fixture should include core + boundary + seeded tail");

    // Verify all 15 core golden vectors at their pinned indices 0..15
    for (idx, &(n, d, exp_val, exp_st)) in EXPECTED_CORE_15.iter().enumerate() {
        let vec = fixture.vectors[idx];
        assert_eq!(
            (vec.numerator, vec.denominator, vec.expected_value, vec.expected_status),
            (n, d, exp_val, exp_st),
            "core vector[{idx}] mismatch",
        );
    }

    for vec in &fixture.vectors {
        assert_eq!(
            div_round_even_u32_words(vec.numerator, vec.denominator),
            [vec.expected_value, vec.expected_status],
        );
    }

    let cpu_receipt = run_u32_div_round_even_cpu(&fixture.vectors);
    assert_eq!(cpu_receipt.outcome, Outcome::Pass);
    assert_eq!(cpu_receipt.execution_class, ExecutionClass::Cpu);
    assert_eq!(
        cpu_receipt.actual_results_sha256.as_deref(),
        Some(cpu_receipt.expected_results_sha256.as_str()),
    );
    assert!(cpu_receipt.detail.is_none());

    // CPU failure receipt must populate diagnostic detail on mismatch
    let bad_vec =
        [DivVector { numerator: 3, denominator: 4, expected_value: 99, expected_status: 0 }];
    let bad_cpu = run_u32_div_round_even_cpu(&bad_vec);
    assert_eq!(bad_cpu.outcome, Outcome::Fail);
    assert!(bad_cpu.detail.is_some());
}

#[test]
fn udre_gpu_002_wgsl_compute_matches_cpu_bit_for_bit() {
    let fixture = load_u32_div_round_even_fixture().expect("fixture JSON should parse cleanly");
    let receipt = run_u32_div_round_even_gpu(&fixture.vectors, AdapterPolicy::HardwarePreferred);

    match receipt.outcome {
        Outcome::Pass => {
            assert_eq!(
                receipt.actual_results_sha256.as_deref(),
                Some(receipt.expected_results_sha256.as_str()),
                "GPU output digest must equal CPU expected digest bit-for-bit",
            );
            assert_ne!(receipt.execution_class, ExecutionClass::Cpu);
            assert!(receipt.environment.adapter_name.is_some());
        }
        Outcome::Unavailable => {
            // Honest unavailability on headless runners without GPU/software adapter:
            // never misreport Unavailable as Pass, and never populate actual_results_sha256.
            assert!(receipt.actual_results_sha256.is_none());
            assert!(receipt.detail.is_some());
            // macOS Apple Silicon hosts always have Metal; fail loudly if Metal unexpectedly vanished.
            #[cfg(target_os = "macos")]
            panic!("expected Metal GPU adapter on macOS, got Unavailable: {:?}", receipt.detail);
        }
        other => {
            panic!(
                "GPU equivalence failed ({other:?}) on {:?} ({:?}): {:?}",
                receipt.execution_class, receipt.environment, receipt.detail
            );
        }
    }
}

#[test]
fn udre_gpu_003_manifest_and_receipt_identities_are_deterministic() {
    let m1 = u32_div_round_even_manifest();
    let m2 = u32_div_round_even_manifest();
    assert_eq!(m1, m2);
    let d1 = m1.digest_sha256().expect("manifest digest");
    let d2 = m2.digest_sha256().expect("manifest digest");
    assert_eq!(d1, d2);
    assert_eq!(d1.len(), 64);
    assert_eq!(m1.helper_wgsl_sha256.len(), 64);
    assert_eq!(m1.assembled_compute_wgsl_sha256.len(), 64);
    assert_eq!(m1.vectors_fixture_sha256.len(), 64);
    assert_eq!(m1.bunny_num_version, CRATE_VERSION);
}

#[test]
fn udre_gpu_004_empty_vectors_return_not_run() {
    let cpu = run_u32_div_round_even_cpu(&[]);
    assert_eq!(cpu.outcome, Outcome::NotRun);
    assert_eq!(cpu.vector_count, 0);
    assert!(cpu.actual_results_sha256.is_none());
    assert!(cpu.detail.is_some());

    let gpu = run_u32_div_round_even_gpu(&[], AdapterPolicy::HardwarePreferred);
    assert_eq!(gpu.outcome, Outcome::NotRun);
    assert_eq!(gpu.vector_count, 0);
    assert_eq!(gpu.environment.backend, "not-run");
    assert!(gpu.actual_results_sha256.is_none());
    assert!(gpu.detail.is_some());
}

#[test]
fn udre_gpu_005_invalid_wgsl_returns_fail_without_panicking() {
    let sample =
        [DivVector { numerator: 3, denominator: 4, expected_value: 1, expected_status: 0 }];
    let receipt = run_u32_div_round_even_gpu_with_shader(
        &sample,
        "this is not valid wgsl !!!",
        AdapterPolicy::HardwarePreferred,
    );
    match receipt.outcome {
        Outcome::Fail => {
            assert!(receipt.actual_results_sha256.is_none());
            assert!(receipt.detail.is_some());
        }
        Outcome::Unavailable => {
            assert!(receipt.actual_results_sha256.is_none());
            #[cfg(target_os = "macos")]
            panic!("expected Metal GPU adapter on macOS to compile-fail, got Unavailable");
        }
        other => panic!("invalid WGSL must return Outcome::Fail (or Unavailable), got {other:?}"),
    }
}
