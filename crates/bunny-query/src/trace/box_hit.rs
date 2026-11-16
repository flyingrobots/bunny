//! Exact ray–box test, contract `bunny-ray-box/1`.
//!
//! The box is axis-aligned. On each axis the ray is inside the slab between the two faces for
//! the distances between two exact rational crossings; the ray is inside the box between the
//! largest entry and the smallest exit. The crossings are compared as exact fractions and the
//! reported distance is the floor of one of them. See
//! `docs/topics/ray-tracing/README.md#ray-box`.

use super::{in_trace_domain, raw3, vec3, TraceRay};
use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use std::cmp::Ordering;
use std::fmt;

/// The contract identity of [`TraceRay::hit_box`].
pub const RAY_BOX_ID: &str = "bunny-ray-box/1";

/// A box prepared for [`TraceRay::hit_box`]: two corners in the trace domain, the minimum
/// not above the maximum on any axis. A box may be flat (equal bounds on an axis).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraceBox {
    min: [i64; 3],
    max: [i64; 3],
}

/// Why [`TraceBox::new`] refused a box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraceBoxError {
    /// A corner coordinate is outside the trace domain ([`crate::TRACE_COORD_LIMIT_RAW`]).
    CornerOutsideTraceDomain,
    /// A minimum coordinate is above the matching maximum coordinate.
    MinimumAboveMaximum,
}

impl fmt::Display for TraceBoxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::CornerOutsideTraceDomain => "box corner is outside the trace domain",
            Self::MinimumAboveMaximum => "box minimum is above its maximum",
        })
    }
}

impl std::error::Error for TraceBoxError {}

impl TraceBox {
    /// Prepares a box.
    ///
    /// # Errors
    ///
    /// Returns [`TraceBoxError`] when a corner coordinate is outside the trace domain
    /// ([`crate::TRACE_COORD_LIMIT_RAW`]) or a minimum coordinate is above its maximum.
    pub fn new(min: FixedVec3, max: FixedVec3) -> Result<Self, TraceBoxError> {
        let (min, max) = (raw3(min), raw3(max));
        if !min.iter().chain(&max).all(|&c| in_trace_domain(c)) {
            Err(TraceBoxError::CornerOutsideTraceDomain)
        } else if min.iter().zip(&max).any(|(lo, hi)| lo > hi) {
            Err(TraceBoxError::MinimumAboveMaximum)
        } else {
            Ok(Self { min, max })
        }
    }

    /// The minimum corner.
    #[must_use]
    pub const fn min(&self) -> FixedVec3 {
        vec3(self.min)
    }

    /// The maximum corner.
    #[must_use]
    pub const fn max(&self) -> FixedVec3 {
        vec3(self.max)
    }

    /// The box as a [`FixedAabb3`].
    #[must_use]
    pub const fn bounds(&self) -> FixedAabb3 {
        FixedAabb3::new(vec3(self.min), vec3(self.max))
    }
}

/// Where a ray hit a box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoxHit {
    /// Distance along the ray, in multiples of the ray direction, rounded down.
    pub distance: FixedQ32_32,
    /// Whether the ray hit the outside of the box: true at the entry (the ray arrives from
    /// outside, or touches the box), false at the exit (the ray leaves from inside).
    pub front_face: bool,
    normal: FixedVec3,
}

impl BoxHit {
    /// The outward normal of the face hit: a unit vector along one axis, exactly 1.0 or
    /// -1.0 in one component and zero in the others.
    #[must_use]
    pub const fn normal(&self) -> FixedVec3 {
        self.normal
    }
}

/// An exact crossing distance in raw units: `numerator / denominator`, denominator positive,
/// with the outward normal of the face crossed.
#[derive(Clone, Copy)]
struct Crossing {
    numerator: i128,
    denominator: i128,
    normal: [i64; 3],
}

impl Crossing {
    fn compare(&self, other: &Self) -> Ordering {
        // `|numerator| < 2^73` and `denominator <= 2^32 + 1`: the products are below `2^107`.
        (self.numerator * other.denominator).cmp(&(other.numerator * self.denominator))
    }

    const fn floor(&self) -> i128 {
        self.numerator.div_euclid(self.denominator)
    }
}

/// The raw unit vector along `axis`, pointing the way of `sign`.
fn unit_normal(axis: usize, sign: i64) -> [i64; 3] {
    std::array::from_fn(|i| if i == axis { sign << FixedQ32_32::FRACTIONAL_BITS } else { 0 })
}

/// What one axis says about the line.
enum Slab {
    /// The direction is zero and the origin is outside the closed slab: no hit.
    Miss,
    /// The direction is zero and the origin is inside the closed slab: no constraint.
    Free,
    /// The line is inside the slab between these two crossings, entry first.
    Between(Crossing, Crossing),
}

/// The slab of `axis` between the raw bounds `min` and `max`, for a ray with this origin and
/// direction component. Offsets of the faces from the origin are below `2^41` in magnitude.
fn slab(axis: usize, origin: i64, direction: i64, (min, max): (i64, i64)) -> Slab {
    let low = i128::from(min) - i128::from(origin);
    let high = i128::from(max) - i128::from(origin);
    let d = i128::from(direction);
    if d == 0 {
        return if low > 0 || high < 0 { Slab::Miss } else { Slab::Free };
    }
    // A positive direction enters at the minimum face and leaves at the maximum face; a
    // negative one the reverse. Scaling by `2^32` makes the distances raw.
    let (near, far, outward) = if d > 0 { (low, high, 1) } else { (-high, -low, -1) };
    let crossing = |offset: i128, sign: i64| Crossing {
        numerator: offset << FixedQ32_32::FRACTIONAL_BITS,
        denominator: d.abs(),
        normal: unit_normal(axis, sign),
    };
    // The entry face's outward normal points against the ray, the exit face's along it.
    Slab::Between(crossing(near, -outward), crossing(far, outward))
}

impl TraceRay {
    /// Exact ray–box test over distances `t_min..=t_max`, contract `bunny-ray-box/1`.
    ///
    /// Returns the entry when its rounded distance is in range, else the exit when its
    /// rounded distance is, else `None`. The box is closed: touching a face, edge or corner
    /// is a hit. Each distance is the floor of the true distance in raw units.
    #[must_use]
    pub fn hit_box(
        &self,
        aabb: &TraceBox,
        t_min: FixedQ32_32,
        t_max: FixedQ32_32,
    ) -> Option<BoxHit> {
        let (entry, exit) = self.span(aabb)?;
        let range = i128::from(t_min.raw())..=i128::from(t_max.raw());
        let (crossing, front_face) = if range.contains(&entry.floor()) {
            (entry, true)
        } else if range.contains(&exit.floor()) {
            (exit, false)
        } else {
            return None;
        };
        let distance = FixedQ32_32::from_raw(i64::try_from(crossing.floor()).ok()?);
        Some(BoxHit { distance, front_face, normal: vec3(crossing.normal) })
    }

    /// The entry and the exit crossings of the line through the box, or `None` when the line
    /// misses the closed box.
    ///
    /// An axis whose direction component is zero constrains nothing when the origin lies in
    /// its slab, bounds included, and rules the box out otherwise. Among equal entries the
    /// lowest axis wins, and likewise among equal exits.
    fn span(&self, aabb: &TraceBox) -> Option<(Crossing, Crossing)> {
        let mut span: Option<(Crossing, Crossing)> = None;
        let axes =
            self.origin.into_iter().zip(self.direction).zip(aabb.min.into_iter().zip(aabb.max));
        for (axis, ((origin, direction), bounds)) in axes.enumerate() {
            match slab(axis, origin, direction, bounds) {
                Slab::Miss => return None,
                Slab::Free => {}
                Slab::Between(near, far) => span = Some(narrow(span, near, far)),
            }
        }
        let (entry, exit) = span?;
        (entry.compare(&exit) != Ordering::Greater).then_some((entry, exit))
    }
}

/// The span tightened by one more slab; an earlier axis keeps a tie.
fn narrow(
    span: Option<(Crossing, Crossing)>,
    near: Crossing,
    far: Crossing,
) -> (Crossing, Crossing) {
    span.map_or((near, far), |(entry, exit)| {
        (
            if near.compare(&entry) == Ordering::Greater { near } else { entry },
            if far.compare(&exit) == Ordering::Less { far } else { exit },
        )
    })
}
