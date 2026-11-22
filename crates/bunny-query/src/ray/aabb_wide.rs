//! Exact ray/AABB slab test for rays whose slab distances do not fit Q32.32.
//!
//! [`crate::ray_intersects_aabb`] runs this only after its rounded slab test meets a
//! coordinate difference or slab distance outside Q32.32. Every slab distance is held
//! as an exact `i128` ratio, the hit is decided by cross-multiplication, and only the
//! two reported bounds are rounded (ties to even, as `FixedQ32_32::checked_div`) and
//! saturated to the Q32.32 range.

use std::cmp::Ordering;

use bunny_geom::{FixedAabb3, FixedRay3};
use bunny_num::FixedQ32_32;

use super::AxisBounds;

/// An exact slab distance `num / den` in Q32.32 units, with `den > 0`.
///
/// `num` is a difference of two raw `i64` values, so `|num| < 2^64`, and `den` is the
/// magnitude of a raw `i64` direction component, so `den <= 2^63`. A cross product
/// `num * den` is therefore below `2^127` and fits `i128`.
#[derive(Clone, Copy)]
struct Ratio {
    num: i128,
    den: i128,
}

impl Ratio {
    /// `(bound - origin) / direction` for a non-zero `direction`.
    fn slab(bound: FixedQ32_32, origin: FixedQ32_32, direction: FixedQ32_32) -> Self {
        let delta = i128::from(bound.to_raw()) - i128::from(origin.to_raw());
        let direction = i128::from(direction.to_raw());
        if direction < 0 {
            Self { num: -delta, den: -direction }
        } else {
            Self { num: delta, den: direction }
        }
    }

    fn compare(self, other: Self) -> Ordering {
        (self.num * other.den).cmp(&(other.num * self.den))
    }

    fn later(self, other: Self) -> Self {
        if self.compare(other) == Ordering::Less {
            other
        } else {
            self
        }
    }

    fn earlier(self, other: Self) -> Self {
        if self.compare(other) == Ordering::Greater {
            other
        } else {
            self
        }
    }

    /// The nearest raw Q32.32 value (ties to even), saturated to the `i64` range.
    fn to_fixed_saturating(self) -> FixedQ32_32 {
        let scaled = self.num.unsigned_abs() << FixedQ32_32::FRACTIONAL_BITS;
        let den = self.den.unsigned_abs();
        let magnitude = round_half_even(scaled / den, scaled % den, den);
        let signed = if self.num < 0 { -i128::from(magnitude) } else { i128::from(magnitude) };
        saturate(signed)
    }
}

/// Rounds `quotient + remainder / den` to the nearest integer, ties to even.
///
/// `quotient < 2^96`, so the result fits `u128` and, after [`saturate`], `i64`.
fn round_half_even(quotient: u128, remainder: u128, den: u128) -> u64 {
    let twice = remainder << 1;
    let up = twice > den || (twice == den && quotient & 1 == 1);
    let rounded = quotient + u128::from(up);
    u64::try_from(rounded).unwrap_or(u64::MAX)
}

fn saturate(value: i128) -> FixedQ32_32 {
    let clamped = value.clamp(i128::from(i64::MIN), i128::from(i64::MAX));
    FixedQ32_32::from_raw(i64::try_from(clamped).unwrap_or(i64::MAX))
}

/// The exact parametric interval; `None` on a side means no axis bounds it yet.
struct ExactInterval {
    enter: Option<Ratio>,
    exit: Option<Ratio>,
}

impl ExactInterval {
    fn update(&mut self, axis: AxisBounds) -> Option<()> {
        if axis.direction == FixedQ32_32::ZERO {
            return axis.contains_origin().then_some(());
        }
        let t0 = Ratio::slab(axis.min_bound, axis.origin, axis.direction);
        let t1 = Ratio::slab(axis.max_bound, axis.origin, axis.direction);
        let (near, far) = (t0.earlier(t1), t0.later(t1));
        self.enter = Some(self.enter.map_or(near, |enter| enter.later(near)));
        self.exit = Some(self.exit.map_or(far, |exit| exit.earlier(far)));
        Some(())
    }

    fn is_hit(&self) -> bool {
        let Some(exit) = self.exit else {
            return true;
        };
        let reaches = self.enter.is_none_or(|enter| enter.compare(exit) != Ordering::Greater);
        reaches && exit.num >= 0
    }

    fn rounded(&self) -> (FixedQ32_32, FixedQ32_32) {
        let enter = self.enter.map_or(FixedQ32_32::from_raw(i64::MIN), Ratio::to_fixed_saturating);
        let exit = self.exit.map_or(FixedQ32_32::from_raw(i64::MAX), Ratio::to_fixed_saturating);
        (enter, exit)
    }
}

/// Exact slab test with rounded, saturated bounds; see [`crate::ray_intersects_aabb`].
///
/// Marked cold: it runs only when the rounded slab test overflows, so the common path stays
/// as it was.
#[cold]
pub(super) fn exact_slab_hit(
    ray: &FixedRay3,
    aabb: &FixedAabb3,
) -> Option<(FixedQ32_32, FixedQ32_32)> {
    let mut interval = ExactInterval { enter: None, exit: None };
    for axis in AxisBounds::of(ray, aabb) {
        interval.update(axis)?;
    }
    interval.is_hit().then(|| interval.rounded())
}
