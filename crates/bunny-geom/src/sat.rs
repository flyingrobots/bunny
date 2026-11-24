//! Exact separating-axis tests for convex shapes (`bunny-sat/1`).
//!
//! Each test projects both shapes onto a fixed, ordered list of candidate axes and compares
//! the projections exactly, on axes that are never normalised: an axis is an integer vector
//! built from raw coordinates, and both shapes are projected onto the same vector, so the
//! comparison needs no division and no square root. Nothing is rounded and no input
//! overflows. See `docs/topics/convex-collision/README.md`.

use core::cmp::Ordering;

use bunny_linalg::{FixedVec2, FixedVec3};
use bunny_num::FixedQ32_32;

use crate::{
    doubled_signed_area2, sextupled_signed_volume3, FixedAabb3, FixedObb3, FixedTriangle3,
};

mod wide;

use wide::I256;

/// The identity string of the separating-axis contract.
pub const SAT_CONTRACT_ID: &str = "bunny-sat/1";

/// How two closed convex shapes relate, with the axis that shows it.
///
/// The shapes are closed sets: a shared face, edge or point is contact. `A` names a
/// candidate axis of the shape pair; the order of the candidates is part of the contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SatClass<A> {
    /// The shapes are disjoint. `axis` is the first candidate, in the pair's order, on which
    /// the two projections are disjoint intervals.
    Separated {
        /// The first strictly separating axis.
        axis: A,
    },
    /// The shapes meet, but only within a plane: no candidate separates them, and on `axis`,
    /// the first candidate on which the projections share exactly one end point, the shapes
    /// lie on opposite closed sides of a plane normal to it. An arbitrarily small translation
    /// along that axis separates them.
    Touching {
        /// The first axis with a zero gap.
        axis: A,
    },
    /// The shapes overlap with room to spare: on every candidate the projections share more
    /// than one point, so every small enough translation of either shape keeps them meeting.
    /// For two solid shapes this is: their interiors meet.
    Overlapping,
}

impl<A: Copy> SatClass<A> {
    /// Whether the closed shapes have a point in common (`Touching` or `Overlapping`).
    #[must_use]
    pub const fn intersects(&self) -> bool {
        !matches!(self, Self::Separated { .. })
    }

    /// The reported axis: the separating one, or the touching one; `None` for `Overlapping`.
    #[must_use]
    pub const fn axis(&self) -> Option<A> {
        match self {
            Self::Separated { axis } | Self::Touching { axis } => Some(*axis),
            Self::Overlapping => None,
        }
    }
}

/// A candidate axis of two oriented boxes, in the order [`sat_obb_obb`] tests them.
///
/// With `a_i` the columns of the first box's axes and `b_j` those of the second, taken
/// exactly as raw integer vectors:
///
/// 1. `FaceOfFirst(0..3)`: `a_1 x a_2`, `a_2 x a_0`, `a_0 x a_1`, the normals of the first
///    box's faces (face `k` is the pair of faces crossed by `a_k`);
/// 2. `FaceOfSecond(0..3)`: the same for the second box;
/// 3. `EdgePair { first: i, second: j }`: `a_i x b_j`, in the order `(0, 0)`, `(0, 1)`,
///    `(0, 2)`, `(1, 0)`, ..., `(2, 2)`.
///
/// An edge pair whose cross product is exactly zero (parallel edges) is skipped, never
/// reported. The face normals are not the columns themselves: the columns are orthonormal
/// only within the box's tolerance, and the face normals are what bound the box exactly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ObbAxis {
    /// The normal of face `k` of the first box.
    FaceOfFirst(usize),
    /// The normal of face `k` of the second box.
    FaceOfSecond(usize),
    /// The cross product of axis `first` of the first box and axis `second` of the second.
    EdgePair {
        /// The first box's axis index.
        first: usize,
        /// The second box's axis index.
        second: usize,
    },
}

/// A candidate axis of a triangle and an axis-aligned box, in the order
/// [`sat_triangle_aabb`] tests them.
///
/// With the triangle's edges `e_0 = v1 - v0`, `e_1 = v2 - v1`, `e_2 = v0 - v2` and the unit
/// axes `u_0 = x`, `u_1 = y`, `u_2 = z`:
///
/// 1. `BoxFace(0..3)`: `u_k`;
/// 2. `TriangleFace`: `(v1 - v0) x (v2 - v0)`, skipped when exactly zero (a degenerate
///    triangle);
/// 3. `EdgeCross { edge, axis }`: `e_edge x u_axis`, in the order `(0, 0)`, `(0, 1)`, ...,
///    `(2, 2)`, skipped when exactly zero (the edge is parallel to `u_axis`, or has zero
///    length).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TriangleAabbAxis {
    /// The normal of the box's faces across axis `k`.
    BoxFace(usize),
    /// The triangle's normal.
    TriangleFace,
    /// The cross product of triangle edge `edge` and box axis `axis`.
    EdgeCross {
        /// The edge index: 0 is `v0` to `v1`, 1 is `v1` to `v2`, 2 is `v2` to `v0`.
        edge: usize,
        /// The box axis index.
        axis: usize,
    },
}

/// The sign of the gap `max(q_lo - p_hi, p_lo - q_hi)` between two closed intervals: positive
/// when they are disjoint, zero when they share exactly one end point, negative otherwise.
/// Formed by comparing end points, never by subtracting them.
fn gap(p: (I256, I256), q: (I256, I256)) -> Ordering {
    q.0.cmp(&p.1).max(p.0.cmp(&q.1))
}

/// The first axis with a positive gap; else the first with a zero gap; else `Overlapping`.
/// `None` gaps (zero axes) are skipped.
fn decide<A: Copy>(gaps: impl Iterator<Item = (A, Option<Ordering>)>) -> SatClass<A> {
    let mut touching = None;
    for (axis, sign) in gaps {
        match sign {
            Some(Ordering::Greater) => return SatClass::Separated { axis },
            Some(Ordering::Equal) if touching.is_none() => touching = Some(axis),
            _ => {}
        }
    }
    match touching {
        Some(axis) => SatClass::Touching { axis },
        None => SatClass::Overlapping,
    }
}

type Vector = [I256; 3];

fn dot(a: &Vector, b: &Vector) -> I256 {
    let ([ax, ay, az], [bx, by, bz]) = (*a, *b);
    ax.mul(bx).add(ay.mul(by)).add(az.mul(bz))
}

fn cross(a: &Vector, b: &Vector) -> Vector {
    let ([ax, ay, az], [bx, by, bz]) = (*a, *b);
    [ay.mul(bz).sub(az.mul(by)), az.mul(bx).sub(ax.mul(bz)), ax.mul(by).sub(ay.mul(bx))]
}

fn wide3(v: FixedVec3) -> Vector {
    [v.x, v.y, v.z].map(|c| I256::from_i64(c.raw()))
}

/// An oriented box as wide integers: the centre, the columns and the half extents, raw.
struct WideObb {
    center: Vector,
    columns: [Vector; 3],
    half: Vector,
}

impl WideObb {
    fn of(obb: &FixedObb3) -> Self {
        let axes = obb.axes();
        Self {
            center: wide3(obb.center()),
            columns: [axes.column0(), axes.column1(), axes.column2()].map(wide3),
            half: wide3(obb.half_extents()),
        }
    }

    /// Column `i`.
    const fn column(&self, i: usize) -> Vector {
        let [c0, c1, c2] = self.columns;
        match i {
            0 => c0,
            1 => c1,
            _ => c2,
        }
    }

    /// The face normal `k`: the cross product of the other two columns, in cyclic order.
    fn face(&self, k: usize) -> Vector {
        cross(&self.column((k + 1) % 3), &self.column((k + 2) % 3))
    }

    /// The projection of the box onto `axis` on the grid of `2^-64` units:
    /// `c . L 2^32 -+ sum(h_i |a_i . L|)` on raw values.
    ///
    /// Widths, for a box the constructor accepted: every column entry has magnitude at most
    /// `2^32 + 11`, so every axis component (a cross product of two columns) is below `2^66`;
    /// `c . L 2^32` is below `3 2^63 2^66 2^32 < 2^163`; each `a_i . L` is below
    /// `3 2^33 2^66 < 2^101`, each `h_i |a_i . L|` below `2^164` and their sum below `2^166`,
    /// so both ends are below `2^167` in magnitude and fit the 256-bit integer exactly.
    fn interval(&self, axis: &Vector) -> (I256, I256) {
        let centre = dot(&self.center, axis).shl32();
        let radius = self
            .columns
            .iter()
            .zip(&self.half)
            .fold(I256::ZERO, |sum, (column, half)| sum.add(half.mul(dot(column, axis).abs())));
        (centre.sub(radius), centre.add(radius))
    }
}

/// The candidate axis at position `n` (`0..15`) of [`ObbAxis`]'s order.
const fn obb_axis(n: usize) -> ObbAxis {
    match n {
        0..=2 => ObbAxis::FaceOfFirst(n),
        3..=5 => ObbAxis::FaceOfSecond(n - 3),
        _ => ObbAxis::EdgePair { first: (n - 6) / 3, second: (n - 6) % 3 },
    }
}

/// Classifies two oriented boxes by the separating-axis theorem, exactly.
///
/// Each box is the exact set `center + axes * l`, `abs(l_i) <= half_extents_i`, of
/// [`FixedObb3`] (no corner is rounded). Both boxes are projected onto each candidate axis of
/// [`ObbAxis`], in its order, as closed intervals on the grid of `2^-64` units; the result is
/// [`SatClass::Separated`] at the first axis whose intervals are disjoint, else
/// [`SatClass::Touching`] at the first whose intervals share exactly one end point, else
/// [`SatClass::Overlapping`]. The fifteen axes include every facet normal of the boxes'
/// Minkowski difference, so `Separated` holds exactly when the boxes are disjoint, for flat
/// (zero half extent) boxes too.
///
/// Exact for every pair of boxes the constructor accepts, at any centre: every intermediate
/// is below `2^168` in magnitude and nothing is rounded, so there is no refusal and no
/// overflow. The classification does not depend on the argument order; the reported axis is
/// the first in the order for the arguments as given, so swapping them can report a
/// different, equally valid, axis.
#[must_use]
pub fn sat_obb_obb(first: &FixedObb3, second: &FixedObb3) -> SatClass<ObbAxis> {
    let (a, b) = (WideObb::of(first), WideObb::of(second));
    decide((0..15).map(|n| {
        let name = obb_axis(n);
        let axis = match name {
            ObbAxis::FaceOfFirst(k) => a.face(k),
            ObbAxis::FaceOfSecond(k) => b.face(k),
            ObbAxis::EdgePair { first, second } => cross(&a.column(first), &b.column(second)),
        };
        let sign =
            (!axis.iter().all(|c| c.is_zero())).then(|| gap(a.interval(&axis), b.interval(&axis)));
        (name, sign)
    }))
}

/// The candidate axis at position `n` (`0..13`) of [`TriangleAabbAxis`]'s order.
const fn triangle_aabb_axis(n: usize) -> TriangleAabbAxis {
    match n {
        0..=2 => TriangleAabbAxis::BoxFace(n),
        3 => TriangleAabbAxis::TriangleFace,
        _ => TriangleAabbAxis::EdgeCross { edge: (n - 4) / 3, axis: (n - 4) % 3 },
    }
}

const fn coordinate(v: FixedVec3, k: usize) -> FixedQ32_32 {
    match k {
        0 => v.x,
        1 => v.y,
        _ => v.z,
    }
}

/// The smallest and largest of some exact values; every caller passes at least one.
fn span(mut values: impl Iterator<Item = I256>) -> (I256, I256) {
    let first = values.next().unwrap_or(I256::ZERO);
    values.fold((first, first), |(lo, hi), x| (lo.min(x), hi.max(x)))
}

/// Classifies a triangle against an axis-aligned box by the separating-axis theorem, exactly.
///
/// The triangle is the closed set of convex combinations of its vertices, degenerate ones
/// included (a segment or a point). The box is the closed set of its eight corners' convex
/// hull; an inverted box (`min > max` on an axis) is the box with the same eight corners, as
/// in plane classification, and a flat box (`min == max`) is a rectangle, a segment or a
/// point. The thirteen candidate axes of [`TriangleAabbAxis`] are tested in order, with the
/// rule of [`sat_obb_obb`].
///
/// The projections are the exact predicates of `bunny-predicates/1`: on a box face, the raw
/// coordinates; on the triangle face, [`crate::sextupled_signed_volume3`] of the triangle and
/// each box corner; on an edge cross product, [`crate::doubled_signed_area2`] of the edge and
/// each point in the plane that drops the box axis (the remaining axes in increasing order).
/// Each is exact for every representable input (magnitude below `2^194`), and comparisons
/// are of end points, so nothing is rounded and nothing overflows.
#[must_use]
pub fn sat_triangle_aabb(
    triangle: &FixedTriangle3,
    aabb: &FixedAabb3,
) -> SatClass<TriangleAabbAxis> {
    let bounds = Bounds::of(aabb);
    decide((0..13).map(|n| {
        let name = triangle_aabb_axis(n);
        let sign = match name {
            TriangleAabbAxis::BoxFace(k) => Some(box_face_gap(triangle, &bounds, k)),
            TriangleAabbAxis::TriangleFace => triangle_face_gap(triangle, &bounds),
            TriangleAabbAxis::EdgeCross { edge, axis } => {
                edge_cross_gap(triangle, &bounds, edge, axis)
            }
        };
        (name, sign)
    }))
}

/// A box with `lo <= hi` on every axis.
struct Bounds {
    lo: FixedVec3,
    hi: FixedVec3,
}

impl Bounds {
    /// The box with the same eight corners, ordered per axis.
    fn of(aabb: &FixedAabb3) -> Self {
        let (a, b) = (aabb.min, aabb.max);
        Self {
            lo: FixedVec3::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z)),
            hi: FixedVec3::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z)),
        }
    }

    /// Coordinate `k` of the low end when `high` is false, else of the high end.
    const fn end(&self, k: usize, high: bool) -> FixedQ32_32 {
        coordinate(if high { self.hi } else { self.lo }, k)
    }

    /// The corner whose coordinate `k` is high exactly when bit `k` of `mask` is set.
    const fn corner(&self, mask: usize) -> FixedVec3 {
        FixedVec3::new(
            self.end(0, mask & 1 != 0),
            self.end(1, mask & 2 != 0),
            self.end(2, mask & 4 != 0),
        )
    }
}

fn wide(x: FixedQ32_32) -> I256 {
    I256::from_i64(x.raw())
}

/// On `u_k` the projections are the raw coordinates.
fn box_face_gap(triangle: &FixedTriangle3, bounds: &Bounds, k: usize) -> Ordering {
    let tri = span(triangle.vertices().iter().map(|p| wide(coordinate(*p, k))));
    gap(tri, (wide(bounds.end(k, false)), wide(bounds.end(k, true))))
}

/// On the triangle's normal the projection of a point `x`, up to the common offset and
/// scale, is the exact `(x - v0) . ((v1 - v0) x (v2 - v0))`: zero for the whole triangle, and
/// for the box its least and greatest value over the eight corners. `None` for a degenerate
/// triangle, whose normal is zero.
fn triangle_face_gap(triangle: &FixedTriangle3, bounds: &Bounds) -> Option<Ordering> {
    if triangle.is_degenerate() {
        return None;
    }
    let [v0, v1, v2] = triangle.vertices();
    let side = |c: FixedVec3| I256::from_exact(&sextupled_signed_volume3(v0, v1, v2, c));
    Some(gap((I256::ZERO, I256::ZERO), span((0..8).map(|m| side(bounds.corner(m))))))
}

/// The two coordinates that remain when `axis` is dropped, in increasing order.
const fn kept_axes(axis: usize) -> (usize, usize) {
    match axis {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    }
}

/// Edge `edge`'s start and end vertices, then the third vertex.
const fn edge_ends(triangle: &FixedTriangle3, edge: usize) -> (FixedVec3, FixedVec3, FixedVec3) {
    let [v0, v1, v2] = triangle.vertices();
    match edge {
        0 => (v0, v1, v2),
        1 => (v1, v2, v0),
        _ => (v2, v0, v1),
    }
}

/// On `e_edge x u_axis` the projection of a point `x`, up to sign and the common offset, is
/// the exact `(q - p) x (x' - p)` in the plane that drops `axis`, with `p`, `q` the edge's
/// ends: zero on the edge, so the triangle is the interval from zero to its third vertex's
/// value, and the box spans its four projected corners. `None` when the projected edge is a
/// point (the edge is parallel to `u_axis`, or of zero length).
fn edge_cross_gap(
    triangle: &FixedTriangle3,
    bounds: &Bounds,
    edge: usize,
    axis: usize,
) -> Option<Ordering> {
    let (kept0, kept1) = kept_axes(axis);
    let (start, end, apex) = edge_ends(triangle, edge);
    let flat =
        |point: FixedVec3| FixedVec2::new(coordinate(point, kept0), coordinate(point, kept1));
    let (start, end, apex) = (flat(start), flat(end), flat(apex));
    if start == end {
        return None;
    }
    let side = |point: FixedVec2| I256::from_exact(&doubled_signed_area2(start, end, point));
    let third = side(apex);
    let tri = (third.min(I256::ZERO), third.max(I256::ZERO));
    let corner =
        |m: usize| FixedVec2::new(bounds.end(kept0, m & 1 != 0), bounds.end(kept1, m & 2 != 0));
    Some(gap(tri, span((0..4).map(|m| side(corner(m))))))
}
