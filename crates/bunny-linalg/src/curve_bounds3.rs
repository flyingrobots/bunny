//! Conservative axis-aligned bounds of `bunny-curve/1` curves.
//!
//! Every box here is exact arithmetic on raw coordinates (minimum, maximum, and a widening
//! by a whole number of raw units): nothing is rounded. Why each box contains every point the
//! kernel returns, rounding included, is proved in `docs/topics/curve/README.md`
//! (Conservative Bounds).

use bunny_num::FixedQ32_32;

use crate::{CatmullRom3, CubicBezier3, FixedVec3};

/// How far each face of a split half's control box is widened, in raw units, so that it
/// contains the original curve's evaluations on the half's range: 3/2 for the rounding of the
/// half's control points, plus 3/2 for the rounding of the original's evaluation.
const SPLIT_WIDENING_RAW: i64 = 3;

/// A closed axis-aligned box in Q32.32, `min` to `max` on each axis (`bunny-curve/1`).
///
/// The bounds functions of [`CubicBezier3`] and [`CatmullRom3`] return one with
/// `min <= max` on every axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CurveBounds3 {
    /// The smallest coordinate on each axis.
    pub min: FixedVec3,
    /// The largest coordinate on each axis.
    pub max: FixedVec3,
}

const fn min_raw(a: i64, b: i64) -> i64 {
    if a < b {
        a
    } else {
        b
    }
}

const fn max_raw(a: i64, b: i64) -> i64 {
    if a > b {
        a
    } else {
        b
    }
}

const fn axis_contains(min: FixedQ32_32, max: FixedQ32_32, value: FixedQ32_32) -> bool {
    min.raw() <= value.raw() && value.raw() <= max.raw()
}

const fn vec_from_raw(x: i64, y: i64, z: i64) -> FixedVec3 {
    FixedVec3::new(FixedQ32_32::from_raw(x), FixedQ32_32::from_raw(y), FixedQ32_32::from_raw(z))
}

const fn vec_min(a: FixedVec3, b: FixedVec3) -> FixedVec3 {
    vec_from_raw(
        min_raw(a.x.raw(), b.x.raw()),
        min_raw(a.y.raw(), b.y.raw()),
        min_raw(a.z.raw(), b.z.raw()),
    )
}

const fn vec_max(a: FixedVec3, b: FixedVec3) -> FixedVec3 {
    vec_from_raw(
        max_raw(a.x.raw(), b.x.raw()),
        max_raw(a.y.raw(), b.y.raw()),
        max_raw(a.z.raw(), b.z.raw()),
    )
}

/// `value - pad`, but never below `floor`: `floor` is at least `i64::MIN`, so saturating
/// first and then taking the larger is the exact `max(value - pad, floor)`.
const fn lower_within(value: i64, pad: i64, floor: i64) -> i64 {
    max_raw(value.saturating_sub(pad), floor)
}

/// `value + pad`, but never above `ceiling`, exactly as [`lower_within`].
const fn upper_within(value: i64, pad: i64, ceiling: i64) -> i64 {
    min_raw(value.saturating_add(pad), ceiling)
}

impl CurveBounds3 {
    /// Whether `point` is in the box: `min <= point <= max` on every axis, faces included.
    #[must_use]
    pub const fn contains(self, point: FixedVec3) -> bool {
        axis_contains(self.min.x, self.max.x, point.x)
            && axis_contains(self.min.y, self.max.y, point.y)
            && axis_contains(self.min.z, self.max.z, point.z)
    }

    const fn of_point(point: FixedVec3) -> Self {
        Self { min: point, max: point }
    }

    const fn union(self, other: Self) -> Self {
        Self { min: vec_min(self.min, other.min), max: vec_max(self.max, other.max) }
    }

    /// The box widened by `pad` raw units on every face, then clipped to `outer`.
    const fn widened_within(self, pad: i64, outer: Self) -> Self {
        let (low, high, out_low, out_high) = (self.min, self.max, outer.min, outer.max);
        Self {
            min: vec_from_raw(
                lower_within(low.x.raw(), pad, out_low.x.raw()),
                lower_within(low.y.raw(), pad, out_low.y.raw()),
                lower_within(low.z.raw(), pad, out_low.z.raw()),
            ),
            max: vec_from_raw(
                upper_within(high.x.raw(), pad, out_high.x.raw()),
                upper_within(high.y.raw(), pad, out_high.y.raw()),
                upper_within(high.z.raw(), pad, out_high.z.raw()),
            ),
        }
    }
}

impl CubicBezier3 {
    /// The control box: the per-axis minimum and maximum of `p0` to `p3` (`bunny-curve/1`).
    ///
    /// Exact: no rounding, never refused. It contains [`Self::at`] at every `t` in `[0, 1]`,
    /// bit for bit as `at` returns it, with no widening: each lerp of the de Casteljau
    /// recurrence with `t` in `[0, 1]` rounds a value between its two ends to the nearest raw
    /// unit, which cannot leave the closed range of those ends, ties to even included. The
    /// faces at `p0` and `p3` are reached (`at(0)` and `at(1.0)`); a face set only by `p1` or
    /// `p2` generally is not. Equal control points give that point.
    #[must_use]
    pub const fn control_bounds(self) -> CurveBounds3 {
        CurveBounds3::of_point(self.p0)
            .union(CurveBounds3::of_point(self.p1))
            .union(CurveBounds3::of_point(self.p2))
            .union(CurveBounds3::of_point(self.p3))
    }

    /// Boxes for the original curve on `[0, t]` and on `[t, 1]` (`bunny-curve/1`).
    ///
    /// Each is the control box of the matching half of [`Self::split`] widened by 3 raw units
    /// on every face, then clipped to [`Self::control_bounds`]; exact, no rounding. The first
    /// contains [`Self::at`] at every `s` in `[0, t]` and the second at every `s` in `[t, 1]`,
    /// and each also contains its half's own evaluations. The widening is needed: a half's
    /// control points are rounded, and the original can leave a half's unwidened box.
    ///
    /// Returns `None` only when `t` is outside `[0, 1]`.
    #[must_use]
    pub const fn split_bounds(self, t: FixedQ32_32) -> Option<(CurveBounds3, CurveBounds3)> {
        let outer = self.control_bounds();
        match self.split(t) {
            Some((left, right)) => Some((
                left.control_bounds().widened_within(SPLIT_WIDENING_RAW, outer),
                right.control_bounds().widened_within(SPLIT_WIDENING_RAW, outer),
            )),
            None => None,
        }
    }
}

impl CatmullRom3<'_> {
    /// The union of the control boxes of every segment (`bunny-curve/1`).
    ///
    /// Each segment's [`CubicBezier3::control_bounds`], from [`Self::to_beziers`], without
    /// allocating: `segment_count()` segments, `n - 1` for an open spline and `n` for a closed
    /// one (the closing segment included). It contains [`Self::at`] at every `u` in
    /// `[0, segment_count]`, because `at` is a segment's `CubicBezier3::at`. It is not the box
    /// of the spline's points: a Catmull-Rom curve overshoots its points, and the segments'
    /// inner control points (rounded, within 1/2 raw unit of exact) widen the box to cover it.
    #[must_use]
    pub fn control_bounds(self) -> CurveBounds3 {
        let mut boxes = self.to_beziers().map(CubicBezier3::control_bounds);
        match boxes.next() {
            Some(first) => boxes.fold(first, CurveBounds3::union),
            // Unreachable: `CatmullRom3::new` admits at least one segment and converts every
            // one, so `to_beziers` is never empty. The first point is the defined answer.
            None => match self.points().first() {
                Some(&point) => CurveBounds3::of_point(point),
                None => CurveBounds3::of_point(vec_from_raw(0, 0, 0)),
            },
        }
    }
}
