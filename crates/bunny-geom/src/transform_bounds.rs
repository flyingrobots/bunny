//! Transform-aware bounds: the smallest raw box that contains the **exact** image of a box, a
//! point set, a triangle or a sphere under a [`FixedAffine3`]. See
//! `docs/topics/transform/README.md#transform-aware-bounds`.
//!
//! In raw units (value times `2^32`), a coordinate of the exact image of `p` is
//! `t_i + (m_i0 p_0 + m_i1 p_1 + m_i2 p_2) / 2^32`. Each product of two raw values is exact in
//! `i128` (magnitude at most `2^126`). A sum of three such products can reach `3 * 2^126`, past
//! `i128`, so each product is split into `q * 2^32 + r` with `0 <= r < 2^32` and the parts are
//! summed apart: the quotients stay below `2^96` and the remainders below `3 * 2^32`. That
//! gives the exact value as `floor * 2^32 + fraction`, with nothing rounded. A face is the
//! floor of the exact minimum or the ceiling of the exact maximum, the only rounding, and it is
//! outward. A result face outside `i64` is refused; nothing wraps or saturates.

use bunny_linalg::{FixedAffine3, FixedMat3, FixedVec3};
use bunny_num::FixedQ32_32;

use crate::{BoundsError, FixedAabb3, FixedSphere3};

mod wide;

const FRACTION_BITS: u32 = 32;
const FRACTION_MASK: i128 = (1 << FRACTION_BITS) - 1;

/// An exact raw coordinate `floor + fraction / 2^32`, with `0 <= fraction < 2^32`.
#[derive(Clone, Copy)]
struct Exact {
    floor: i128,
    fraction: i128,
}

impl Exact {
    /// The exact `translation + (p0 + p1 + p2) / 2^32` of three exact raw products.
    fn sum(translation: i64, products: [i128; 3]) -> Self {
        let quotients: i128 = products.iter().map(|p| p >> FRACTION_BITS).sum();
        let remainders: i128 = products.iter().map(|p| p & FRACTION_MASK).sum();
        Self {
            floor: i128::from(translation) + quotients + (remainders >> FRACTION_BITS),
            fraction: remainders & FRACTION_MASK,
        }
    }

    const fn ceil(self) -> i128 {
        self.floor + if self.fraction == 0 { 0 } else { 1 }
    }
}

/// The rows of `linear` as raw entries.
const fn rows(linear: FixedMat3) -> [[i64; 3]; 3] {
    [raw3(linear.row0()), raw3(linear.row1()), raw3(linear.row2())]
}

const fn raw3(v: FixedVec3) -> [i64; 3] {
    [v.x.raw(), v.y.raw(), v.z.raw()]
}

/// Per-component `f` of two triples.
fn zip3<A, B, C>(a: [A; 3], b: [B; 3], f: impl Fn(A, B) -> C) -> [C; 3] {
    let ([a0, a1, a2], [b0, b1, b2]) = (a, b);
    [f(a0, b0), f(a1, b1), f(a2, b2)]
}

/// The three exact products of a row with a point.
fn products(row: [i64; 3], point: [i64; 3]) -> [i128; 3] {
    zip3(row, point, |entry, coordinate| i128::from(entry) * i128::from(coordinate))
}

/// The exact image of `p`, one [`Exact`] per axis.
fn image(transform: &FixedAffine3, p: [i64; 3]) -> [Exact; 3] {
    let (m, t) = (rows(transform.linear), raw3(transform.translation));
    zip3(m, t, |row, shift| Exact::sum(shift, products(row, p)))
}

/// A box from `i128` faces, or [`BoundsError::OutOfRange`] when one is outside `i64`.
fn narrow(lo: [i128; 3], hi: [i128; 3]) -> Result<FixedAabb3, BoundsError> {
    let face = |v: i128| i64::try_from(v).map(FixedQ32_32::from_raw);
    let vec = |v: [i128; 3]| -> Result<FixedVec3, BoundsError> {
        let [x, y, z] = v.map(face);
        match (x, y, z) {
            (Ok(x), Ok(y), Ok(z)) => Ok(FixedVec3::new(x, y, z)),
            _ => Err(BoundsError::OutOfRange),
        }
    };
    Ok(FixedAabb3::new(vec(lo)?, vec(hi)?))
}

impl FixedAabb3 {
    /// The smallest raw box that contains the exact image of this box under `transform`.
    ///
    /// On each axis `i` the exact image's lowest coordinate is `t_i + sum_j min(m_ij lo_j,
    /// m_ij hi_j) / 2^32` and its highest the same with `max`, the extremes of an affine map
    /// over a box. The minimum face is the floor of the exact lowest coordinate and the maximum
    /// face the ceiling of the exact highest, so each face is less than 1 raw outside the exact
    /// image's axis-aligned hull, and on it when that face is a whole raw value. Every point of
    /// the box maps, exactly, inside the result. The rounded
    /// [`FixedAffine3::checked_transform_point`] of a point can be up to 3/2 raw from its exact
    /// image, so it can lie outside; [`FixedAabb3::checked_transform`] is the hull of the
    /// eight rounded corners instead.
    ///
    /// # Errors
    ///
    /// [`BoundsError::InvertedBox`] when a minimum is above its maximum, before any arithmetic;
    /// [`BoundsError::OutOfRange`] exactly when a face is outside `i64`, that is when the
    /// exact image leaves the raw range. No intermediate is range-checked, so a box whose
    /// corners [`FixedAffine3::checked_transform_point`] refuses can still be bounded.
    pub fn transformed_bounds(&self, transform: &FixedAffine3) -> Result<Self, BoundsError> {
        let (lo, hi) = (raw3(self.min), raw3(self.max));
        if lo.iter().zip(&hi).any(|(l, h)| l > h) {
            return Err(BoundsError::InvertedBox);
        }
        let (m, t) = (rows(transform.linear), raw3(transform.translation));
        let face = |row: [i64; 3], shift: i64, pick: fn(i128, i128) -> i128| {
            Exact::sum(shift, zip3(products(row, lo), products(row, hi), pick))
        };
        let low = zip3(m, t, |row, shift| face(row, shift, Ord::min).floor);
        let high = zip3(m, t, |row, shift| face(row, shift, Ord::max).ceil());
        narrow(low, high)
    }

    /// The smallest raw box that contains the exact image of every point under `transform`:
    /// on each axis the floor of the least exact image coordinate and the ceiling of the
    /// greatest. A mesh is bounded by passing its vertex slice. The slice is read once and
    /// nothing allocates.
    ///
    /// # Errors
    ///
    /// [`BoundsError::NoPoints`] for an empty slice; [`BoundsError::OutOfRange`] exactly when a
    /// face is outside `i64`.
    pub fn from_transformed_points(
        points: &[FixedVec3],
        transform: &FixedAffine3,
    ) -> Result<Self, BoundsError> {
        let (first, rest) = points.split_first().ok_or(BoundsError::NoPoints)?;
        let faces = |p: &FixedVec3| {
            let e = image(transform, raw3(*p));
            (e.map(|c| c.floor), e.map(Exact::ceil))
        };
        let (low, high) = rest.iter().map(faces).fold(faces(first), |(lo, hi), (l, h)| {
            (zip3(lo, l, i128::min), zip3(hi, h, i128::max))
        });
        narrow(low, high)
    }

    /// The smallest raw box that contains the exact image of the triangle under `transform`,
    /// [`FixedAabb3::from_transformed_points`] of its three vertices. Degenerate triangles are
    /// accepted.
    ///
    /// # Errors
    ///
    /// [`BoundsError::OutOfRange`] exactly when a face is outside `i64`.
    pub fn from_transformed_triangle(
        vertices: [FixedVec3; 3],
        transform: &FixedAffine3,
    ) -> Result<Self, BoundsError> {
        Self::from_transformed_points(&vertices, transform)
    }

    /// The smallest raw box that contains the exact image of the sphere under `transform`.
    ///
    /// The image is an ellipsoid `c' + L u` with `|u| <= r`, whose extent on axis `i` is `r`
    /// times the length of row `i` of `L`. In raw units let `C = 2^32 c'_i` (exact) and
    /// `X = R^2 (m_i0^2 + m_i1^2 + m_i2^2)`. The minimum face is the greatest integer `F` with
    /// `C - 2^32 F >= 0` and `(C - 2^32 F)^2 >= X`; the maximum face is the least `G` with
    /// `2^32 G - C >= 0` and `(2^32 G - C)^2 >= X`. Those are the floor and the ceiling of the
    /// exact extremes, decided by 256-bit comparisons with no square root rounded, so each face
    /// is less than 1 raw outside the exact image's hull. A radius of zero gives the bounds of
    /// the exact image of the centre; a singular `L` is accepted (the image is flat).
    ///
    /// # Errors
    ///
    /// [`BoundsError::NegativeRadius`] for a negative radius, first;
    /// [`BoundsError::OutOfRange`] exactly when a face is outside `i64`.
    pub fn from_transformed_sphere(
        sphere: &FixedSphere3,
        transform: &FixedAffine3,
    ) -> Result<Self, BoundsError> {
        let radius = u64::try_from(sphere.radius.raw()).map_err(|_| BoundsError::NegativeRadius)?;
        let centre = image(transform, raw3(sphere.center));
        let reach = rows(transform.linear).map(|row| wide::Reach::new(radius, row));
        let low =
            zip3(centre, reach, |c, r: wide::Reach| c.floor.saturating_sub(r.below(c.fraction)));
        let high =
            zip3(centre, reach, |c, r: wide::Reach| c.floor.saturating_add(r.above(c.fraction)));
        narrow(low, high)
    }
}
