//! Exact ray–disc test, contract `bunny-ray-disc/1`.
//!
//! The disc lies in the plane `z = height`, centred on the `z` axis at `(0, 0, height)`.
//! With `f = origin.z - height` and `d` the direction, all raw, a ray not parallel to the
//! plane (`d.z != 0`) meets it at the distance `t = -f / d.z`. The hit point is inside the
//! disc when `(o.x d.z - d.x f)^2 + (o.y d.z - d.y f)^2 <= r^2 d.z^2`, which is the radius
//! test on the exact hit point multiplied through by `d.z^2`; it is decided exactly in 256
//! bits. See `docs/topics/ray-tracing/README.md#ray-disc`.

use super::wide::U256;
use super::{in_trace_domain, vec3, wide3, TraceRay};
use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use std::fmt;

/// A disc prepared for [`TraceRay::hit_disc`]: the circle of a positive radius about the
/// point `(0, 0, height)` in the plane `z = height`, both in the trace domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraceDisc {
    height: i64,
    radius: i64,
}

/// Why [`TraceDisc::new`] refused a disc.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraceDiscError {
    /// The height is outside the trace domain ([`crate::TRACE_COORD_LIMIT_RAW`]).
    HeightOutsideTraceDomain,
    /// The radius is zero or negative.
    RadiusNotPositive,
    /// The radius is outside the trace domain ([`crate::TRACE_COORD_LIMIT_RAW`]).
    RadiusOutsideTraceDomain,
}

impl fmt::Display for TraceDiscError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::HeightOutsideTraceDomain => "disc height is outside the trace domain",
            Self::RadiusNotPositive => "disc radius is not positive",
            Self::RadiusOutsideTraceDomain => "disc radius is outside the trace domain",
        })
    }
}

impl std::error::Error for TraceDiscError {}

impl TraceDisc {
    /// Prepares the disc of `radius` about `(0, 0, height)` in the plane `z = height`.
    ///
    /// # Errors
    ///
    /// Returns [`TraceDiscError`] when the height or the radius is outside the trace domain
    /// ([`crate::TRACE_COORD_LIMIT_RAW`]) or the radius is not positive.
    pub fn new(height: FixedQ32_32, radius: FixedQ32_32) -> Result<Self, TraceDiscError> {
        let (height, radius) = (height.raw(), radius.raw());
        if !in_trace_domain(height) {
            Err(TraceDiscError::HeightOutsideTraceDomain)
        } else if radius <= 0 {
            Err(TraceDiscError::RadiusNotPositive)
        } else if !in_trace_domain(radius) {
            Err(TraceDiscError::RadiusOutsideTraceDomain)
        } else {
            Ok(Self { height, radius })
        }
    }

    /// The height of the plane of the disc (its `z`).
    #[must_use]
    pub const fn height(&self) -> FixedQ32_32 {
        FixedQ32_32::from_raw(self.height)
    }

    /// The radius.
    #[must_use]
    pub const fn radius(&self) -> FixedQ32_32 {
        FixedQ32_32::from_raw(self.radius)
    }

    /// The smallest box that contains the disc: flat, at `z = height`.
    #[must_use]
    pub fn bounds(&self) -> FixedAabb3 {
        // Each bound is below 2^40 in magnitude, so it fits.
        let (r, h) = (self.radius, self.height);
        FixedAabb3::new(vec3([-r, -r, h]), vec3([r, r, h]))
    }
}

/// Where a ray hit a disc.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiscHit {
    /// Distance along the ray, in multiples of the ray direction, rounded down.
    pub distance: FixedQ32_32,
    /// Whether the ray hit the `+z` side of the disc: true when the direction points
    /// toward `-z`, false when it points toward `+z`.
    pub front_face: bool,
}

impl DiscHit {
    /// The normal of the `+z` side, `(0, 0, 1)`, whichever side was hit; `front_face` says
    /// which side that was.
    #[must_use]
    pub const fn normal(&self) -> FixedVec3 {
        vec3([0, 0, 1 << FixedQ32_32::FRACTIONAL_BITS])
    }
}

impl TraceRay {
    /// Exact ray–disc test over distances `t_min..=t_max`, contract `bunny-ray-disc/1`.
    ///
    /// The ray hits when it is not parallel to the plane of the disc, the exact point where
    /// it meets the plane is within the radius (the rim counts as inside) and the rounded
    /// distance is in range. A ray parallel to the plane misses, including one that lies in
    /// it. The distance is the floor of the true distance in raw units.
    #[must_use]
    pub fn hit_disc(
        &self,
        disc: &TraceDisc,
        t_min: FixedQ32_32,
        t_max: FixedQ32_32,
    ) -> Option<DiscHit> {
        let dz = i128::from(self.direction[2]);
        let f = i128::from(self.origin[2]) - i128::from(disc.height);
        let t = plane_distance(f, dz)?;
        if !(i128::from(t_min.raw())..=i128::from(t_max.raw())).contains(&t) {
            return None;
        }
        if !self.meets_within(disc.radius, f)? {
            return None;
        }
        // In range, so it fits `i64`.
        let distance = i64::try_from(t).ok()?;
        Some(DiscHit { distance: FixedQ32_32::from_raw(distance), front_face: dz < 0 })
    }

    /// Whether the exact point where the ray meets the plane, `f` raw below the origin, is
    /// at most `radius` from the axis; `None` when a sum does not fit 256 bits (never in the
    /// trace domain).
    ///
    /// Each numerator is below `2^75` and `radius * d.z` below `2^73`, so the squares are
    /// below `2^150`.
    fn meets_within(&self, radius: i64, f: i128) -> Option<bool> {
        let [ox, oy, _] = wide3(self.origin);
        let [dx, dy, dz] = wide3(self.direction);
        let across = |o: i128, d: i128| (o * dz - d * f).unsigned_abs();
        let (x, y) = (across(ox, dx), across(oy, dy));
        let reach = u128::from(radius.unsigned_abs()) * dz.unsigned_abs();
        let outside = U256::product(x, x).checked_add(U256::product(y, y))?;
        Some(outside <= U256::product(reach, reach))
    }
}

/// `floor(-f * 2^32 / dz)`, the raw distance at which a ray `f` raw above the plane with
/// direction `z` of `dz` meets it, or `None` when the ray is parallel to the plane.
///
/// `|f * 2^32| < 2^73` fits `i128`; the divisor is made positive so that `div_euclid` floors.
fn plane_distance(f: i128, dz: i128) -> Option<i128> {
    if dz == 0 {
        return None;
    }
    let sign = dz.signum();
    Some(((-f * sign) << FixedQ32_32::FRACTIONAL_BITS).div_euclid(dz * sign))
}
