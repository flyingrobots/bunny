//! Content-addressed implementation manifest for shared Bunny GPU kernels.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Computes a lowercase 64-hex SHA-256 digest of `bytes`.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Content-addressed implementation manifest connecting a Bunny math contract,
/// portable WGSL helper artifact, compute wrapper, and ordered vector fixture.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImplementationManifest {
    /// Manifest schema version.
    pub manifest_version: String,
    /// Stable Bunny math contract identifier (e.g. `bunny-u32-div-round-even/1`).
    pub contract_id: String,
    /// Crate version of `bunny-gpu` producing this manifest.
    pub bunny_gpu_version: String,
    /// Crate version of `bunny-num` providing the CPU reference.
    pub bunny_num_version: String,
    /// Exact SHA-256 digest of the portable WGSL helper source bytes.
    pub helper_wgsl_sha256: String,
    /// Exact SHA-256 digest of the WGSL compute wrapper source bytes.
    pub compute_wrapper_wgsl_sha256: String,
    /// Exact SHA-256 digest of the assembled WGSL compute module bytes.
    pub assembled_compute_wgsl_sha256: String,
    /// Exact SHA-256 digest of the checked-in ordered vector fixture bytes.
    pub vectors_fixture_sha256: String,
}

impl ImplementationManifest {
    /// Computes a deterministic SHA-256 digest of this manifest serialized as
    /// canonical compact JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if JSON serialization fails.
    pub fn digest_sha256(&self) -> Result<String, serde_json::Error> {
        let json = serde_json::to_vec(self)?;
        Ok(sha256_hex(&json))
    }
}
