//! A whole-packet bound on `TraceRay::box_entry` for rays that share one origin.
//!
//! For one axis, `box_entry` computes a slab lower bound `scaled(near, r)` and upper bound
//! `scaled(far, r + 1) + 1` from the coordinate differences `near` and `far`, which depend
//! only on the origin, the box and the sign of the direction component, and the ray's
//! reciprocal `r = floor(2^63 / |d|)`. `scaled(v, r) = min(floor(v * r / 2^31), u64::MAX)` is
//! non-decreasing in `r` for a fixed `v`, so over rays with one origin and one sign per
//! axis, the smallest and largest reciprocal bound every ray's slab bounds on that axis.
//! See `docs/topics/ray-tracing/README.md#packet-traversal`.

use bunny_geom::FixedAabb3;
use bunny_num::FixedQ32_32;
use bunny_query::TraceRay;

/// What the direction components of a packet on one axis have in common.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Axis {
    /// Every component is positive (`true`) or every one is negative (`false`), with the
    /// smallest and largest reciprocal over the packet.
    Signed { positive: bool, low: u64, high: u64 },
    /// Every component is zero.
    Zero,
    /// Components of different signs, or zero and nonzero ones.
    Mixed,
}

/// The packet's common origin and per-axis reciprocal intervals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Frustum {
    origin: [i64; 3],
    axes: [Axis; 3],
}

/// The answer of the whole-packet test for one box, for rays whose limits lie in a range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PacketVerdict {
    /// `box_entry` returns `None` for every ray of the packet with its own limit.
    Reject,
    /// `box_entry` returns a bound for every ray, and each bound is at least this one.
    Accept(FixedQ32_32),
    /// Neither is certain: test the rays one by one.
    Test,
}

/// Bounds of one axis over the packet: `(lower_lo, lower_hi, upper_lo, upper_hi)`, and
/// whether the `_hi` lower and `_lo` upper values bound every ray (they do not on a mixed
/// axis). `None` means every ray rejects the box on this axis.
type AxisBounds = Option<((u64, u64, u64, u64), bool)>;

impl Frustum {
    /// The frustum of rays that share one origin, or `None` when their origins differ or
    /// there are no rays.
    pub(super) fn new(rays: &[TraceRay]) -> Option<Self> {
        let origin = raw3(rays.first()?.origin());
        if rays.iter().any(|ray| raw3(ray.origin()) != origin) {
            return None;
        }
        let directions: Vec3s<'_> = Vec3s(rays);
        Some(Self { origin, axes: [0, 1, 2].map(|axis| directions.axis(axis)) })
    }

    /// The whole-packet test of `aabb` for rays whose limits lie in `low..=high`.
    pub(super) fn classify(
        &self,
        aabb: &FixedAabb3,
        low: FixedQ32_32,
        high: FixedQ32_32,
    ) -> PacketVerdict {
        let Ok(high) = u64::try_from(high.raw()) else { return PacketVerdict::Reject };
        let (min, max) = (raw3(aabb.min), raw3(aabb.max));
        let mut enter = (0_u64, 0_u64);
        let mut exit = (u64::MAX, u64::MAX);
        let mut exact = true;
        for axis in 0..3 {
            let Some(((lo, hi, up_lo, up_hi), all)) = self.bounds(axis, min, max) else {
                return PacketVerdict::Reject;
            };
            enter = (enter.0.max(lo), enter.1.max(hi));
            exit = (exit.0.min(up_lo), exit.1.min(up_hi));
            exact &= all;
        }
        decide(enter, exit, exact, (low, high))
    }

    fn bounds(&self, axis: usize, min: [i64; 3], max: [i64; 3]) -> AxisBounds {
        let pick = |v: [i64; 3]| v.get(axis).copied().map(i128::from);
        let (origin, min, max) = (pick(self.origin)?, pick(min)?, pick(max)?);
        match self.axes.get(axis)? {
            Axis::Zero => {
                (min <= origin && origin <= max).then_some(((0, 0, u64::MAX, u64::MAX), true))
            }
            Axis::Mixed => Some(((0, u64::MAX, 0, u64::MAX), false)),
            &Axis::Signed { positive, low, high } => {
                let (near, far) = if positive {
                    (min - origin, max - origin)
                } else {
                    (origin - max, origin - min)
                };
                let far = u64::try_from(far).ok()?;
                let near = u64::try_from(near).unwrap_or(0);
                let upper = |r: u64| scaled(far, r + 1).saturating_add(1);
                Some(((scaled(near, low), scaled(near, high), upper(low), upper(high)), true))
            }
        }
    }
}

/// Reject when even the smallest entry exceeds the largest exit or the largest limit;
/// accept when the largest entry is within the smallest exit and the smallest limit.
fn decide(
    enter: (u64, u64),
    exit: (u64, u64),
    exact: bool,
    (low, high): (FixedQ32_32, u64),
) -> PacketVerdict {
    if enter.0 > exit.1 || enter.0 > high {
        return PacketVerdict::Reject;
    }
    let low = u64::try_from(low.raw()).ok();
    let certain = exact && enter.1 <= exit.0 && low.is_some_and(|low| enter.1 <= low);
    match i64::try_from(enter.0) {
        Ok(bound) if certain => PacketVerdict::Accept(FixedQ32_32::from_raw(bound)),
        _ => PacketVerdict::Test,
    }
}

/// The rays of a packet, read one axis at a time.
struct Vec3s<'a>(&'a [TraceRay]);

impl Vec3s<'_> {
    fn axis(&self, axis: usize) -> Axis {
        let component = |ray: &TraceRay| raw3(ray.direction()).get(axis).copied().unwrap_or(0);
        let mut components = self.0.iter().map(component);
        let Some(first) = components.next() else { return Axis::Mixed };
        let sign = first.signum();
        let (mut low, mut high) = (reciprocal(first), reciprocal(first));
        for c in components {
            if c.signum() != sign {
                return Axis::Mixed;
            }
            (low, high) = (low.min(reciprocal(c)), high.max(reciprocal(c)));
        }
        if sign == 0 {
            return Axis::Zero;
        }
        Axis::Signed { positive: sign > 0, low, high }
    }
}

/// `floor(2^63 / |d|)`, the reciprocal `TraceRay::new` stores; zero for a zero component.
const fn reciprocal(direction: i64) -> u64 {
    match direction.unsigned_abs() {
        0 => 0,
        magnitude => (1_u64 << 63) / magnitude,
    }
}

/// `floor(value * reciprocal / 2^31)`, saturating, as `box_entry` computes it.
fn scaled(value: u64, reciprocal: u64) -> u64 {
    let product = u128::from(value) * u128::from(reciprocal);
    u64::try_from(product >> 31).unwrap_or(u64::MAX)
}

const fn raw3(v: bunny_linalg::FixedVec3) -> [i64; 3] {
    [v.x.raw(), v.y.raw(), v.z.raw()]
}
