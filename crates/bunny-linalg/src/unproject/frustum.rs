//! The exact inverse of a `clip_from_view` matrix of the builders' shapes, at NDC depths `0`
//! and `1`, as ratios of wide integers.

use bunny_num::fixed_q32_32::ONE_RAW;

use super::wide::Wide;

const NEG_ONE_RAW: i64 = -ONE_RAW;
use super::UnprojectError;
use crate::FixedMat4;

/// The two shapes [`FixedMat4::try_perspective`] and [`FixedMat4::try_orthographic`] produce.
#[derive(Clone, Copy)]
enum Shape {
    /// Last row `(0, 0, -1, 0)`.
    Perspective,
    /// Last row `(0, 0, 0, 1)`.
    Orthographic,
}

/// A supported `clip_from_view`: its shape and its raw entries `m00`, `m11`, `m22`, `m23`.
pub(super) struct Frustum {
    shape: Shape,
    p: i64,
    q: i64,
    r: i64,
    s: i64,
}

/// A view-space ray as exact ratios: the depth-0 point is `origin / e` and the depth-1 point
/// minus it is `direction / f`, all in units.
pub(super) struct ViewRay {
    pub(super) origin: [Wide; 3],
    pub(super) e: Wide,
    pub(super) direction: [Wide; 3],
    pub(super) f: Wide,
}

// The short names are those of the chapter's formulas (`p, q, r, s` for `m00, m11, m22, m23`).
#[allow(clippy::many_single_char_names)]
impl Frustum {
    /// Accepts the builders' shapes with any entry values, refusing a singular matrix and, for
    /// a perspective matrix, a depth-0 or depth-1 point at infinity or behind the camera.
    pub(super) fn classify(m: FixedMat4) -> Result<Self, UnprojectError> {
        let off = [m.m01(), m.m02(), m.m03(), m.m10(), m.m12(), m.m13(), m.m20(), m.m21()];
        if off.iter().chain(&[m.m30(), m.m31()]).any(|v| v.raw() != 0) {
            return Err(UnprojectError::UnsupportedProjection);
        }
        let shape = match (m.m32().raw(), m.m33().raw()) {
            (NEG_ONE_RAW, 0) => Shape::Perspective,
            (0, ONE_RAW) => Shape::Orthographic,
            _ => return Err(UnprojectError::UnsupportedProjection),
        };
        let (p, q, r, s) = (m.m00().raw(), m.m11().raw(), m.m22().raw(), m.m23().raw());
        let frustum = Self { shape, p, q, r, s };
        frustum.check()?;
        Ok(frustum)
    }

    fn check(&self) -> Result<(), UnprojectError> {
        let singular = match self.shape {
            Shape::Perspective => self.s == 0,
            Shape::Orthographic => self.r == 0,
        };
        if self.p == 0 || self.q == 0 || singular {
            return Err(UnprojectError::SingularProjection);
        }
        match self.shape {
            Shape::Perspective => homogeneous_w_check(self.r, self.s),
            Shape::Orthographic => Ok(()),
        }
    }

    /// The exact view-space ray of the NDC numerators `[ax, ay, width, height]`, where
    /// `x_ndc = ax / (width ONE)` and `y_ndc = ay / (height ONE)`.
    ///
    /// With `E = width height m00 m11 m22`, the depth-0 point is `N / E` with
    /// `N = (ax s height q, ay s width p, -s width height p q)` for a perspective matrix
    /// (`ax height q r` and `ay width p r` in `x` and `y` for an orthographic one). The
    /// perspective depth-1 point is the depth-0 point times `r / (r + ONE)`, so the direction
    /// is `-ONE N / (E (r + ONE))`; the orthographic direction is `(0, 0, ONE / r)`.
    pub(super) fn view_ray(&self, ndc: [i128; 4]) -> Option<ViewRay> {
        let [ax, ay, width, height] = ndc.map(Wide::from_i128);
        let [p, q, r, s] = [self.p, self.q, self.r, self.s].map(|v| Wide::from_i128(v.into()));
        let (wp, hq) = (width.checked_mul(p)?, height.checked_mul(q)?);
        let e = wp.checked_mul(hq)?.checked_mul(r)?;
        let z = s.checked_mul(wp)?.checked_mul(hq)?.negated();
        match self.shape {
            Shape::Perspective => {
                let x = ax.checked_mul(s)?.checked_mul(hq)?;
                let y = ay.checked_mul(s)?.checked_mul(wp)?;
                perspective_ray([x, y, z], e, r)
            }
            Shape::Orthographic => {
                let x = ax.checked_mul(hq)?.checked_mul(r)?;
                let y = ay.checked_mul(wp)?.checked_mul(r)?;
                let direction = [Wide::ZERO, Wide::ZERO, one()];
                Some(ViewRay { origin: [x, y, z], e, direction, f: r })
            }
        }
    }
}

/// The perspective homogeneous `w` at depth `z` is `(z + m22) / m23`: `m22 / m23` at depth 0
/// and `(1.0 + m22) / m23` at depth 1. Each must be positive.
fn homogeneous_w_check(m22: i64, m23: i64) -> Result<(), UnprojectError> {
    let (near, far) = (i128::from(m22), i128::from(m22) + i128::from(ONE_RAW));
    if near == 0 || far == 0 {
        return Err(UnprojectError::PointAtInfinity);
    }
    let sign = i128::from(m23).signum();
    if near.signum() != sign || far.signum() != sign {
        return Err(UnprojectError::PointBehindCamera);
    }
    Ok(())
}

fn one() -> Wide {
    Wide::from_i128(ONE_RAW.into())
}

/// The depth-1 point is the depth-0 point `origin / e` times `r / (r + ONE)`, so the direction
/// is `-ONE origin / (e (r + ONE))`.
#[allow(clippy::many_single_char_names)]
fn perspective_ray(origin: [Wide; 3], e: Wide, r: Wide) -> Option<ViewRay> {
    let scale = one().negated();
    let [x, y, z] = origin;
    let direction = [x.checked_mul(scale)?, y.checked_mul(scale)?, z.checked_mul(scale)?];
    let f = e.checked_mul(r.checked_add(one())?)?;
    Some(ViewRay { origin, e, direction, f })
}
