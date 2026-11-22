//! Structured kernel conformance receipt types for Layer 2 verification.

use serde::{Deserialize, Serialize};

/// Outcome of a kernel conformance execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    /// All executed vectors matched the canonical CPU reference bit-for-bit.
    Pass,
    /// Execution completed, or shader compilation failed on an available path,
    /// and output did not match the canonical CPU reference.
    Fail,
    /// No compatible GPU adapter/device was available on this host.
    Unavailable,
    /// Execution was skipped before attempting adapter selection.
    NotRun,
}

/// Classification of the execution target that produced the receipt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExecutionClass {
    /// Pure Rust CPU reference implementation (`bunny-num`).
    Cpu,
    /// Software GPU implementation (e.g., Mesa lavapipe, WARP, `SwiftShader`).
    SoftwareGpu,
    /// Dedicated or integrated hardware GPU (e.g., Apple Metal, Vulkan, DX12).
    HardwareGpu,
    /// Adapter device type does not establish the execution class (virtual or other adapters).
    Unknown,
}

impl ExecutionClass {
    /// Classifies a `wgpu` adapter by the device type it reports.
    ///
    /// Only `Cpu` identifies software execution. A `VirtualGpu` is a virtual or hosted
    /// adapter, which may pass through to hardware, so it is `Unknown` rather than
    /// `SoftwareGpu`.
    #[must_use]
    pub const fn from_device_type(device_type: wgpu::DeviceType) -> Self {
        match device_type {
            wgpu::DeviceType::DiscreteGpu | wgpu::DeviceType::IntegratedGpu => Self::HardwareGpu,
            wgpu::DeviceType::Cpu => Self::SoftwareGpu,
            wgpu::DeviceType::VirtualGpu | wgpu::DeviceType::Other => Self::Unknown,
        }
    }
}

/// Adapter selection preference for `wgpu` equivalence runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdapterPolicy {
    /// Prefer a hardware GPU adapter, falling back to any available adapter.
    HardwarePreferred,
    /// Request only a fallback (software) adapter via `force_fallback_adapter`.
    SoftwareFallbackOnly,
}

/// Host and adapter metadata recorded alongside a kernel conformance run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionEnvironment {
    /// Target operating system (`std::env::consts::OS`).
    pub os: String,
    /// Target CPU architecture (`std::env::consts::ARCH`).
    pub arch: String,
    /// Execution backend name (`cpu-rust`, `Metal`, `Vulkan`, `Dx12`, etc.).
    pub backend: String,
    /// Adapter display name when a GPU adapter was selected.
    pub adapter_name: Option<String>,
    /// Adapter driver description when available.
    pub driver: Option<String>,
    /// Adapter driver info string when available.
    pub driver_info: Option<String>,
}

/// Structured Layer 2 kernel conformance receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelConformanceReceipt {
    /// Receipt schema version.
    pub receipt_version: String,
    /// Math contract identifier.
    pub contract_id: String,
    /// SHA-256 digest of the [`crate::ImplementationManifest`].
    pub implementation_manifest_sha256: String,
    /// SHA-256 digest of the actual executed artifact (assembled WGSL or CPU reference).
    pub execution_artifact_sha256: String,
    /// SHA-256 digest of the canonical input vector bytes (`n_le4 || d_le4` per vector).
    pub vectors_sha256: String,
    /// SHA-256 digest of the canonical expected output bytes (`value_le4 || status_le4`).
    pub expected_results_sha256: String,
    /// SHA-256 digest of the actual observed output bytes, or `None` when unavailable.
    pub actual_results_sha256: Option<String>,
    /// Number of vectors in the ordered fixture.
    pub vector_count: usize,
    /// Conformance outcome (`pass`, `fail`, `unavailable`, `not-run`).
    pub outcome: Outcome,
    /// Execution class (`cpu`, `software-gpu`, `hardware-gpu`, `unknown`).
    pub execution_class: ExecutionClass,
    /// Observed environment and adapter details.
    pub environment: ExecutionEnvironment,
    /// Optional diagnostic detail when `outcome` is `fail` or `unavailable`.
    pub detail: Option<String>,
}
