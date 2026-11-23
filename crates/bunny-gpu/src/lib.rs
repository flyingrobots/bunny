//! Quarantined GPU shader artifacts and `wgpu` CPU-vs-GPU equivalence harnesses
//! for Bunny deterministic math kernels (`docs/design/0005-gpu-compute-backend.md`).
//!
//! Core math remains in `bunny-num`; this crate provides portable WGSL shader
//! sources, content-addressed implementation manifests, structured conformance
//! receipts, and `wgpu` compute execution harnesses.

pub mod manifest;
pub mod receipt;
pub mod u32_div_round_even;
pub mod u32_ellipse_classify;

pub use manifest::{sha256_hex, ImplementationManifest};
pub use receipt::{
    AdapterPolicy, ExecutionClass, ExecutionEnvironment, KernelConformanceReceipt, Outcome,
};
pub use u32_div_round_even::{
    assembled_u32_div_round_even_compute_wgsl, load_u32_div_round_even_fixture,
    run_u32_div_round_even_cpu, run_u32_div_round_even_gpu, run_u32_div_round_even_gpu_with_shader,
    u32_div_round_even_manifest, DivRoundEvenFixture, DivVector,
    U32_DIV_ROUND_EVEN_COMPUTE_WRAPPER_WGSL, U32_DIV_ROUND_EVEN_VECTORS_JSON,
    U32_DIV_ROUND_EVEN_WGSL,
};
pub use u32_ellipse_classify::{
    assembled_u32_ellipse_classify_compute_wgsl, load_u32_ellipse_classify_fixture,
    run_u32_ellipse_classify_cpu, run_u32_ellipse_classify_gpu,
    run_u32_ellipse_classify_gpu_with_shader, u32_ellipse_classify_manifest,
    EllipseClassifyFixture, EllipseVector, U32_ELLIPSE_CLASSIFY_COMPUTE_WRAPPER_WGSL,
    U32_ELLIPSE_CLASSIFY_VECTORS_JSON, U32_ELLIPSE_CLASSIFY_WGSL,
};
