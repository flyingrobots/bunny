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
/// Returns `None` when `m`, after the shift of step 2, is exactly zero: normals whose shifted
/// weights cancel at this point. The exact interpolation `sum w_i n_i` can be nonzero then,
/// with `rho` (below) tiny.
///
/// **Bound.** Let `rho = |sum w_i n_i| / (det * 2^32)`, the length of the exact interpolation in
/// units. For steps 2 to 4, each component is within `1 + 2^-26 / rho` raw units of the exact
/// unit vector along `sum w_i n_i` (within 1 when `k = 0`): one raw unit from step 4, and at
/// most `6 (2^32 + 1) / (rho 2^61)` from the floors of step 2. Step 1 returns its input, which
/// a `FixedUnitVec3` holds only to within one raw unit of unit length (length in
/// `[2^32 - 1, 2^32 + 2)` raw), so each component of a shared normal is less than 2 raw units
/// from the exact unit vector along it. See `docs/topics/optics/README.md#smooth-normal`.
///
/// # Examples
///
/// ```
/// use bunny_linalg::{FixedUnitVec3, FixedVec3};
/// use bunny_num::fixed_q32_32::ONE_RAW;
/// use bunny_num::FixedQ32_32;
/// use bunny_optics::smooth_normal;
/// use bunny_query::BarycentricNumerators;
///
/// let unit = |x: i64, y: i64, z: i64| {
///     let [x, y, z] = [x, y, z].map(FixedQ32_32::from_raw);
///     FixedUnitVec3::new(FixedVec3::new(x, y, z)).expect("a nonzero vector")
/// };
/// let (up, east) = (unit(0, 0, ONE_RAW), unit(ONE_RAW, 0, 0));
/// // Halfway along the edge from v0 to v1: weights (1, 1, 0) / 2.
/// let midpoint = BarycentricNumerators::new(1, 0, 2).expect("on the triangle");
/// let normal = smooth_normal([up, east, up], midpoint).expect("the normals do not cancel");
/// let n = normal.into_inner();
/// assert_eq!((n.x, n.y), (n.z, FixedQ32_32::ZERO)); // midway between up and east
/// // At v0 alone the shared-normal step returns v0's normal unchanged.
/// let corner = BarycentricNumerators::new(0, 0, 2).expect("on the triangle");
/// assert_eq!(smooth_normal([up, east, up], corner), Some(up));
/// ```
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
