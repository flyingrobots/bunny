//! Exact ray–cylinder test, contract `bunny-ray-cylinder/1`.
//!
//! The cylinder's axis is the object-space `+Y` axis. The solid is the slab
//! `bottom <= y <= top` intersected with the infinite cylinder `x^2 + z^2 <= r^2`; the line
//! is inside the slab between its two cap crossings and inside the infinite cylinder between
//! the roots of the side's quadratic, which are those of the sphere test with the `y`
//! components set to zero. The entry is the later of the two starts and the exit the earlier
//! of the two ends; every comparison is exact. See
//! `docs/topics/ray-tracing/README.md#ray-cylinder`.

use super::{in_trace_domain, vec3, TraceRay};
use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use span::{normal, Line};
use std::fmt;

mod span;

/// A capped cylinder prepared for [`TraceRay::hit_cylinder`]: its axis is the object-space
/// `+Y` axis, with a positive radius and a bottom cap below the top cap, all in the trace
/// domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraceCylinder {
    radius: i64,
    bottom: i64,
    top: i64,
}

/// Why [`TraceCylinder::new`] refused a cylinder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraceCylinderError {
    /// The radius is zero or negative.
    RadiusNotPositive,
    /// The radius is outside the trace domain ([`crate::TRACE_COORD_LIMIT_RAW`]).
    RadiusOutsideTraceDomain,
    /// A cap height is outside the trace domain ([`crate::TRACE_COORD_LIMIT_RAW`]).
    CapOutsideTraceDomain,
    /// The bottom cap is not below the top cap.
    CapsNotOrdered,
}

impl fmt::Display for TraceCylinderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::RadiusNotPositive => "cylinder radius is not positive",
            Self::RadiusOutsideTraceDomain => "cylinder radius is outside the trace domain",
            Self::CapOutsideTraceDomain => "cylinder cap is outside the trace domain",
            Self::CapsNotOrdered => "cylinder bottom cap is not below its top cap",
        })
    }
}

impl std::error::Error for TraceCylinderError {}

impl TraceCylinder {
    /// Prepares a cylinder about the `+Y` axis with caps at heights `bottom` and `top`.
    ///
    /// # Errors
    ///
    /// Returns [`TraceCylinderError`] when the radius is not positive, the radius or a cap
    /// height is outside the trace domain ([`crate::TRACE_COORD_LIMIT_RAW`]), or `bottom` is
    /// not below `top`, checked in that order.
    pub const fn new(
        radius: FixedQ32_32,
        bottom: FixedQ32_32,
        top: FixedQ32_32,
    ) -> Result<Self, TraceCylinderError> {
        let (radius, bottom, top) = (radius.raw(), bottom.raw(), top.raw());
        if radius <= 0 {
            Err(TraceCylinderError::RadiusNotPositive)
        } else if !in_trace_domain(radius) {
            Err(TraceCylinderError::RadiusOutsideTraceDomain)
        } else if !(in_trace_domain(bottom) && in_trace_domain(top)) {
            Err(TraceCylinderError::CapOutsideTraceDomain)
        } else if bottom >= top {
            Err(TraceCylinderError::CapsNotOrdered)
        } else {
            Ok(Self { radius, bottom, top })
        }
    }

    /// The radius.
    #[must_use]
    pub const fn radius(&self) -> FixedQ32_32 {
        FixedQ32_32::from_raw(self.radius)
    }

    /// The height of the bottom cap.
    #[must_use]
    pub const fn bottom(&self) -> FixedQ32_32 {
        FixedQ32_32::from_raw(self.bottom)
    }

    /// The height of the top cap.
    #[must_use]
    pub const fn top(&self) -> FixedQ32_32 {
        FixedQ32_32::from_raw(self.top)
    }

    /// The smallest box that contains the cylinder.
    #[must_use]
    pub const fn bounds(&self) -> FixedAabb3 {
        let min = vec3([-self.radius, self.bottom, -self.radius]);
        FixedAabb3::new(min, vec3([self.radius, self.top, self.radius]))
    }
}

/// Where a ray hit a cylinder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CylinderHit {
    /// Distance along the ray, in multiples of the ray direction, rounded down.
    pub distance: FixedQ32_32,
    /// Whether the ray hit the outside of the cylinder: true at the entry (the ray arrives
    /// from outside, or touches the surface), false at the exit (the ray leaves from inside).
    pub front_face: bool,
    normal: FixedVec3,
}

impl CylinderHit {
    /// The outward normal at the hit. On a cap it is exactly `(0, -1, 0)` (bottom) or
    /// `(0, 1, 0)` (top), and a cap wins a tie with the side at the rim. On the side it is
    /// `(x, 0, z) / radius` at the rounded distance, each component rounded down; its length
    /// is within a few raw units of 1.0, not exactly 1.0.
    #[must_use]
    pub const fn normal(&self) -> FixedVec3 {
        self.normal
    }
}

impl TraceRay {
    /// Exact ray–cylinder test over distances `t_min..=t_max`, contract
    /// `bunny-ray-cylinder/1`.
    ///
    /// Returns the entry when its rounded distance is in range, else the exit when its
    /// rounded distance is, else `None`. Each distance is the floor of the true distance in
    /// raw units. Contact is inclusive: a ray that only touches the cylinder hits it.
    #[must_use]
    pub fn hit_cylinder(
        &self,
        cylinder: &TraceCylinder,
        t_min: FixedQ32_32,
        t_max: FixedQ32_32,
    ) -> Option<CylinderHit> {
        let (entry, exit) = Line::new(self, cylinder)?.span()?;
        let range = i128::from(t_min.raw())..=i128::from(t_max.raw());
        let (crossing, front_face) = if range.contains(&entry.distance) {
            (entry, true)
        } else if range.contains(&exit.distance) {
            (exit, false)
        } else {
            return None;
        };
        let distance = i64::try_from(crossing.distance).ok()?;
        let normal = normal(self, cylinder.radius, crossing.surface, distance)?;
        Some(CylinderHit { distance: FixedQ32_32::from_raw(distance), front_face, normal })
    }
}
