//! Exact ray–sphere test, contract `bunny-ray-sphere/1`.
//!
//! With `f = origin - center`, `d` the direction and `r` the radius, all raw, the ray meets
//! the sphere where `a t^2 + 2 b t + (f.f - r^2) = 0`, with `a = d.d` and `b = f.d`. Its
//! quarter discriminant is `a r^2 - |f x d|^2` (Lagrange's identity), which is computed
//! exactly in 256 bits; the roots are then floors of exact quotients. See
//! `docs/topics/ray-tracing/README.md#ray-sphere`.

use super::wide::U256;
use super::{cross, dot, in_trace_domain, raw3, sub3, vec3, wide3, TraceRay, Wide3};
use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use std::fmt;

/// A sphere prepared for [`TraceRay::hit_sphere`]: a centre and a positive radius, both in
/// the trace domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraceSphere {
    center: [i64; 3],
    radius: i64,
}

/// Why [`TraceSphere::new`] refused a sphere.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraceSphereError {
    /// A centre coordinate is outside the trace domain ([`crate::TRACE_COORD_LIMIT_RAW`]).
    CenterOutsideTraceDomain,
    /// The radius is zero or negative.
    RadiusNotPositive,
    /// The radius is outside the trace domain ([`crate::TRACE_COORD_LIMIT_RAW`]).
    RadiusOutsideTraceDomain,
}

impl fmt::Display for TraceSphereError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::CenterOutsideTraceDomain => "sphere centre is outside the trace domain",
            Self::RadiusNotPositive => "sphere radius is not positive",
            Self::RadiusOutsideTraceDomain => "sphere radius is outside the trace domain",
        })
    }
}

impl std::error::Error for TraceSphereError {}

impl TraceSphere {
    /// Prepares a sphere.
    ///
    /// # Errors
    ///
    /// Returns [`TraceSphereError`] when a centre coordinate or the radius is outside the
    /// trace domain ([`crate::TRACE_COORD_LIMIT_RAW`]) or the radius is not positive.
    pub fn new(center: FixedVec3, radius: FixedQ32_32) -> Result<Self, TraceSphereError> {
        let center = raw3(center);
        let radius = radius.raw();
        if !center.iter().all(|&c| in_trace_domain(c)) {
            Err(TraceSphereError::CenterOutsideTraceDomain)
        } else if radius <= 0 {
            Err(TraceSphereError::RadiusNotPositive)
        } else if !in_trace_domain(radius) {
            Err(TraceSphereError::RadiusOutsideTraceDomain)
        } else {
            Ok(Self { center, radius })
        }
    }

    /// The centre.
    #[must_use]
    pub const fn center(&self) -> FixedVec3 {
        vec3(self.center)
    }

    /// The radius.
    #[must_use]
    pub const fn radius(&self) -> FixedQ32_32 {
        FixedQ32_32::from_raw(self.radius)
    }

    /// The smallest box that contains the sphere.
    #[must_use]
    pub fn bounds(&self) -> FixedAabb3 {
        // Each bound is below 2^41 in magnitude, so it fits.
        let min = self.center.map(|c| c - self.radius);
        let max = self.center.map(|c| c + self.radius);
        FixedAabb3::new(vec3(min), vec3(max))
    }
}

/// Where a ray hit a sphere.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SphereHit {
    /// Distance along the ray, in multiples of the ray direction, rounded down.
    pub distance: FixedQ32_32,
    /// Whether the ray hit the outside of the sphere: true at the near root (the ray enters,
    /// or touches a tangent point), false at the far root (the ray leaves from inside).
    pub front_face: bool,
    normal: FixedVec3,
}

impl SphereHit {
    /// The outward normal at the hit: `(origin + direction * distance - center) / radius`
    /// at the rounded distance, each component rounded down. Its length is within a few raw
    /// units of 1.0, not exactly 1.0.
    #[must_use]
    pub const fn normal(&self) -> FixedVec3 {
        self.normal
    }
}

impl TraceRay {
    /// Exact ray–sphere test over distances `t_min..=t_max`, contract `bunny-ray-sphere/1`.
    ///
    /// Returns the near root when its rounded distance is in range, else the far root when
    /// its rounded distance is, else `None`. A tangent ray hits at its one root, front face.
    /// Each distance is the floor of the true root in raw units.
    #[must_use]
    pub fn hit_sphere(
        &self,
        sphere: &TraceSphere,
        t_min: FixedQ32_32,
        t_max: FixedQ32_32,
    ) -> Option<SphereHit> {
        let offset = sub3(self.origin, sphere.center);
        let (near, far) = roots(offset, wide3(self.direction), sphere.radius)?;
        let range = i128::from(t_min.raw())..=i128::from(t_max.raw());
        let (t, front_face) = if range.contains(&near) {
            (near, true)
        } else if range.contains(&far) {
            (far, false)
        } else {
            return None;
        };
        let distance = i64::try_from(t).ok()?;
        let normal = normal(offset, self.direction, distance, sphere.radius)?;
        Some(SphereHit { distance: FixedQ32_32::from_raw(distance), front_face, normal })
    }
}

/// The floors of the two roots in raw distance units, near first, or `None` when the line
/// misses the sphere.
///
/// Bounds in the trace domain: `|f| < 2^41` and `|d| <= 2^32 + 1` per component, so
/// `a < 2^66`, `|b| < 2^75`, each `|f x d|` component is below `2^75`, `a r^2 < 2^146`, the
/// scaled discriminant `E` is below `2^210` and its root `g` below `2^105`. `2^32 b` and
/// `2^32 b +- g` are below `2^108`; all fit `i128`.
fn roots(offset: Wide3, direction: Wide3, radius: i64) -> Option<(i128, i128)> {
    let a = dot(direction, direction);
    let b = dot(offset, direction);
    let perpendicular = cross(offset, direction)
        .iter()
        .try_fold(U256::product(0, 0), |sum, c| sum.checked_add(square(*c)))?;
    let radius_squared = u128::from(radius.unsigned_abs()).pow(2);
    let scaled_radius = U256::product(a.unsigned_abs(), radius_squared);
    let discriminant = scaled_radius.checked_sub(perpendicular)?.checked_shl64()?;
    let root = discriminant.isqrt();
    let exact = U256::product(root, root) == discriminant;
    let (root, scaled_b) = (i128::try_from(root).ok()?, b << FixedQ32_32::FRACTIONAL_BITS);
    // `(-2^32 b - sqrt E) / a` lies strictly below `-2^32 b - root` unless `E` is a square.
    let near = (-scaled_b - root - i128::from(!exact)).div_euclid(a);
    let far = (-scaled_b + root).div_euclid(a);
    Some((near, far))
}

/// The exact square of an `i128` below `2^128` in magnitude.
fn square(value: i128) -> U256 {
    U256::product(value.unsigned_abs(), value.unsigned_abs())
}

/// `floor((f * 2^32 + d * t) / r)` per component: the raw hit offset from the centre at raw
/// distance `t`, over the radius.
///
/// `|f * 2^32| < 2^73` and `|d * t| < 2^96` fit `i128`. The hit point at the rounded
/// distance is within `|d| / 2^32` raw units of the surface, so each quotient is below
/// `2^35` in magnitude and fits `i64`.
fn normal(offset: Wide3, direction: [i64; 3], t: i64, radius: i64) -> Option<FixedVec3> {
    let mut raw = [0_i64; 3];
    for ((out, f), d) in raw.iter_mut().zip(offset).zip(direction) {
        let numerator = (f << FixedQ32_32::FRACTIONAL_BITS) + i128::from(d) * i128::from(t);
        *out = i64::try_from(numerator.div_euclid(i128::from(radius))).ok()?;
    }
    Some(vec3(raw))
}
