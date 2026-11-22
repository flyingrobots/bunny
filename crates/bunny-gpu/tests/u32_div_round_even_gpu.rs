//! `wgpu` CPU-vs-GPU equivalence tests for `bunny-u32-div-round-even/1` (`flyingrobots/bunny#428`).

use bunny_gpu::{
    load_u32_div_round_even_fixture, run_u32_div_round_even_cpu, run_u32_div_round_even_gpu,
    u32_div_round_even_manifest, AdapterPolicy, ExecutionClass, Outcome,
};
use bunny_num::{div_round_even_u32_words, U32_DIV_ROUND_EVEN_CONTRACT_ID};

#[test]
fn udre_gpu_001_fixture_matches_cpu_reference_and_15_core_vectors() {
    let fixture = load_u32_div_round_even_fixture().expect("fixture JSON should parse cleanly");
    assert_eq!(fixture.contract_id, U32_DIV_ROUND_EVEN_CONTRACT_ID);
    assert_eq!(fixture.core_count, 15);
    assert!(fixture.vectors.len() >= 1_500, "fixture should include core + boundary + seeded tail");

    // Verify key adversarial golden vectors at their pinned core indices
    let overflow_vec = fixture.vectors[10];
    assert_eq!(
        (
            overflow_vec.numerator,
            overflow_vec.denominator,
            overflow_vec.expected_value,
            overflow_vec.expected_status,
        ),
        (3_000_000_000, 4_000_000_000, 1, 0),
        "core vector 10 must be the 2*r overflow counterexample",
    );

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
}
