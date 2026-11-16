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

/// An exact crossing distance in raw units: `numerator / denominator`, denominator positive.
#[derive(Clone, Copy)]
struct Crossing {
    numerator: i128,
    denominator: i128,
    /// The axis of the face crossed and the sign of its outward normal.
    axis: usize,
    outward: i64,
}

impl Crossing {
    fn compare(&self, other: &Self) -> Ordering {
        // `|numerator| < 2^73` and `denominator <= 2^32 + 1`: the products are below `2^107`.
        (self.numerator * other.denominator).cmp(&(other.numerator * self.denominator))
    }

    fn floor(&self) -> i128 {
        self.numerator.div_euclid(self.denominator)
    }

    /// The outward unit normal of the face crossed.
    fn normal(&self) -> FixedVec3 {
        let mut raw = [0_i64; 3];
        raw[self.axis] = self.outward << FixedQ32_32::FRACTIONAL_BITS;
        vec3(raw)
    }
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
        let distance = i64::try_from(crossing.floor()).ok()?;
        Some(BoxHit {
            distance: FixedQ32_32::from_raw(distance),
            front_face,
            normal: crossing.normal(),
        })
    }

    /// The entry and the exit crossings of the line through the box, or `None` when the line
    /// misses the closed box.
    ///
    /// An axis whose direction component is zero constrains nothing when the origin lies in
    /// its slab, bounds included, and rules the box out otherwise. Among equal entries the
    /// lowest axis wins, and likewise among equal exits.
    fn span(&self, aabb: &TraceBox) -> Option<(Crossing, Crossing)> {
        let mut entry: Option<Crossing> = None;
        let mut exit: Option<Crossing> = None;
        for axis in 0..3 {
            // Offsets of the faces from the origin: below `2^41` in magnitude.
            let low = i128::from(aabb.min[axis]) - i128::from(self.origin[axis]);
            let high = i128::from(aabb.max[axis]) - i128::from(self.origin[axis]);
            let d = i128::from(self.direction[axis]);
            if d == 0 {
                if low > 0 || high < 0 {
                    return None;
                }
                continue;
            }
            // A positive direction enters at the minimum face and leaves at the maximum
            // face; a negative one the reverse. Scaling by `2^32` makes raw distances.
            let (near, far, outward) = if d > 0 { (low, high, 1) } else { (-high, -low, -1) };
            let denominator = d.abs();
            let near = Crossing {
                numerator: near << FixedQ32_32::FRACTIONAL_BITS,
                denominator,
                axis,
                // The entry face's outward normal points against the ray.
                outward: -outward,
            };
            let far = Crossing {
                numerator: far << FixedQ32_32::FRACTIONAL_BITS,
                denominator,
                axis,
                outward,
            };
            if entry.is_none_or(|best| near.compare(&best) == Ordering::Greater) {
                entry = Some(near);
            }
            if exit.is_none_or(|best| far.compare(&best) == Ordering::Less) {
                exit = Some(far);
            }
        }
        let (entry, exit) = (entry?, exit?);
        (entry.compare(&exit) != Ordering::Greater).then_some((entry, exit))
    }
}
