//! The boundary of the beyond-reach guard of `SurfaceHit::new` and `SurfaceHit::try_new`: a hit
//! point one raw unit outside the trace domain can still step inside, and one two raw units
//! outside cannot, so both paths refuse it before the plane test.
//!
//! Every expected value here is hand computed with exact integers (the plane values are in the
//! comments); nothing is compared with the retained 0.7.1 code.

use bunny_linalg::FixedVec3;
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_optics::{point_at, SecondaryRayError, SurfaceHit, TriangleSurface};
use bunny_query::{TraceRay, TraceTriangle, TriangleHit, TRACE_COORD_LIMIT_RAW};
use wasm_bindgen_test::wasm_bindgen_test;

const ONE: i64 = ONE_RAW;
const LIMIT: i64 = TRACE_COORD_LIMIT_RAW;
const MAX: FixedQ32_32 = FixedQ32_32::from_raw(i64::MAX);

/// Every axis, each with the outward sign `+1` and `-1`.
const AXES_AND_SIGNS: [(usize, i64); 6] = [(0, 1), (0, -1), (1, 1), (1, -1), (2, 1), (2, -1)];

type Raw3 = [i64; 3];

fn fixed(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

fn vec3(raw: Raw3) -> FixedVec3 {
    FixedVec3::new(fixed(raw[0]), fixed(raw[1]), fixed(raw[2]))
}

fn triangle(v: [Raw3; 3]) -> TraceTriangle {
    TraceTriangle::new(vec3(v[0]), vec3(v[1]), vec3(v[2])).expect("in the trace domain")
}

/// The scene coordinates `[a, b, c]` in world coordinates: `a` on `axis`, `b` on the next axis
/// and `c` on the one after (a cyclic rotation, which keeps every cross product), each times
/// `sign`.
fn place(axis: usize, sign: i64, [a, b, c]: Raw3) -> Raw3 {
    let mut world = [0; 3];
    world[axis] = sign * a;
    world[(axis + 1) % 3] = sign * b;
    world[(axis + 2) % 3] = sign * c;
    world
}

/// The boundary scene for `axis` and outward `sign`, with a hit whose distance the test sets.
///
/// In scene coordinates (`a` outward along the axis, `b` along the next), the triangle lies in
/// the plane `2a + b = 2 LIMIT - 1`: its vertices `(LIMIT - 1, 1, -1u)`, `(LIMIT - 1, 1, 1u)` and
/// `(LIMIT - 1 - 1u, 1 + 2u, 0)` (u = one unit, other values raw) are inside the domain. Its
/// geometric normal `(v1 - v0) x (v2 - v0)` is `(-4, -2, 0) u^2`: the largest component is on
/// the axis, so every step is along it. For sign `+1` that normal points inward, so the front
/// face is the one that steps inward; negating every coordinate leaves the cross product as it
/// is, so for sign `-1` it points outward and the back face steps inward. Either way a point is
/// strictly on the arrival side exactly when `2a + b < 2 LIMIT - 1`.
///
/// The ray runs outward along the axis from the origin, so at distance `d` raw the hit point is
/// `a = d, b = c = 0` exactly (the direction is exactly one unit). The hit is the one the trace
/// kernel reports on a carrier wall at `a = 1u`, with its face set as above; only its distance
/// and face are read.
fn boundary_scene(axis: usize, sign: i64) -> (TriangleSurface, TraceRay, TriangleHit) {
    let at = |scene: Raw3| place(axis, sign, scene);
    let slope =
        [at([LIMIT - 1, 1, -ONE]), at([LIMIT - 1, 1, ONE]), at([LIMIT - 1 - ONE, 1 + 2 * ONE, 0])];
    let surface = TriangleSurface::new(&triangle(slope)).expect("not degenerate");
    let ray = TraceRay::new(vec3([0; 3]), vec3(at([ONE, 0, 0]))).expect("a valid trace ray");
    let wall = [at([ONE, -ONE, -ONE]), at([ONE, 2 * ONE, -ONE]), at([ONE, -ONE, 2 * ONE])];
    let mut hit = ray.hit_triangle(&triangle(wall), fixed(0), MAX).expect("crosses the wall");
    hit.front_face = sign == 1;
    (surface, ray, hit)
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_096_a_hit_point_one_raw_unit_outside_the_domain_steps_inside() {
    for (axis, sign) in AXES_AND_SIGNS {
        let context = format!("axis {axis}, sign {sign}");
        let (surface, ray, mut hit) = boundary_scene(axis, sign);
        hit.distance = fixed(LIMIT + 1);
        let point = vec3(place(axis, sign, [LIMIT + 1, 0, 0]));
        assert_eq!(point_at(&ray, hit.distance), Some(point), "{context}: hit point");
        // 2a + b at a = LIMIT + 1 and at one step, a = LIMIT, is 2 LIMIT + 2 and 2 LIMIT: neither
        // below 2 LIMIT - 1. Two steps, a = LIMIT - 1, give 2 LIMIT - 2: below, and inside.
        let start = vec3(place(axis, sign, [LIMIT - 1, 0, 0]));
        let fast = SurfaceHit::new(&surface, &ray, &hit);
        let fast = fast.unwrap_or_else(|| panic!("{context}: the fast path refused the hit"));
        assert_eq!(fast.point(), start, "{context}: start point");
        assert_eq!(fast.frame(), surface.facing_frame(hit.front_face), "{context}: frame");
        assert_eq!(SurfaceHit::try_new(&surface, &ray, &hit), Ok(fast), "{context}: diagnostic");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_097_a_hit_point_two_raw_units_outside_the_domain_is_refused_by_both_paths() {
    for (axis, sign) in AXES_AND_SIGNS {
        let context = format!("axis {axis}, sign {sign}");
        let (surface, ray, mut hit) = boundary_scene(axis, sign);
        hit.distance = fixed(LIMIT + 2);
        // Two steps reach only a = LIMIT, outside the domain: refused before the plane test.
        let point = vec3(place(axis, sign, [LIMIT + 2, 0, 0]));
        assert_eq!(point_at(&ray, hit.distance), Some(point), "{context}: hit point");
        assert_eq!(SurfaceHit::new(&surface, &ray, &hit), None, "{context}: fast path");
        let refusal = SurfaceHit::try_new(&surface, &ray, &hit);
        let expected = SecondaryRayError::HitPointOutsideTraceDomain { point };
        assert_eq!(refusal, Err(expected), "{context}: diagnostic path");
    }
}
