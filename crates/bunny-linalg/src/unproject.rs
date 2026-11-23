//! Unprojection `bunny-unproject/1`: a viewport coordinate, a `clip_from_view` matrix and a
//! `view_from_world` matrix to the world-space ray through that coordinate.
//!
//! The ray runs from the point the coordinate unprojects to at NDC depth `0` (the near plane)
//! toward the point at depth `1` (the far plane). Every step is inverted exactly, in wide
//! integers, and each output component is rounded once. See
//! `docs/topics/coordinate-law/README.md#unprojection`.

mod frustum;
mod wide;

use std::fmt;

use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;

use crate::{FixedMat4, FixedVec2, FixedVec3, Viewport};
use frustum::{Frustum, ViewRay};
use wide::Wide;

/// The versioned name of [`Viewport::try_unproject`]. Any change to its output is a new
/// version.
pub const UNPROJECT_CONTRACT_ID: &str = "bunny-unproject/1";

/// A world-space ray from [`Viewport::try_unproject`].
///
/// `origin` is the point at NDC depth `0` and `origin + direction` the point at depth `1`, so
/// the parameter `t` in `[0, 1]` spans the projection's depth range. The direction is not
/// normalized.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ViewportRay {
    /// The world point at NDC depth `0`: on the near plane.
    pub origin: FixedVec3,
    /// The world point at NDC depth `1` minus `origin`.
    pub direction: FixedVec3,
}

/// Why [`Viewport::try_unproject`] refused, in the order the checks run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnprojectError {
    /// A coordinate is below zero or above the viewport size: `x` outside `[0, width]` or `y`
    /// outside `[0, height]`, in raw units `[0, size * 2^32]`.
    OutsideViewport,
    /// `clip_from_view` has neither shape the projection builders produce.
    UnsupportedProjection,
    /// `clip_from_view` is singular: `m00` or `m11` is zero, or `m23` of a perspective or
    /// `m22` of an orthographic matrix.
    SingularProjection,
    /// A perspective point's homogeneous `w` is zero: `m22` is `0` (depth `0`) or `-1.0`
    /// (depth `1`), so that point is at infinity.
    PointAtInfinity,
    /// A perspective point's homogeneous `w` is negative: the point at depth `0` or `1` is
    /// behind the camera.
    PointBehindCamera,
    /// `view_from_world` is not affine: its last row is not `(0, 0, 0, 1)`.
    UnsupportedView,
    /// The linear part of `view_from_world` has determinant zero.
    SingularView,
    /// A rounded origin or direction component is outside the Q32.32 range.
    Overflow,
    /// The rounded direction is the zero vector.
    DegenerateRay,
}

impl fmt::Display for UnprojectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::OutsideViewport => "viewport coordinate is outside the viewport",
            Self::UnsupportedProjection => "projection matrix has an unsupported shape",
            Self::SingularProjection => "projection matrix is singular",
            Self::PointAtInfinity => "unprojected point is at infinity",
            Self::PointBehindCamera => "unprojected point is behind the camera",
            Self::UnsupportedView => "view matrix is not affine",
            Self::SingularView => "view matrix is singular",
            Self::Overflow => "unprojected ray is outside the Q32.32 range",
            Self::DegenerateRay => "unprojected ray direction rounds to zero",
        })
    }
}

impl std::error::Error for UnprojectError {}

impl Viewport {
    /// The world-space ray through the continuous viewport coordinate `coordinate`
    /// (`bunny-unproject/1`).
    ///
    /// `coordinate` is in this viewport's convention: origin at the top-left corner, `y` down,
    /// integers on pixel corners, so pixel `(i, j)` has its centre at `(i + 1/2, j + 1/2)`.
    /// The closed rectangle `[0, width] x [0, height]` is accepted, edges included.
    ///
    /// The coordinate goes back through the exact inverse of [`Self::try_ndc_to_viewport`]'s
    /// formula before its floor (`x_ndc = 2 x / width - 1`, `y_ndc = 1 - 2 y / height`), then
    /// the exact inverse of `clip_from_view` at NDC depths `0` and `1`, perspective divide
    /// included, then the exact inverse of `view_from_world`. `clip_from_view` must have the
    /// shape [`FixedMat4::try_perspective`] or [`FixedMat4::try_orthographic`] produces (any
    /// entry values); `view_from_world` must be affine. No step rounds: each of the six output
    /// components is the exact value rounded once to nearest, ties to even, so it is within
    /// half a raw unit.
    ///
    /// # Errors
    /// The [`UnprojectError`] of the first check that fails, in the order the variants are
    /// listed.
    pub fn try_unproject(
        self,
        coordinate: FixedVec2,
        clip_from_view: FixedMat4,
        view_from_world: FixedMat4,
    ) -> Result<ViewportRay, UnprojectError> {
        let ndc = self.ndc_numerators(coordinate)?;
        let frustum = Frustum::classify(clip_from_view)?;
        let view = InverseView::new(view_from_world)?;
        view.world_ray(&frustum.view_ray(ndc).ok_or(UnprojectError::Overflow)?)
    }

    /// The exact NDC as `(2 x - width ONE) / (width ONE)` and `(height ONE - 2 y) / (height ONE)`:
    /// the two numerators and the sizes.
    fn ndc_numerators(self, coordinate: FixedVec2) -> Result<[i128; 4], UnprojectError> {
        let one = i128::from(ONE_RAW);
        let (x, y) = (i128::from(coordinate.x.raw()), i128::from(coordinate.y.raw()));
        let (width, height) = (i128::from(self.width()), i128::from(self.height()));
        if x < 0 || y < 0 || x > width * one || y > height * one {
            return Err(UnprojectError::OutsideViewport);
        }
        Ok([2 * x - width * one, height * one - 2 * y, width, height])
    }
}

/// The adjugate, determinant and translation of an affine `view_from_world`, raw.
struct InverseView {
    adjugate: [[Wide; 3]; 3],
    determinant: Wide,
    translation: [Wide; 3],
}

impl InverseView {
    fn new(m: FixedMat4) -> Result<Self, UnprojectError> {
        let last = m.row3();
        if [last.0.raw(), last.1.raw(), last.2.raw(), last.3.raw()] != [0, 0, 0, ONE_RAW] {
            return Err(UnprojectError::UnsupportedView);
        }
        let rows = [m.row0(), m.row1(), m.row2()];
        let a = rows.map(|row| [row.0, row.1, row.2].map(|v| Wide::from_i128(v.raw().into())));
        let translation = rows.map(|row| Wide::from_i128(row.3.raw().into()));
        let (adjugate, determinant) = adjugate(&a).ok_or(UnprojectError::Overflow)?;
        if determinant.is_zero() {
            return Err(UnprojectError::SingularView);
        }
        Ok(Self { adjugate, determinant, translation })
    }

    /// `origin_i = 2^32 sum_j adj_ij (2^32 N_j - t_j E) / (E det)` and
    /// `direction_i = 2^64 sum_j adj_ij D_j / (F det)`, each rounded once.
    fn world_ray(&self, ray: &ViewRay) -> Result<ViewportRay, UnprojectError> {
        let one = Wide::from_i128(ONE_RAW.into());
        let shifted = |n: &Wide, t: &Wide| one.checked_mul(*n)?.checked_sub(t.checked_mul(ray.e)?);
        let [n0, n1, n2] = &ray.origin;
        let [t0, t1, t2] = &self.translation;
        let terms = [shifted(n0, t0), shifted(n1, t1), shifted(n2, t2)];
        let origin = self.solve(terms, Some(one), ray.e)?;
        let direction = self.solve(ray.direction.map(Some), one.checked_mul(one), ray.f)?;
        if [direction.x, direction.y, direction.z].iter().all(|c| c.raw() == 0) {
            return Err(UnprojectError::DegenerateRay);
        }
        Ok(ViewportRay { origin, direction })
    }

    /// `round(scale sum_j adj_ij terms_j / (denominator det))` for each row `i`.
    fn solve(
        &self,
        terms: [Option<Wide>; 3],
        scale: Option<Wide>,
        denominator: Wide,
    ) -> Result<FixedVec3, UnprojectError> {
        let divisor = denominator.checked_mul(self.determinant);
        let row = |adj: &[Wide; 3]| -> Option<FixedQ32_32> {
            let mut sum = Wide::ZERO;
            for (entry, term) in adj.iter().zip(terms) {
                sum = sum.checked_add(entry.checked_mul(term?)?)?;
            }
            let raw = sum.checked_mul(scale?)?.round_div(divisor?)?;
            Some(FixedQ32_32::from_raw(raw))
        };
        let [x, y, z] = self.adjugate.each_ref().map(row);
        let overflow = UnprojectError::Overflow;
        Ok(FixedVec3::new(x.ok_or(overflow)?, y.ok_or(overflow)?, z.ok_or(overflow)?))
    }
}

/// The adjugate (`adj_ij` the cofactor of `a_ji`) and the determinant of a raw 3 x 3 matrix.
fn adjugate(a: &[[Wide; 3]; 3]) -> Option<([[Wide; 3]; 3], Wide)> {
    let [[a00, a01, a02], [a10, a11, a12], [a20, a21, a22]] = *a;
    let adj = [
        [minor(a11, a22, a12, a21)?, minor(a02, a21, a01, a22)?, minor(a01, a12, a02, a11)?],
        [minor(a12, a20, a10, a22)?, minor(a00, a22, a02, a20)?, minor(a02, a10, a00, a12)?],
        [minor(a10, a21, a11, a20)?, minor(a01, a20, a00, a21)?, minor(a00, a11, a01, a10)?],
    ];
    let [[c00, _, _], [c10, _, _], [c20, _, _]] = adj;
    let first = a00.checked_mul(c00)?.checked_add(a01.checked_mul(c10)?)?;
    Some((adj, first.checked_add(a02.checked_mul(c20)?)?))
}

/// `a b - c d`, exactly.
fn minor(a: Wide, b: Wide, c: Wide, d: Wide) -> Option<Wide> {
    a.checked_mul(b)?.checked_sub(c.checked_mul(d)?)
}
