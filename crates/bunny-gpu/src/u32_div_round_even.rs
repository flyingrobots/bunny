//! `bunny-u32-div-round-even/1` WGSL artifacts, ordered vector fixture, and
//! `wgpu` CPU-vs-GPU compute equivalence harness.

use std::sync::mpsc;

use bunny_num::{div_round_even_u32_bytes, U32_DIV_ROUND_EVEN_CONTRACT_ID};
use serde::Deserialize;

use crate::manifest::{sha256_hex, ImplementationManifest};
use crate::receipt::{
    AdapterPolicy, ExecutionClass, ExecutionEnvironment, KernelConformanceReceipt, Outcome,
};

/// Portable WGSL helper source for `bunny-u32-div-round-even/1` (zero entry
/// points; adoptable directly by downstream shaders).
pub const U32_DIV_ROUND_EVEN_WGSL: &str = include_str!("../shaders/bunny_u32_div_round_even.wgsl");

/// Thin WGSL compute entry wrapper used by Bunny's `wgpu` equivalence harness.
pub const U32_DIV_ROUND_EVEN_COMPUTE_WRAPPER_WGSL: &str =
    include_str!("../shaders/bunny_u32_div_round_even_compute.wgsl");

/// Checked-in ordered vector fixture (15 core golden + boundary grid + large
/// ties + 1,024 seeded random pairs).
pub const U32_DIV_ROUND_EVEN_VECTORS_JSON: &str =
    include_str!("../fixtures/u32_div_round_even_vectors.json");

const MANIFEST_VERSION: &str = "bunny-gpu-manifest/1";
const RECEIPT_VERSION: &str = "bunny-gpu-kernel-receipt/1";
const WORKGROUP_SIZE: u32 = 64;

/// A single ordered test vector for `bunny-u32-div-round-even/1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DivVector {
    /// Unsigned 32-bit numerator.
    pub numerator: u32,
    /// Unsigned 32-bit denominator.
    pub denominator: u32,
    /// Expected rounded quotient (`0` when `expected_status == 1`).
    pub expected_value: u32,
    /// Expected canonical status word (`0 = Success`, `1 = DivisionByZero`).
    pub expected_status: u32,
}

/// Parsed ordered fixture for `bunny-u32-div-round-even/1`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DivRoundEvenFixture {
    /// Contract identifier recorded in the fixture.
    pub contract_id: String,
    /// Hex seed used for the randomized tail.
    pub seed: String,
    /// Number of readable core golden vectors at the start of `vectors`.
    pub core_count: usize,
    /// Ordered test vectors.
    pub vectors: Vec<DivVector>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawFixture {
    contract_id: String,
    seed: String,
    core_count: usize,
    vector_count: usize,
    vectors: Vec<[u32; 4]>,
}

/// Assembles the executable WGSL compute shader by concatenating the portable
/// helper source and the compute wrapper.
#[must_use]
pub fn assembled_u32_div_round_even_compute_wgsl() -> String {
    format!("{U32_DIV_ROUND_EVEN_WGSL}\n{U32_DIV_ROUND_EVEN_COMPUTE_WRAPPER_WGSL}")
}

/// Builds the content-addressed [`ImplementationManifest`] for
/// `bunny-u32-div-round-even/1`.
#[must_use]
pub fn u32_div_round_even_manifest() -> ImplementationManifest {
    let assembled = assembled_u32_div_round_even_compute_wgsl();
    ImplementationManifest {
        manifest_version: MANIFEST_VERSION.to_owned(),
        contract_id: U32_DIV_ROUND_EVEN_CONTRACT_ID.to_owned(),
        bunny_gpu_version: env!("CARGO_PKG_VERSION").to_owned(),
        bunny_num_version: "0.7.1".to_owned(),
        helper_wgsl_sha256: sha256_hex(U32_DIV_ROUND_EVEN_WGSL.as_bytes()),
        compute_wrapper_wgsl_sha256: sha256_hex(U32_DIV_ROUND_EVEN_COMPUTE_WRAPPER_WGSL.as_bytes()),
        assembled_compute_wgsl_sha256: sha256_hex(assembled.as_bytes()),
        vectors_fixture_sha256: sha256_hex(U32_DIV_ROUND_EVEN_VECTORS_JSON.as_bytes()),
    }
}

/// Parses and validates the checked-in `u32_div_round_even_vectors.json` fixture.
///
/// # Errors
///
/// Returns an error if the JSON is malformed or its `vectorCount` disagrees
/// with the vector list length.
pub fn load_u32_div_round_even_fixture() -> Result<DivRoundEvenFixture, String> {
    let raw: RawFixture = serde_json::from_str(U32_DIV_ROUND_EVEN_VECTORS_JSON)
        .map_err(|err| format!("failed to parse fixture JSON: {err}"))?;
    if raw.vector_count != raw.vectors.len() {
        return Err(format!(
            "fixture vectorCount ({}) != vectors.len() ({})",
            raw.vector_count,
            raw.vectors.len()
        ));
    }
    let vectors = raw
        .vectors
        .into_iter()
        .map(|[numerator, denominator, expected_value, expected_status]| DivVector {
            numerator,
            denominator,
            expected_value,
            expected_status,
        })
        .collect();
    Ok(DivRoundEvenFixture {
        contract_id: raw.contract_id,
        seed: raw.seed,
        core_count: raw.core_count,
        vectors,
    })
}

fn serialize_inputs(vectors: &[DivVector]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vectors.len() * 8);
    for vec in vectors {
        bytes.extend_from_slice(&vec.numerator.to_le_bytes());
        bytes.extend_from_slice(&vec.denominator.to_le_bytes());
    }
    bytes
}

fn compute_cpu_expected_bytes(vectors: &[DivVector]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vectors.len() * 8);
    for vec in vectors {
        bytes.extend_from_slice(&div_round_even_u32_bytes(vec.numerator, vec.denominator));
    }
    bytes
}

/// Executes the CPU reference (`bunny-num`) over `vectors` and emits a Layer 2
/// kernel conformance receipt.
#[must_use]
pub fn run_u32_div_round_even_cpu(vectors: &[DivVector]) -> KernelConformanceReceipt {
    let manifest = u32_div_round_even_manifest();
    let manifest_sha = manifest.digest_sha256().unwrap_or_default();
    let input_bytes = serialize_inputs(vectors);
    let cpu_bytes = compute_cpu_expected_bytes(vectors);
    let mut fixture_bytes = Vec::with_capacity(vectors.len() * 8);
    for vec in vectors {
        fixture_bytes.extend_from_slice(&vec.expected_value.to_le_bytes());
        fixture_bytes.extend_from_slice(&vec.expected_status.to_le_bytes());
    }
    let expected_sha = sha256_hex(&fixture_bytes);
    let actual_sha = sha256_hex(&cpu_bytes);
    let outcome = if cpu_bytes == fixture_bytes { Outcome::Pass } else { Outcome::Fail };

    KernelConformanceReceipt {
        receipt_version: RECEIPT_VERSION.to_owned(),
        contract_id: U32_DIV_ROUND_EVEN_CONTRACT_ID.to_owned(),
        implementation_manifest_sha256: manifest_sha,
        execution_artifact_sha256: sha256_hex(b"bunny-num::div_round_even_u32_bytes"),
        vectors_sha256: sha256_hex(&input_bytes),
        expected_results_sha256: expected_sha,
        actual_results_sha256: Some(actual_sha),
        vector_count: vectors.len(),
        outcome,
        execution_class: ExecutionClass::Cpu,
        environment: ExecutionEnvironment {
            os: std::env::consts::OS.to_owned(),
            arch: std::env::consts::ARCH.to_owned(),
            backend: "cpu-rust".to_owned(),
            adapter_name: None,
            driver: None,
            driver_info: None,
        },
        detail: None,
    }
}

fn classify_device_type(device_type: wgpu::DeviceType) -> ExecutionClass {
    match device_type {
        wgpu::DeviceType::DiscreteGpu | wgpu::DeviceType::IntegratedGpu => {
            ExecutionClass::HardwareGpu
        }
        wgpu::DeviceType::Cpu | wgpu::DeviceType::VirtualGpu => ExecutionClass::SoftwareGpu,
        wgpu::DeviceType::Other => ExecutionClass::Unknown,
    }
}

fn unavailable_receipt(
    manifest_sha: String,
    shader_sha: String,
    vectors_sha: String,
    expected_sha: String,
    vector_count: usize,
    detail: String,
) -> KernelConformanceReceipt {
    KernelConformanceReceipt {
        receipt_version: RECEIPT_VERSION.to_owned(),
        contract_id: U32_DIV_ROUND_EVEN_CONTRACT_ID.to_owned(),
        implementation_manifest_sha256: manifest_sha,
        execution_artifact_sha256: shader_sha,
        vectors_sha256: vectors_sha,
        expected_results_sha256: expected_sha,
        actual_results_sha256: None,
        vector_count,
        outcome: Outcome::Unavailable,
        execution_class: ExecutionClass::Unknown,
        environment: ExecutionEnvironment {
            os: std::env::consts::OS.to_owned(),
            arch: std::env::consts::ARCH.to_owned(),
            backend: "unavailable".to_owned(),
            adapter_name: None,
            driver: None,
            driver_info: None,
        },
        detail: Some(detail),
    }
}

fn first_mismatch_detail(vectors: &[DivVector], expected: &[u8], actual: &[u8]) -> Option<String> {
    for (idx, vec) in vectors.iter().enumerate() {
        let start = idx * 8;
        let end = start + 8;
        let exp_slice = expected.get(start..end);
        let act_slice = actual.get(start..end);
        if exp_slice != act_slice {
            return Some(format!(
                "mismatch at vector[{idx}] ({} / {}): expected={exp_slice:?}, actual={act_slice:?}",
                vec.numerator, vec.denominator
            ));
        }
    }
    if expected.len() != actual.len() {
        return Some(format!(
            "byte length mismatch: expected={}, actual={}",
            expected.len(),
            actual.len()
        ));
    }
    None
}

/// Executes the assembled WGSL compute shader for `bunny-u32-div-round-even/1`
/// on `wgpu` over `vectors`, compares canonical little-endian output bytes
/// bit-for-bit against `bunny-num`, and returns a structured receipt.
#[must_use]
pub fn run_u32_div_round_even_gpu(
    vectors: &[DivVector],
    adapter_policy: AdapterPolicy,
) -> KernelConformanceReceipt {
    let manifest = u32_div_round_even_manifest();
    let manifest_sha = manifest.digest_sha256().unwrap_or_default();
    let shader_source = assembled_u32_div_round_even_compute_wgsl();
    let shader_sha = sha256_hex(shader_source.as_bytes());
    let input_bytes = serialize_inputs(vectors);
    let expected_bytes = compute_cpu_expected_bytes(vectors);
    let vectors_sha = sha256_hex(&input_bytes);
    let expected_sha = sha256_hex(&expected_bytes);

    if vectors.is_empty() {
        return unavailable_receipt(
            manifest_sha,
            shader_sha,
            vectors_sha,
            expected_sha,
            0,
            "vector list is empty".to_owned(),
        );
    }

    let instance = wgpu::Instance::default();
    let force_fallback = matches!(adapter_policy, AdapterPolicy::SoftwareFallbackOnly);
    let adapter = match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: force_fallback,
    })) {
        Ok(adapter) => adapter,
        Err(err) => {
            return unavailable_receipt(
                manifest_sha,
                shader_sha,
                vectors_sha,
                expected_sha,
                vectors.len(),
                format!("no compatible wgpu adapter: {err}"),
            );
        }
    };

    let info = adapter.get_info();
    let exec_class = classify_device_type(info.device_type);
    let environment = ExecutionEnvironment {
        os: std::env::consts::OS.to_owned(),
        arch: std::env::consts::ARCH.to_owned(),
        backend: format!("{:?}", info.backend),
        adapter_name: Some(info.name),
        driver: Some(info.driver),
        driver_info: Some(info.driver_info),
    };

    let (device, queue) =
        match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())) {
            Ok(pair) => pair,
            Err(err) => {
                return unavailable_receipt(
                    manifest_sha,
                    shader_sha,
                    vectors_sha,
                    expected_sha,
                    vectors.len(),
                    format!("failed to open wgpu device: {err}"),
                );
            }
        };

    let byte_len = u64::try_from(input_bytes.len()).expect("buffer byte length fits u64");
    let input_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("bunny-div-inputs"),
        size: byte_len,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&input_buf, 0, &input_bytes);

    let output_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("bunny-div-outputs"),
        size: byte_len,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let staging_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("bunny-div-staging"),
        size: byte_len,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("bunny-u32-div-round-even"),
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("bunny-u32-div-round-even-pipeline"),
        layout: None,
        module: &module,
        entry_point: Some("main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });

    let bind_group_layout = pipeline.get_bind_group_layout(0);
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bunny-div-bind-group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: input_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: output_buf.as_entire_binding() },
        ],
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("bunny-div-encoder"),
    });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("bunny-div-pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        let count_u32 = u32::try_from(vectors.len()).expect("vector count fits u32");
        let workgroups = count_u32.div_ceil(WORKGROUP_SIZE);
        pass.dispatch_workgroups(workgroups, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output_buf, 0, &staging_buf, 0, byte_len);
    queue.submit(Some(encoder.finish()));

    let slice = staging_buf.slice(..);
    let (tx, rx) = mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |res| {
        let _ = tx.send(res);
    });
    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    let map_result = rx.recv().map_err(|err| format!("map channel closed: {err}"));
    if let Err(err) = map_result.and_then(|inner| inner.map_err(|e| format!("map failed: {e}"))) {
        return KernelConformanceReceipt {
            receipt_version: RECEIPT_VERSION.to_owned(),
            contract_id: U32_DIV_ROUND_EVEN_CONTRACT_ID.to_owned(),
            implementation_manifest_sha256: manifest_sha,
            execution_artifact_sha256: shader_sha,
            vectors_sha256: vectors_sha,
            expected_results_sha256: expected_sha,
            actual_results_sha256: None,
            vector_count: vectors.len(),
            outcome: Outcome::Fail,
            execution_class: exec_class,
            environment,
            detail: Some(err),
        };
    }

    let actual_bytes: Vec<u8> = slice.get_mapped_range().to_vec();
    staging_buf.unmap();

    let actual_sha = sha256_hex(&actual_bytes);
    let detail = first_mismatch_detail(vectors, &expected_bytes, &actual_bytes);
    let outcome = if detail.is_none() { Outcome::Pass } else { Outcome::Fail };

    KernelConformanceReceipt {
        receipt_version: RECEIPT_VERSION.to_owned(),
        contract_id: U32_DIV_ROUND_EVEN_CONTRACT_ID.to_owned(),
        implementation_manifest_sha256: manifest_sha,
        execution_artifact_sha256: shader_sha,
        vectors_sha256: vectors_sha,
        expected_results_sha256: expected_sha,
        actual_results_sha256: Some(actual_sha),
        vector_count: vectors.len(),
        outcome,
        execution_class: exec_class,
        environment,
        detail,
    }
}
