//! Contract helpers for Bunny schemas and generated types.

pub mod generated;

/// Current authored Bunny contract schema version.
pub const BUNNY_CONTRACT_VERSION: &str = "bunny-contract/0";

pub use generated::graphics::BUNNY_GRAPHICS_SCHEMA_SHA256;
