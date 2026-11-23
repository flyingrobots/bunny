//! `bunny-u32-ellipse-classify/1` portable 2-limb `u32` WGSL artifacts, ordered
//! vector fixture, and `wgpu` CPU-vs-GPU compute equivalence harness.

use std::sync::mpsc;

use bunny_num::{classify_ellipse_u32_bytes, CRATE_VERSION, U32_ELLIPSE_CLASSIFY_CONTRACT_ID};
use serde::Deserialize;

use crate::manifest::{sha256_hex, ImplementationManifest};
use crate::receipt::{
    AdapterPolicy, ExecutionClass, ExecutionEnvironment, KernelConformanceReceipt, Outcome,
};

/// Portable 2-limb `u32` WGSL helper source for `bunny-u32-ellipse-classify/1`
/// (zero entry points; adoptable directly by downstream shaders).
pub const U32_ELLIPSE_CLASSIFY_WGSL: &str =
    include_str!("../shaders/bunny_u32_ellipse_classify.wgsl");

/// Thin WGSL compute entry wrapper used by Bunny's `wgpu` equivalence harness.
pub const U32_ELLIPSE_CLASSIFY_COMPUTE_WRAPPER_WGSL: &str =
    include_str!("../shaders/bunny_u32_ellipse_classify_compute.wgsl");

/// Checked-in ordered vector fixture (24 core golden + scaled ties/carry stress
/// + 1,024 seeded random tuples).
pub const U32_ELLIPSE_CLASSIFY_VECTORS_JSON: &str =
    include_str!("../fixtures/u32_ellipse_classify_vectors.json");

/// CPU reference implementation source bytes hashed for CPU conformance receipts.
const U32_ELLIPSE_CLASSIFY_CPU_SOURCE_BYTES: &[u8] =
    include_bytes!("../../bunny-num/src/u32_ellipse_classify.rs");

const MANIFEST_VERSION: &str = "bunny-gpu-manifest/1";
const RECEIPT_VERSION: &str = "bunny-gpu-kernel-receipt/1";
const WORKGROUP_SIZE: u32 = 64;

/// A single ordered test vector for `bunny-u32-ellipse-classify/1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EllipseVector {
    /// Horizontal distance from ellipse center (`0..=46,340` valid).
    pub dx: u32,
    /// Vertical distance from ellipse center (`0..=46,340` valid).
    pub dy: u32,
    /// Horizontal semiaxis (`0..=46,340` valid).
    pub semi_x: u32,
    /// Vertical semiaxis (`0..=46,340` valid).
    pub semi_y: u32,
    /// Expected class code (`0 = Inside`, `1 = OnBoundary`, `2 = Outside`, or `0` on error).
    pub expected_class: u32,
    /// Expected status code (`0 = Ok`, `1 = OutOfDomain`).
    pub expected_status: u32,
}

/// Parsed ordered fixture for `bunny-u32-ellipse-classify/1`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EllipseClassifyFixture {
    /// Contract identifier recorded in the fixture.
    pub contract_id: String,
    /// Maximum coordinate recorded in the fixture.
    pub max_coord: u32,
    /// Hex seed used for the randomized tail.
    pub seed: String,
    /// Number of readable core golden vectors at the start of `vectors`.
    pub core_count: usize,
    /// Ordered test vectors.
    pub vectors: Vec<EllipseVector>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawEllipseFixture {
    contract_id: String,
    max_coord: u32,
    seed: String,
    core_count: usize,
    vector_count: usize,
    vectors: Vec<[u32; 6]>,
}

/// Assembles the executable WGSL compute shader by concatenating the portable
/// helper source and the compute wrapper.
#[must_use]
pub fn assembled_u32_ellipse_classify_compute_wgsl() -> String {
    format!("{U32_ELLIPSE_CLASSIFY_WGSL}\n{U32_ELLIPSE_CLASSIFY_COMPUTE_WRAPPER_WGSL}")
}

/// Builds the content-addressed [`ImplementationManifest`] for
/// `bunny-u32-ellipse-classify/1`.
#[must_use]
pub fn u32_ellipse_classify_manifest() -> ImplementationManifest {
    let assembled = assembled_u32_ellipse_classify_compute_wgsl();
    ImplementationManifest {
        manifest_version: MANIFEST_VERSION.to_owned(),
        contract_id: U32_ELLIPSE_CLASSIFY_CONTRACT_ID.to_owned(),
        bunny_gpu_version: env!("CARGO_PKG_VERSION").to_owned(),
        bunny_num_version: CRATE_VERSION.to_owned(),
        helper_wgsl_sha256: sha256_hex(U32_ELLIPSE_CLASSIFY_WGSL.as_bytes()),
        compute_wrapper_wgsl_sha256: sha256_hex(
            U32_ELLIPSE_CLASSIFY_COMPUTE_WRAPPER_WGSL.as_bytes(),
        ),
        assembled_compute_wgsl_sha256: sha256_hex(assembled.as_bytes()),
        vectors_fixture_sha256: sha256_hex(U32_ELLIPSE_CLASSIFY_VECTORS_JSON.as_bytes()),
    }
}

/// Parses and validates the checked-in `u32_ellipse_classify_vectors.json` fixture.
///
/// # Errors
///
/// Returns an error if the JSON is malformed or its `vectorCount` disagrees
/// with the vector list length.
pub fn load_u32_ellipse_classify_fixture() -> Result<EllipseClassifyFixture, String> {
    let raw: RawEllipseFixture = serde_json::from_str(U32_ELLIPSE_CLASSIFY_VECTORS_JSON)
        .map_err(|err| format!("failed to parse ellipse fixture JSON: {err}"))?;
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
        .map(|[dx, dy, semi_x, semi_y, expected_class, expected_status]| EllipseVector {
            dx,
            dy,
            semi_x,
            semi_y,
            expected_class,
            expected_status,
        })
        .collect();
    Ok(EllipseClassifyFixture {
        contract_id: raw.contract_id,
        max_coord: raw.max_coord,
        seed: raw.seed,
        core_count: raw.core_count,
        vectors,
    })
}

fn serialize_inputs(vectors: &[EllipseVector]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vectors.len() * 16);
    for vec in vectors {
        bytes.extend_from_slice(&vec.dx.to_le_bytes());
        bytes.extend_from_slice(&vec.dy.to_le_bytes());
        bytes.extend_from_slice(&vec.semi_x.to_le_bytes());
        bytes.extend_from_slice(&vec.semi_y.to_le_bytes());
    }
    bytes
}

fn compute_cpu_expected_bytes(vectors: &[EllipseVector]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vectors.len() * 8);
    for vec in vectors {
        bytes
            .extend_from_slice(&classify_ellipse_u32_bytes(vec.dx, vec.dy, vec.semi_x, vec.semi_y));
    }
    bytes
}

fn first_mismatch_detail(
    vectors: &[EllipseVector],
    expected: &[u8],
    actual: &[u8],
) -> Option<String> {
    for (idx, vec) in vectors.iter().enumerate() {
        let start = idx * 8;
        let end = start + 8;
        let exp_slice = expected.get(start..end);
        let act_slice = actual.get(start..end);
        if exp_slice != act_slice {
            return Some(format!(
                "mismatch at vector[{idx}] ({},{},{},{}): expected={exp_slice:?}, actual={act_slice:?}",
                vec.dx, vec.dy, vec.semi_x, vec.semi_y
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

/// Executes the CPU reference (`bunny-num`) over `vectors` and emits a Layer 2
/// kernel conformance receipt.
#[must_use]
pub fn run_u32_ellipse_classify_cpu(vectors: &[EllipseVector]) -> KernelConformanceReceipt {
    let manifest = u32_ellipse_classify_manifest();
    let manifest_sha = manifest.digest_sha256().unwrap_or_default();
    let input_bytes = serialize_inputs(vectors);
    let cpu_bytes = compute_cpu_expected_bytes(vectors);
    let mut fixture_bytes = Vec::with_capacity(vectors.len() * 8);
    for vec in vectors {
        fixture_bytes.extend_from_slice(&vec.expected_class.to_le_bytes());
        fixture_bytes.extend_from_slice(&vec.expected_status.to_le_bytes());
    }
    let expected_sha = sha256_hex(&fixture_bytes);
    let env = ExecutionEnvironment {
        os: std::env::consts::OS.to_owned(),
        arch: std::env::consts::ARCH.to_owned(),
        backend: "cpu-rust".to_owned(),
        adapter_name: None,
        driver: None,
        driver_info: None,
    };
    let cpu_artifact_sha = sha256_hex(U32_ELLIPSE_CLASSIFY_CPU_SOURCE_BYTES);
    if vectors.is_empty() {
        return KernelConformanceReceipt {
            receipt_version: RECEIPT_VERSION.to_owned(),
            contract_id: U32_ELLIPSE_CLASSIFY_CONTRACT_ID.to_owned(),
            implementation_manifest_sha256: manifest_sha,
            execution_artifact_sha256: cpu_artifact_sha,
            vectors_sha256: sha256_hex(&input_bytes),
            expected_results_sha256: expected_sha,
            actual_results_sha256: None,
            vector_count: 0,
            outcome: Outcome::NotRun,
            execution_class: ExecutionClass::Cpu,
            environment: env,
            detail: Some("vector list is empty".to_owned()),
        };
    }

    let actual_sha = sha256_hex(&cpu_bytes);
    let detail = first_mismatch_detail(vectors, &fixture_bytes, &cpu_bytes);
    let outcome = if detail.is_none() { Outcome::Pass } else { Outcome::Fail };

    KernelConformanceReceipt {
        receipt_version: RECEIPT_VERSION.to_owned(),
        contract_id: U32_ELLIPSE_CLASSIFY_CONTRACT_ID.to_owned(),
        implementation_manifest_sha256: manifest_sha,
        execution_artifact_sha256: cpu_artifact_sha,
        vectors_sha256: sha256_hex(&input_bytes),
        expected_results_sha256: expected_sha,
        actual_results_sha256: Some(actual_sha),
        vector_count: vectors.len(),
        outcome,
        execution_class: ExecutionClass::Cpu,
        environment: env,
        detail,
    }
}

struct RunContext {
    manifest_sha: String,
    shader_sha: String,
    vectors_sha: String,
    expected_sha: String,
    vector_count: usize,
}

struct TargetOutcome {
    actual_results_sha256: Option<String>,
    outcome: Outcome,
    execution_class: ExecutionClass,
    environment: ExecutionEnvironment,
    detail: Option<String>,
}

impl RunContext {
    fn receipt(&self, result: TargetOutcome) -> KernelConformanceReceipt {
        KernelConformanceReceipt {
            receipt_version: RECEIPT_VERSION.to_owned(),
            contract_id: U32_ELLIPSE_CLASSIFY_CONTRACT_ID.to_owned(),
            implementation_manifest_sha256: self.manifest_sha.clone(),
            execution_artifact_sha256: self.shader_sha.clone(),
            vectors_sha256: self.vectors_sha.clone(),
            expected_results_sha256: self.expected_sha.clone(),
            actual_results_sha256: result.actual_results_sha256,
            vector_count: self.vector_count,
            outcome: result.outcome,
            execution_class: result.execution_class,
            environment: result.environment,
            detail: result.detail,
        }
    }

    fn unavailable(&self, detail: String) -> KernelConformanceReceipt {
        self.receipt(TargetOutcome {
            actual_results_sha256: None,
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
        })
    }

    fn not_run(&self, detail: String) -> KernelConformanceReceipt {
        self.receipt(TargetOutcome {
            actual_results_sha256: None,
            outcome: Outcome::NotRun,
            execution_class: ExecutionClass::Unknown,
            environment: ExecutionEnvironment {
                os: std::env::consts::OS.to_owned(),
                arch: std::env::consts::ARCH.to_owned(),
                backend: "not-run".to_owned(),
                adapter_name: None,
                driver: None,
                driver_info: None,
            },
            detail: Some(detail),
        })
    }
}

struct ErrorScopes {
    internal: wgpu::ErrorScopeGuard,
    oom: wgpu::ErrorScopeGuard,
    validation: wgpu::ErrorScopeGuard,
}

impl ErrorScopes {
    fn push(device: &wgpu::Device) -> Self {
        let internal = device.push_error_scope(wgpu::ErrorFilter::Internal);
        let oom = device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
        Self { internal, oom, validation }
    }

    fn pop(self) -> Result<(), String> {
        let val = pollster::block_on(self.validation.pop());
        let oom = pollster::block_on(self.oom.pop());
        let internal = pollster::block_on(self.internal.pop());
        if let Some(err) = val.or(oom).or(internal) {
            return Err(err.to_string());
        }
        Ok(())
    }
}

fn dispatch_ellipse_shader(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    shader_source: &str,
    input_bytes: &[u8],
) -> Result<Vec<u8>, String> {
    let vector_count = input_bytes.len() / 16;
    let in_byte_len =
        u64::try_from(input_bytes.len()).map_err(|_| "input byte length exceeds u64".to_owned())?;
    let out_byte_len =
        u64::try_from(vector_count * 8).map_err(|_| "output byte length exceeds u64".to_owned())?;
    let count_u32 =
        u32::try_from(vector_count).map_err(|_| "vector count exceeds u32".to_owned())?;
    let workgroups = count_u32.div_ceil(WORKGROUP_SIZE);
    let max_wg = device.limits().max_compute_workgroups_per_dimension;
    if workgroups > max_wg {
        return Err(format!("dispatch workgroups ({workgroups}) exceeds device limit ({max_wg})"));
    }

    let scopes = ErrorScopes::push(device);

    let input_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("bunny-ellipse-inputs"),
        size: in_byte_len,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&input_buf, 0, input_bytes);

    let output_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("bunny-ellipse-outputs"),
        size: out_byte_len,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let staging_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("bunny-ellipse-staging"),
        size: out_byte_len,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("bunny-u32-ellipse-classify"),
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("bunny-u32-ellipse-classify-pipeline"),
        layout: None,
        module: &module,
        entry_point: Some("main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bunny-ellipse-bind-group"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: input_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: output_buf.as_entire_binding() },
        ],
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("bunny-ellipse-encoder"),
    });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("bunny-ellipse-pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(workgroups, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output_buf, 0, &staging_buf, 0, out_byte_len);
    queue.submit(Some(encoder.finish()));

    scopes.pop()?;

    let slice = staging_buf.slice(..);
    let (tx, rx) = mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |res| {
        let _ = tx.send(res);
    });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|err| format!("device poll failed: {err}"))?;
    rx.recv()
        .map_err(|err| format!("map channel closed: {err}"))?
        .map_err(|err| format!("staging map failed: {err}"))?;

    let actual_bytes = slice.get_mapped_range().to_vec();
    staging_buf.unmap();
    Ok(actual_bytes)
}

/// Executes the assembled WGSL compute shader for `bunny-u32-ellipse-classify/1`
/// on `wgpu` over `vectors`, compares canonical output bytes bit-for-bit against
/// `bunny-num`, and returns a structured receipt.
#[must_use]
pub fn run_u32_ellipse_classify_gpu(
    vectors: &[EllipseVector],
    adapter_policy: AdapterPolicy,
) -> KernelConformanceReceipt {
    let shader_source = assembled_u32_ellipse_classify_compute_wgsl();
    run_u32_ellipse_classify_gpu_with_shader(vectors, &shader_source, adapter_policy)
}

/// Executes an explicit WGSL compute shader source over `vectors` and compares
/// canonical output bytes bit-for-bit against `bunny-num`.
#[must_use]
pub fn run_u32_ellipse_classify_gpu_with_shader(
    vectors: &[EllipseVector],
    shader_source: &str,
    adapter_policy: AdapterPolicy,
) -> KernelConformanceReceipt {
    let manifest = u32_ellipse_classify_manifest();
    let input_bytes = serialize_inputs(vectors);
    let expected_bytes = compute_cpu_expected_bytes(vectors);
    let ctx = RunContext {
        manifest_sha: manifest.digest_sha256().unwrap_or_default(),
        shader_sha: sha256_hex(shader_source.as_bytes()),
        vectors_sha: sha256_hex(&input_bytes),
        expected_sha: sha256_hex(&expected_bytes),
        vector_count: vectors.len(),
    };

    if vectors.is_empty() {
        return ctx.not_run("vector list is empty".to_owned());
    }

    let instance = wgpu::Instance::default();
    let force_fallback = matches!(adapter_policy, AdapterPolicy::SoftwareFallbackOnly);
    let adapter = match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: force_fallback,
    })) {
        Ok(adapter) => adapter,
        Err(err) => return ctx.unavailable(format!("no compatible wgpu adapter: {err}")),
    };

    let info = adapter.get_info();
    let exec_class = ExecutionClass::from_device_type(info.device_type);
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
            Err(err) => return ctx.unavailable(format!("failed to open wgpu device: {err}")),
        };

    match dispatch_ellipse_shader(&device, &queue, shader_source, &input_bytes) {
        Ok(actual_bytes) => {
            let actual_sha = sha256_hex(&actual_bytes);
            let detail = first_mismatch_detail(vectors, &expected_bytes, &actual_bytes);
            let outcome = if detail.is_none() { Outcome::Pass } else { Outcome::Fail };
            ctx.receipt(TargetOutcome {
                actual_results_sha256: Some(actual_sha),
                outcome,
                execution_class: exec_class,
                environment,
                detail,
            })
        }
        Err(err) => ctx.receipt(TargetOutcome {
            actual_results_sha256: None,
            outcome: Outcome::Fail,
            execution_class: exec_class,
            environment,
            detail: Some(err),
        }),
    }
}
