//! CHARACTERIZATION tests for flyingrobots/bunny#116: what each public `bunny-query`
//! constructor and query does TODAY with each degenerate input that applies to it.
//!
//! These tests pin current behaviour, whatever it is. They are not an endorsed contract: a
//! cell pinned here can be inconsistent with a neighbouring cell, and several are (an inverted
//! box is a hit for `ray_intersects_aabb` and a miss for `TraceRay::box_entry`; a negative
//! radius is read as its magnitude; saturating arithmetic gives a false overlap at the ends of
//! the raw range). Changing a pinned cell is a behaviour change to a public function, which
//! the repository owner decides (see `docs/topics/degeneracy/README.md`); update the test and
//! the inventory together.

use bunny_geom::{FixedAabb3, FixedRay3, FixedSphere3};
use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::FixedQ32_32;
use bunny_query::{
    aabb_intersects_sphere, closest_point_aabb, closest_point_triangle, closest_points_segments,
    ray_intersects_aabb, ray_intersects_aabb_prepared, ray_intersects_sphere,
    ray_intersects_triangle, PreparedRay3, ShapeFrame, ShapeFrameError, TraceBox, TraceBoxError,
    TraceCylinder, TraceCylinderError, TraceDisc, TraceDiscError, TraceRay, TraceSphere,
    TraceSphereError, TraceTriangle, TRACE_COORD_LIMIT_RAW,
};
use wasm_bindgen_test::wasm_bindgen_test;

const MAX: i64 = i64::MAX;
const MIN: i64 = i64::MIN;
const ONE: i64 = 1 << 32;

type Raw3 = [i64; 3];

fn q(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

fn v(raw: Raw3) -> FixedVec3 {
    FixedVec3::new(q(raw[0]), q(raw[1]), q(raw[2]))
}

/// Whole units, as raw.
fn u(units: Raw3) -> FixedVec3 {
    v(units.map(|c| c * ONE))
}

fn raw(p: FixedVec3) -> Raw3 {
    [p.x.raw(), p.y.raw(), p.z.raw()]
}

fn box_units(min: Raw3, max: Raw3) -> FixedAabb3 {
    FixedAabb3::new(u(min), u(max))
}

/// The ray from `(-10, 0, 0)` along `+x`.
fn x_ray() -> FixedRay3 {
    FixedRay3::new(u([-10, 0, 0]), FixedUnitVec3::UNIT_X)
}

/// `ray_intersects_aabb` (and its prepared form) reads an axis inverted under a nonzero
/// direction component as the swapped slab, but an axis inverted under a zero component as a
/// miss: the same box is a hit or a miss depending on the ray's direction.
#[wasm_bindgen_test(unsupported = test)]
fn degen_tp_020_characterization_ray_aabb_inverted_and_zero_size() {
    let ray = x_ray();
    let prepared = PreparedRay3::new(ray);
    let swapped = box_units([0, -1, -1], [1, 1, 1]);
    let inverted_x = box_units([1, -1, -1], [0, 1, 1]);
    let inverted_y = box_units([0, 1, -1], [1, -1, 1]);
    let hit = Some((q(10 * ONE), q(11 * ONE)));
    assert_eq!(ray_intersects_aabb(&ray, &swapped), hit);
    assert_eq!(ray_intersects_aabb(&ray, &inverted_x), hit, "swapped slab");
    assert_eq!(ray_intersects_aabb(&ray, &inverted_y), None, "zero component: a miss");
    assert_eq!(ray_intersects_aabb_prepared(&prepared, &inverted_x), hit);
    assert_eq!(ray_intersects_aabb_prepared(&prepared, &inverted_y), None);
    let point = box_units([0; 3], [0; 3]);
    assert_eq!(ray_intersects_aabb(&ray, &point), Some((q(10 * ONE), q(10 * ONE))));
}

/// `TraceRay::box_entry` misses an inverted box on every axis, including the axis where
/// `ray_intersects_aabb` reads it as the swapped slab. A zero-size box on the ray is entered.
#[wasm_bindgen_test(unsupported = test)]
fn degen_tp_021_characterization_box_entry_inverted_and_zero_size() {
    let ray = TraceRay::new(u([-10, 0, 0]), u([1, 0, 0])).expect("in the trace domain");
    let t_max = q(100 * ONE);
    assert_eq!(ray.box_entry(&box_units([0, -1, -1], [1, 1, 1]), t_max), Some(q(10 * ONE)));
    assert_eq!(ray.box_entry(&box_units([1, -1, -1], [0, 1, 1]), t_max), None);
    assert_eq!(ray.box_entry(&box_units([0, 1, -1], [1, -1, 1]), t_max), None);
    assert_eq!(ray.box_entry(&box_units([0; 3], [0; 3]), t_max), Some(q(10 * ONE)));
}

/// `ray_intersects_sphere` squares the radius, so a negative radius is read as its magnitude;
/// a zero radius is never hit, even by a ray through the centre (its normal cannot be
/// normalized).
#[wasm_bindgen_test(unsupported = test)]
fn degen_tp_022_characterization_ray_sphere_radius() {
    let ray = x_ray();
    let at = |r: i64| ray_intersects_sphere(&ray, &FixedSphere3::new(u([0; 3]), q(r)));
    let hit = Some((u([-1, 0, 0]), u([-1, 0, 0])));
    assert_eq!(at(ONE), hit);
    assert_eq!(at(-ONE), hit, "negative radius read as its magnitude");
    assert_eq!(at(0), None, "a point sphere on the ray is not hit");
}

/// Degenerate triangles are never hit: `ray_intersects_triangle` refuses a zero rounded
/// determinant, and `TraceTriangle::new` accepts the triangle while `hit_triangle` refuses its
/// exact zero determinant.
#[wasm_bindgen_test(unsupported = test)]
fn degen_tp_023_characterization_ray_degenerate_triangle() {
    let ray = FixedRay3::new(u([0, 0, -5]), FixedUnitVec3::UNIT_Z);
    let (a, b, c) = (u([-1, 0, 0]), u([0, 0, 0]), u([1, 0, 0]));
    assert_eq!(ray_intersects_triangle(&ray, a, b, c), None, "collinear");
    assert_eq!(ray_intersects_triangle(&ray, b, b, b), None, "one point");
    let trace = TraceRay::new(u([0, 0, -5]), u([0, 0, 1])).expect("in the trace domain");
    let collinear = TraceTriangle::new(a, b, c).expect("degenerate triangles are accepted");
    assert_eq!(trace.hit_triangle(&collinear, q(0), q(100 * ONE)), None);
    let point = TraceTriangle::new(b, b, b).expect("a point triangle is accepted");
    assert_eq!(trace.hit_triangle(&point, q(0), q(100 * ONE)), None);
}

/// `closest_point_aabb` clamps with `min` tested first, so on an inverted axis `[10, 0]` a
/// coordinate below 10 gives 10 and one above gives 0: the result is in neither the box nor
/// its swap's interior order. A zero-size box returns its point; the clamp is exact at the
/// ends of the raw range.
#[wasm_bindgen_test(unsupported = test)]
fn degen_tp_024_characterization_closest_point_aabb() {
    let inverted = box_units([10, 0, 0], [0, 0, 0]);
    assert_eq!(closest_point_aabb(&inverted, u([5, 0, 0])), u([10, 0, 0]));
    assert_eq!(closest_point_aabb(&inverted, u([-5, 0, 0])), u([10, 0, 0]));
    assert_eq!(closest_point_aabb(&inverted, u([20, 0, 0])), u([0, 0, 0]));
    let point = box_units([3; 3], [3; 3]);
    assert_eq!(closest_point_aabb(&point, u([-7, 9, 0])), u([3; 3]));
    let full = FixedAabb3::new(v([MIN; 3]), v([MAX; 3]));
    assert_eq!(closest_point_aabb(&full, v([MIN, 0, MAX])), v([MIN, 0, MAX]));
}

/// `aabb_intersects_sphere`: a negative radius is read as its magnitude, a zero radius is
/// point containment, and the saturating operators give a FALSE overlap at the ends of the raw
/// range: a sphere of radius 65 536 at `(MAX, MAX, MAX)` raw is reported as meeting the point
/// box at `(MIN, MIN, MIN)`, about `3.7e9` units away, because the squared distance and the
/// squared radius both saturate to `i64::MAX` raw.
#[wasm_bindgen_test(unsupported = test)]
fn degen_tp_025_characterization_aabb_sphere() {
    let segment_box = box_units([0; 3], [2, 0, 0]);
    let at = |centre: Raw3, r: i64| {
        aabb_intersects_sphere(&segment_box, &FixedSphere3::new(u(centre), q(r)))
    };
    assert_eq!(at([3, 0, 0], ONE), Some(u([2, 0, 0])));
    assert_eq!(at([3, 0, 0], -ONE), Some(u([2, 0, 0])), "negative radius read as magnitude");
    assert_eq!(at([3, 0, 0], 0), None);
    assert_eq!(at([2, 0, 0], 0), Some(u([2, 0, 0])), "a point sphere on the box");
    let inverted = box_units([10, 0, 0], [0, 0, 0]);
    let sphere = FixedSphere3::new(u([5, 0, 0]), q(ONE));
    assert_eq!(aabb_intersects_sphere(&inverted, &sphere), None);

    let far = FixedSphere3::new(v([MAX; 3]), q(ONE << 16));
    let corner = FixedAabb3::new(v([MIN; 3]), v([MIN; 3]));
    assert_eq!(aabb_intersects_sphere(&corner, &far), Some(v([MIN; 3])), "saturated: false hit");
}

/// `closest_point_triangle` on degenerate triangles. Collinear distinct vertices give the
/// true closest point on the segment they span. A repeated vertex does NOT: for the triangle
/// `(0, 0, 0)`, `(0, 0, 0)`, `(2, 0, 0)` and the point `(1, 1, 0)` it returns `(0, 0, 0)`, at
/// distance `sqrt(2)`, while `(1, 0, 0)` on the triangle is at distance 1 (the zero-length
/// edge `ab` is taken as its region and its parameter `0 / 0` falls back to zero).
#[wasm_bindgen_test(unsupported = test)]
fn degen_tp_026_characterization_closest_point_degenerate_triangle() {
    let (a, b, c) = (u([0; 3]), u([2, 0, 0]), u([1, 0, 0]));
    assert_eq!(closest_point_triangle(a, b, c, u([1, 1, 0])), u([1, 0, 0]));
    assert_eq!(closest_point_triangle(a, b, c, u([3, 1, 0])), u([2, 0, 0]));
    assert_eq!(closest_point_triangle(a, b, c, u([-1, 1, 0])), u([0; 3]));
    assert_eq!(closest_point_triangle(a, a, b, u([1, 1, 0])), u([0; 3]), "not the closest");
    assert_eq!(closest_point_triangle(a, a, b, u([3, 1, 0])), u([2, 0, 0]));
    assert_eq!(closest_point_triangle(a, a, a, u([5, 5, 5])), a);
    // Near the ends of the raw range the answer is within one raw unit of the exact one,
    // (-1/2, -1/2, 0) raw on the hypotenuse.
    let (lo, x_hi, y_hi) = (v([MIN, MIN, 0]), v([MAX, MIN, 0]), v([MIN, MAX, 0]));
    assert_eq!(raw(closest_point_triangle(lo, x_hi, y_hi, v([0, 0, ONE]))), [-1, -1, 0]);
}

/// `closest_points_segments` with zero-length segments returns the points themselves, or the
/// clamped projection of a point onto the other segment (rounded: here 2 raw units off the
/// exact `(3, 0, 0)`); parallel segments return one valid pair.
#[wasm_bindgen_test(unsupported = test)]
fn degen_tp_027_characterization_closest_points_degenerate_segments() {
    let (p, r) = (u([1, 1, 1]), u([5, 0, 0]));
    assert_eq!(closest_points_segments(p, p, r, r), (p, r));
    let (point, start, end) = (u([3, 1, 0]), u([0; 3]), u([10, 0, 0]));
    let (on_point, on_segment) = closest_points_segments(point, point, start, end);
    assert_eq!((on_point, raw(on_segment)), (point, [3 * ONE + 2, 0, 0]));
    assert_eq!(closest_points_segments(start, end, point, point), (u([0; 3]), point));
    let (c1, c2) = closest_points_segments(start, end, u([2, 1, 0]), u([12, 1, 0]));
    assert_eq!((raw(c1), c2), ([2 * ONE - 2, 0, 0], u([2, 1, 0])), "parallel");
    let full_x = (v([MIN, 0, 0]), v([MAX, 0, 0]));
    let full_y = (v([0, MIN, ONE]), v([0, MAX, ONE]));
    let (c1, c2) = closest_points_segments(full_x.0, full_x.1, full_y.0, full_y.1);
    assert_eq!((raw(c1), raw(c2)), ([-1, 0, 0], [0, -1, ONE]), "within a raw unit at the ends");
}

/// Trace-domain constructors: a zero direction is refused; a zero-size box is accepted and an
/// inverted one refused; a zero or negative radius is refused for spheres, discs and
/// cylinders (unlike `FixedSphere3::try_new`, which accepts zero); a zero-height cylinder is
/// refused; coordinates at `2^40` raw are outside the domain.
#[wasm_bindgen_test(unsupported = test)]
fn degen_tp_028_characterization_trace_constructors() {
    let origin = u([0; 3]);
    assert_eq!(TraceRay::new(origin, v([0; 3])), None);
    assert!(TraceRay::new(origin, v([1, 0, 0])).is_some(), "one raw unit is a direction");
    assert_eq!(TraceRay::new(v([TRACE_COORD_LIMIT_RAW, 0, 0]), u([1, 0, 0])), None);

    assert!(TraceBox::new(u([1; 3]), u([1; 3])).is_ok(), "zero-size accepted");
    assert_eq!(TraceBox::new(u([1, 0, 0]), u([0, 1, 1])), Err(TraceBoxError::MinimumAboveMaximum));
    assert_eq!(
        TraceBox::new(v([TRACE_COORD_LIMIT_RAW, 0, 0]), v([TRACE_COORD_LIMIT_RAW, 0, 0])),
        Err(TraceBoxError::CornerOutsideTraceDomain)
    );

    assert_eq!(TraceSphere::new(origin, q(0)), Err(TraceSphereError::RadiusNotPositive));
    assert_eq!(TraceSphere::new(origin, q(-1)), Err(TraceSphereError::RadiusNotPositive));
    assert!(TraceSphere::new(origin, q(1)).is_ok());
    assert_eq!(TraceDisc::new(q(0), q(0)), Err(TraceDiscError::RadiusNotPositive));
    assert_eq!(TraceCylinder::new(q(0), q(0), q(ONE)), Err(TraceCylinderError::RadiusNotPositive));
    assert_eq!(TraceCylinder::new(q(ONE), q(ONE), q(ONE)), Err(TraceCylinderError::CapsNotOrdered));
    assert_eq!(TraceCylinder::new(q(ONE), q(ONE), q(0)), Err(TraceCylinderError::CapsNotOrdered));
}

/// Shape frames: repeated axes are refused as not orthogonal, and a reflection as left-handed.
#[wasm_bindgen_test(unsupported = test)]
fn degen_tp_029_characterization_shape_frame() {
    let x = FixedUnitVec3::UNIT_X;
    let (y, z) = (FixedUnitVec3::UNIT_Y, FixedUnitVec3::UNIT_Z);
    assert_eq!(ShapeFrame::new([x, x, z], u([0; 3])), Err(ShapeFrameError::AxesNotOrthogonal));
    assert_eq!(ShapeFrame::new([y, x, z], u([0; 3])), Err(ShapeFrameError::AxesLeftHanded));
    assert!(ShapeFrame::translation(u([0; 3])).is_ok());
}
