//! Deterministic numeric profiles for Bunny graphics primitives.

/// Bunny's first scalar representation.
///
/// This is intentionally tiny while the deterministic profile is still being
/// designed. Public APIs should name `Scalar` instead of spelling a primitive
/// float directly.
pub type Scalar = f32;

/// Returns true when the scalar can participate in canonical Bunny contracts.
#[must_use]
pub fn is_finite(value: Scalar) -> bool {
    value.is_finite()
}
