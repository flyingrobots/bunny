//! Exact integer ellipse trichotomy predicate (`bunny-u32-ellipse-classify/1`).
//!
//! Compares `(dx * semi_y)^2 + (dy * semi_x)^2` against `(semi_x * semi_y)^2`
//! using exact unsigned 64-bit arithmetic (2-limb `u32` on portable WGSL) with
//! zero division, zero rounding, and zero subtraction overflow over
//! `0..=46_340`. Makes no claim of full Q32.32 GPU conformance.

use core::fmt::{self, Display, Formatter};

/// Stable contract identifier for exact bounded unsigned ellipse classification.
pub const U32_ELLIPSE_CLASSIFY_CONTRACT_ID: &str = "bunny-u32-ellipse-classify/1";

/// Maximum permitted value for `dx`, `dy`, `semi_x`, and `semi_y`
/// (`floor(sqrt(2^31 - 1)) = 46_340`, ensuring `p = dx * semi_y < 2^31` and
/// `p^2 + q^2 <= 2^63 < 2^64`).
pub const U32_ELLIPSE_MAX_COORD: u32 = 46_340;

/// Canonical status word for a valid classification.
pub const U32_ELLIPSE_STATUS_OK: u32 = 0;

/// Canonical status word when any operand exceeds [`U32_ELLIPSE_MAX_COORD`].
pub const U32_ELLIPSE_STATUS_OUT_OF_DOMAIN: u32 = 1;

/// Exact trichotomy classification relative to an axis-aligned ellipse.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum EllipseClass {
    /// Strictly inside the ellipse (`lhs < rhs`).
    Inside = 0,
    /// Exactly on the ellipse boundary (`lhs == rhs`, or origin when a semiaxis is zero).
    OnBoundary = 1,
    /// Strictly outside the ellipse (`lhs > rhs`, or non-origin when a semiaxis is zero).
    Outside = 2,
}

impl EllipseClass {
    /// Returns the canonical `u32` interchange code (`0 = Inside`, `1 = OnBoundary`,
    /// `2 = Outside`).
    #[must_use]
    pub const fn code(self) -> u32 {
        self as u32
    }
}

/// Error returned by [`classify_ellipse_u32`] when any operand exceeds
/// [`U32_ELLIPSE_MAX_COORD`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EllipseDomainError;

impl Display for EllipseDomainError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("operand exceeds 46,340 maximum in bunny-u32-ellipse-classify/1")
    }
}

impl core::error::Error for EllipseDomainError {}

const fn in_domain(dx: u32, dy: u32, semi_x: u32, semi_y: u32) -> bool {
    let max = U32_ELLIPSE_MAX_COORD;
    dx <= max && dy <= max && semi_x <= max && semi_y <= max
}

const fn classify_nonzero_axes(dx: u32, dy: u32, semi_x: u32, semi_y: u32) -> EllipseClass {
    let term_x = (dx as u64) * (semi_y as u64);
    let term_y = (dy as u64) * (semi_x as u64);
    let radius = (semi_x as u64) * (semi_y as u64);
    let lhs = term_x * term_x + term_y * term_y;
    let rhs = radius * radius;
    if lhs < rhs {
        EllipseClass::Inside
    } else if lhs == rhs {
        EllipseClass::OnBoundary
    } else {
        EllipseClass::Outside
    }
}

/// Classifies `(dx, dy)` relative to the ellipse with semiaxes `(semi_x, semi_y)`
/// (`bunny-u32-ellipse-classify/1`).
///
/// # Errors
///
/// Returns [`EllipseDomainError`] when any operand exceeds [`U32_ELLIPSE_MAX_COORD`].
pub const fn classify_ellipse_u32(
    dx: u32,
    dy: u32,
    semi_x: u32,
    semi_y: u32,
) -> Result<EllipseClass, EllipseDomainError> {
    if !in_domain(dx, dy, semi_x, semi_y) {
        return Err(EllipseDomainError);
    }
    if semi_x == 0 || semi_y == 0 {
        let class =
            if dx == 0 && dy == 0 { EllipseClass::OnBoundary } else { EllipseClass::Outside };
        return Ok(class);
    }
    Ok(classify_nonzero_axes(dx, dy, semi_x, semi_y))
}

/// Canonical two-word interchange adapter `[class, status]` for
/// `bunny-u32-ellipse-classify/1`.
#[must_use]
pub const fn classify_ellipse_u32_words(dx: u32, dy: u32, semi_x: u32, semi_y: u32) -> [u32; 2] {
    match classify_ellipse_u32(dx, dy, semi_x, semi_y) {
        Ok(class) => [class.code(), U32_ELLIPSE_STATUS_OK],
        Err(EllipseDomainError) => [0, U32_ELLIPSE_STATUS_OUT_OF_DOMAIN],
    }
}

/// Canonical 8-byte little-endian serialization (`class_le4 || status_le4`) for
/// `bunny-u32-ellipse-classify/1`.
#[must_use]
pub const fn classify_ellipse_u32_bytes(dx: u32, dy: u32, semi_x: u32, semi_y: u32) -> [u8; 8] {
    let [class_code, status] = classify_ellipse_u32_words(dx, dy, semi_x, semi_y);
    let [c0, c1, c2, c3] = class_code.to_le_bytes();
    let [s0, s1, s2, s3] = status.to_le_bytes();
    [c0, c1, c2, c3, s0, s1, s2, s3]
}
