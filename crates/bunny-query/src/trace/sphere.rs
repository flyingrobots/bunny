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
use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::FixedQ32_32;
use core::fmt;

/// The contract identity of [`TraceRay::hit_sphere`].
pub const RAY_SPHERE_ID: &str = "bunny-ray-sphere/1";

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

impl core::error::Error for TraceSphereError {}

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
    normal: FixedUnitVec3,
}

impl SphereHit {
    /// The outward unit normal at the exact hit point: at the true, unrounded root, not at
    /// the rounded distance. It is normalized by Bunny's unit-vector rule (the prescale to
    /// `[2^61, 2^62)` and `FixedUnitVec3::new`), so it is never zero, and each component is
    /// within `1/2 + 2^-24` raw units of the exact unit normal for every admitted radius
    /// (`docs/topics/ray-tracing/README.md#ray-sphere`).
    #[must_use]
    pub const fn normal(&self) -> FixedUnitVec3 {
        self.normal
    }
}

impl TraceRay {
    /// Exact ray–sphere test over distances `t_min..=t_max`, contract `bunny-ray-sphere/1`
    /// ([`RAY_SPHERE_ID`]).
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
        let line = Line::new(offset, wide3(self.direction), sphere.radius)?;
        let (near, far) = line.roots()?;
        let range = i128::from(t_min.raw())..=i128::from(t_max.raw());
        let (t, front_face) = if range.contains(&near) {
            (near, true)
        } else if range.contains(&far) {
            (far, false)
        } else {
            return None;
        };
        let distance = FixedQ32_32::from_raw(i64::try_from(t).ok()?);
        let normal = line.unit_normal(front_face)?;
        Some(SphereHit { distance, front_face, normal })
    }
}

/// A ray's line against a sphere it meets: `a = d.d`, `b = f.d`, the radius and the exact
/// quarter discriminant `a r^2 - |f x d|^2 >= 0`, with `f = origin - center`.
///
/// Bounds in the trace domain: `|f| < 2^41` and `|d| <= 2^32 + 1` per component, so
/// `a < 2^66`, `|b| < 2^75`, each `|f x d|` component is below `2^75` and the discriminant is
/// at most `a r^2 < 2^146`.
struct Line {
    offset: Wide3,
    direction: Wide3,
    a: i128,
    b: i128,
    radius: i64,
    discriminant: U256,
}

impl Line {
    /// The line's quadratic, or `None` when it misses the sphere.
    fn new(offset: Wide3, direction: Wide3, radius: i64) -> Option<Self> {
        let perpendicular = cross(offset, direction)
            .iter()
            .try_fold(U256::product(0, 0), |sum, c| sum.checked_add(square(*c)))?;
        let a = dot(direction, direction);
        let radius_squared = u128::from(radius.unsigned_abs()).pow(2);
        let discriminant =
            U256::product(a.unsigned_abs(), radius_squared).checked_sub(perpendicular)?;
        let b = dot(offset, direction);
        Some(Self { offset, direction, a, b, radius, discriminant })
    }

    /// The floors of the two roots in raw distance units, near first.
    ///
    /// `E = 2^64 * discriminant` is below `2^210` and its root `g` below `2^105`; `2^32 b`
    /// and `2^32 b +- g` are below `2^108`, so all fit `i128`.
    fn roots(&self) -> Option<(i128, i128)> {
        let scaled = self.discriminant.checked_shl(2 * FixedQ32_32::FRACTIONAL_BITS)?;
        let root = scaled.isqrt();
        let exact = U256::product(root, root) == scaled;
        let root = i128::try_from(root).ok()?;
        let scaled_b = self.b << FixedQ32_32::FRACTIONAL_BITS;
        // `(-2^32 b - sqrt E) / a` lies strictly below `-2^32 b - root` unless `E` is a square.
        let near = (-scaled_b - root - i128::from(!exact)).div_euclid(self.a);
        let far = (-scaled_b + root).div_euclid(self.a);
        Some((near, far))
    }

    /// The outward unit normal at the exact near (`true`) or far root.
    ///
    /// At the root `t = (-b -+ sqrt(disc)) / a` the offset from the centre is `f + d t`, and
    /// `a (f + d t) = w -+ d sqrt(disc)` with `w = a f - b d`, a vector of length `a r`. With
    /// `k = 124 - bits(a r)` (so `2^k a r` is in `[2^123, 2^124)`) the kernel forms
    /// `v = 2^k w -+ d g` with `g = floor(2^k sqrt(disc))`: `|2^k w|` and `|d g|` are each at
    /// most `2^k a r < 2^124`, so `v` fits `i128`; `2^2k disc <= (2^k a r)^2 / a < 2^248`
    /// fits `U256`. `v` is off `2^k a (f + d t)` by less than `|d_i| <= 2^32 + 1` per
    /// component and is at least `2^123 - 2^34` long, so it is never zero.
    fn unit_normal(&self, near: bool) -> Option<FixedUnitVec3> {
        let scale = self.a.unsigned_abs() * u128::from(self.radius.unsigned_abs());
        let shift = 124_u32.checked_sub(128 - scale.leading_zeros())?;
        let root = self.discriminant.checked_shl(2 * shift)?.isqrt();
        let root = i128::try_from(root).ok()?;
        let along = if near { -root } else { root };
        let (a, b) = (self.a, self.b);
        let mut v = [0_i128; 3];
        for ((out, f), d) in v.iter_mut().zip(self.offset).zip(self.direction) {
            *out = ((a * f - b * d) << shift) + d * along;
        }
        unit(v)
    }
}

/// The floors of the two roots in raw distance units, near first, or `None` when the line
/// misses the sphere. The ray–cylinder kernel uses it for its side in the plane across the
/// axis.
pub(super) fn roots(offset: Wide3, direction: Wide3, radius: i64) -> Option<(i128, i128)> {
    Line::new(offset, direction, radius)?.roots()
}

/// The exact square of an `i128` below `2^128` in magnitude.
fn square(value: i128) -> U256 {
    U256::product(value.unsigned_abs(), value.unsigned_abs())
}

/// Bunny's unit-vector rule for a long vector, as the triangle surfaces apply it: shift right
/// (flooring) until the largest component magnitude is in `[2^61, 2^62)`, then
/// `FixedUnitVec3::new`. Every vector given here has a component of at least `2^122`.
fn unit(v: Wide3) -> Option<FixedUnitVec3> {
    let top = v.iter().map(|c| c.unsigned_abs()).max()?;
    let shift = top.checked_ilog2()?.checked_sub(61)?;
    let mut raw = [0_i64; 3];
    for (out, c) in raw.iter_mut().zip(v) {
        *out = i64::try_from(c >> shift).ok()?;
    }
    FixedUnitVec3::new(vec3(raw))
}
