//! A sphere against one plane, compared exactly in wide integers.
//!
//! In raw units (value times `2^32`) with `D` the rounded centre distance, `R` the radius
//! and `N` the normal, `d^2 >= r^2 |n|^2` is `D^2 2^64 >= R^2 (Nx^2 + Ny^2 + Nz^2)`.
//! Widths: `D^2 <= 2^126`, so the left side is at most `2^190`; `R^2 < 2^126` and the sum
//! of squares is at most `3 * 2^126 < 2^128`, so the right side is below `3 * 2^252`.
//! Both are held as 256-bit `(high, low)` pairs of `u128`.

use bunny_linalg::FixedVec3;

use crate::{FixedPlane3, PlaneSpan};

const LOW_64: u128 = u128::MAX >> 64;

/// The side of the sphere `(center, radius)` against `plane`; `radius` is raw and not
/// negative. `None` when the centre distance overflows.
pub(super) fn span(plane: FixedPlane3, center: FixedVec3, radius: u64) -> Option<PlaneSpan> {
    let distance = plane.signed_distance(center)?.raw();
    let reach = wide_mul(square(radius), norm_squared(plane.normal));
    let clearance = shifted_64(square(distance.unsigned_abs()));
    Some(if clearance < reach {
        PlaneSpan::Straddling
    } else if distance < 0 {
        PlaneSpan::Behind
    } else {
        PlaneSpan::Front
    })
}

fn square(value: u64) -> u128 {
    u128::from(value) * u128::from(value)
}

/// `Nx^2 + Ny^2 + Nz^2` in raw units squared: at most `3 * 2^126`, so it fits `u128`.
fn norm_squared(normal: FixedVec3) -> u128 {
    let component = |v: bunny_num::FixedQ32_32| square(v.raw().unsigned_abs());
    component(normal.x) + component(normal.y) + component(normal.z)
}

/// `value * 2^64` as `(high, low)`.
const fn shifted_64(value: u128) -> (u128, u128) {
    (value >> 64, value << 64)
}

/// The exact 256-bit product `a * b` as `(high, low)`, from four 64-bit by 64-bit products.
///
/// General for any `u128` operands. Here `a < 2^126` and `b <= 3 * 2^126`, so the middle
/// sum is below `2^62 * 2^64 + 2^64 * 3 * 2^62 = 2^128` and its carry is never set; it is
/// kept so the helper does not depend on its callers' widths.
fn wide_mul(a: u128, b: u128) -> (u128, u128) {
    let (a_hi, a_lo) = (a >> 64, a & LOW_64);
    let (b_hi, b_lo) = (b >> 64, b & LOW_64);
    let (middle, middle_carry) = (a_hi * b_lo).overflowing_add(a_lo * b_hi);
    let (low, low_carry) = (a_lo * b_lo).overflowing_add(middle << 64);
    // The product is below 2^256, so these additions cannot overflow.
    let high = a_hi * b_hi + (middle >> 64) + (u128::from(middle_carry) << 64);
    (high + u128::from(low_carry), low)
}
