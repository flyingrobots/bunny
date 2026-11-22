//! `wgpu` CPU-vs-GPU equivalence tests for `bunny-u32-ellipse-classify/1`
//! (`flyingrobots/bunny#430`).

use bunny_gpu::{
    load_u32_ellipse_classify_fixture, run_u32_ellipse_classify_cpu, run_u32_ellipse_classify_gpu,
    run_u32_ellipse_classify_gpu_with_shader, u32_ellipse_classify_manifest, AdapterPolicy,
    EllipseVector, ExecutionClass, Outcome,
};
use bunny_num::{
    classify_ellipse_u32_words, CRATE_VERSION, U32_ELLIPSE_CLASSIFY_CONTRACT_ID,
    U32_ELLIPSE_MAX_COORD,
};

#[test]
fn uec_gpu_001_fixture_matches_cpu_reference_and_zero_axis_traps() {
    let fixture =
        load_u32_ellipse_classify_fixture().expect("ellipse fixture JSON should parse cleanly");
    assert_eq!(fixture.contract_id, U32_ELLIPSE_CLASSIFY_CONTRACT_ID);
    assert_eq!(fixture.max_coord, U32_ELLIPSE_MAX_COORD);
    assert_eq!(fixture.core_count, 24);
    assert!(fixture.vectors.len() >= 1_400);

    // Pin the raw-polynomial zero-semiaxis traps at core indices 11..14
    assert_eq!(
        (
            fixture.vectors[11].dx,
            fixture.vectors[11].dy,
            fixture.vectors[11].semi_x,
            fixture.vectors[11].semi_y,
            fixture.vectors[11].expected_class,
            fixture.vectors[11].expected_status,
        ),
        (0, 0, 0, 5, 1, 0),
    );
    assert_eq!(
        (
            fixture.vectors[12].dx,
            fixture.vectors[12].dy,
            fixture.vectors[12].semi_x,
            fixture.vectors[12].semi_y,
            fixture.vectors[12].expected_class,
            fixture.vectors[12].expected_status,
        ),
        (0, 1, 0, 5, 2, 0),
    );
    assert_eq!(
        (
            fixture.vectors[13].dx,
            fixture.vectors[13].dy,
            fixture.vectors[13].semi_x,
            fixture.vectors[13].semi_y,
            fixture.vectors[13].expected_class,
            fixture.vectors[13].expected_status,
        ),
        (1, 0, 5, 0, 2, 0),
    );

    for vec in &fixture.vectors {
        assert_eq!(
            classify_ellipse_u32_words(vec.dx, vec.dy, vec.semi_x, vec.semi_y),
            [vec.expected_class, vec.expected_status],
        );
    }

    let cpu_receipt = run_u32_ellipse_classify_cpu(&fixture.vectors);
    assert_eq!(cpu_receipt.outcome, Outcome::Pass);
    assert_eq!(cpu_receipt.execution_class, ExecutionClass::Cpu);
    assert_eq!(
        cpu_receipt.actual_results_sha256.as_deref(),
        Some(cpu_receipt.expected_results_sha256.as_str()),
    );
    assert!(cpu_receipt.detail.is_none());
}

#[test]
fn uec_gpu_002_wgsl_2limb_compute_matches_cpu_bit_for_bit() {
    let fixture =
        load_u32_ellipse_classify_fixture().expect("ellipse fixture JSON should parse cleanly");
    let receipt = run_u32_ellipse_classify_gpu(&fixture.vectors, AdapterPolicy::HardwarePreferred);

    match receipt.outcome {
        Outcome::Pass => {
            assert_eq!(
                receipt.actual_results_sha256.as_deref(),
                Some(receipt.expected_results_sha256.as_str()),
                "GPU 2-limb ellipse output digest must equal CPU expected digest bit-for-bit",
            );
            assert_ne!(receipt.execution_class, ExecutionClass::Cpu);
            assert!(receipt.environment.adapter_name.is_some());
        }
        Outcome::Unavailable => {
            assert!(receipt.actual_results_sha256.is_none());
            assert!(receipt.detail.is_some());
            #[cfg(target_os = "macos")]
            panic!("expected Metal GPU adapter on macOS, got Unavailable: {:?}", receipt.detail);
        }
        other => {
            panic!(
                "GPU ellipse equivalence failed ({other:?}) on {:?} ({:?}): {:?}",
                receipt.execution_class, receipt.environment, receipt.detail
            );
        }
    }
}

#[test]
fn uec_gpu_003_manifest_empty_and_invalid_wgsl_behave_honestly() {
    let m1 = u32_ellipse_classify_manifest();
    let m2 = u32_ellipse_classify_manifest();
    assert_eq!(m1, m2);
    assert_eq!(m1.bunny_num_version, CRATE_VERSION);
    assert_eq!(m1.digest_sha256().expect("digest").len(), 64);

    let cpu_empty = run_u32_ellipse_classify_cpu(&[]);
    assert_eq!(cpu_empty.outcome, Outcome::NotRun);
    assert!(cpu_empty.actual_results_sha256.is_none());

    let gpu_empty = run_u32_ellipse_classify_gpu(&[], AdapterPolicy::HardwarePreferred);
    assert_eq!(gpu_empty.outcome, Outcome::NotRun);
    assert_eq!(gpu_empty.environment.backend, "not-run");
    assert!(gpu_empty.actual_results_sha256.is_none());

    let sample = [EllipseVector {
        dx: 3,
        dy: 4,
        semi_x: 5,
        semi_y: 5,
        expected_class: 1,
        expected_status: 0,
    }];
    let bad = run_u32_ellipse_classify_gpu_with_shader(
        &sample,
        "invalid wgsl !!!",
        AdapterPolicy::HardwarePreferred,
    );
    match bad.outcome {
        Outcome::Fail => {
            assert!(bad.actual_results_sha256.is_none());
            assert!(bad.detail.is_some());
        }
        Outcome::Unavailable => {
            assert!(bad.actual_results_sha256.is_none());
            #[cfg(target_os = "macos")]
            panic!("expected Metal compile failure on macOS, got Unavailable");
        }
        other => panic!("expected Fail or Unavailable for invalid WGSL, got {other:?}"),
    }
}
