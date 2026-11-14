//! The interpolated shading normal at a point of a triangle: `bunny-smooth-normal/1`.

use bunny_linalg::FixedUnitVec3;
use bunny_query::BarycentricNumerators;

use super::wide::{pick, unit, wide, Wide};

/// The versioned name of the rule [`smooth_normal`] computes. Any change to an output bit is a
/// new version.
pub const SMOOTH_NORMAL_ID: &str = "bunny-smooth-normal/1";

/// The shifted denominator is below `2^WEIGHT_BITS`, so each product of a shifted weight and a
/// normal component (at most `2^32 + 1` raw) is below `2^95`.
const WEIGHT_BITS: u32 = 62;

/// The unit normal at the point with barycentric `weights` on a triangle whose vertex unit
/// normals are `normals` (in the order `v0`, `v1`, `v2`).
///
/// With the exact weights `w = (det - u - v, u, v)`:
///
/// 1. **Shared normal.** When every vertex with a nonzero weight has the same normal, that
///    normal is returned unchanged: a corner, an edge between equal normals and a face with
///    equal normals give their vertex normal exactly.
/// 2. **Shift.** Otherwise each `w_i` is shifted right (floored) by the same `k`, the smallest
///    that puts `det >> k` below `2^62`; `k` is zero when `det` is already below `2^62`.
/// 3. **Combine.** `m = sum (w_i >> k) n_i`, exact in `i128` (each component below `2^95`).
///    The scale `det >> k` is not divided out: normalizing makes it cancel.
/// 4. **Normalize.** `m` normalized as `TriangleSurface::new` normalizes a face normal (a
///    power-of-two prescale to `[2^61, 2^62)`, then `FixedUnitVec3::new`).
///
/// Returns `None` when `m` is exactly zero (opposite normals that cancel at this point).
///
/// **Bound.** Let `rho = |sum w_i n_i| / (det * 2^32)`, the length of the exact interpolation in
/// units. Each component is within `1 + 2^-26 / rho` raw units of the exact unit vector along
/// `sum w_i n_i` (within 1 when `k = 0`): one raw unit from step 4, and at most
/// `6 (2^32 + 1) / (rho 2^61)` from the floors of step 2. See
/// `docs/topics/optics/README.md#smooth-normal`.
#[must_use]
pub fn smooth_normal(
    normals: [FixedUnitVec3; 3],
    weights: BarycentricNumerators,
) -> Option<FixedUnitVec3> {
    let (u, v, det) = (weights.u(), weights.v(), weights.det());
    let exact = [det - u - v, u, v];
    if let Some(normal) = shared_normal(normals, exact) {
        return Some(normal);
    }
    let shift = weight_shift(det);
    let terms = normals.map(|n| wide(n.into_inner()));
    let sum = terms.iter().zip(exact).fold([0; 3], |sum: Wide, (normal, weight)| {
        let scaled = weight >> shift;
        [0, 1, 2].map(|axis| pick(sum, axis) + scaled * pick(*normal, axis))
    });
    unit(sum)
}

/// The normal shared by every vertex with a nonzero weight, if they all have the same one.
fn shared_normal(normals: [FixedUnitVec3; 3], weights: [i128; 3]) -> Option<FixedUnitVec3> {
    let mut present = normals.iter().zip(weights).filter(|(_, w)| *w > 0).map(|(n, _)| *n);
    let first = present.next()?;
    present.all(|n| n == first).then_some(first)
}

/// The smallest `k` with `det >> k < 2^62`, for a positive `det`.
const fn weight_shift(det: i128) -> u32 {
    match det.checked_ilog2() {
        Some(bits) => bits.saturating_sub(WEIGHT_BITS - 1),
        None => 0,
    }
}
