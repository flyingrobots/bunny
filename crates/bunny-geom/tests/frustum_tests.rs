//! Frustum classification of points, boxes and spheres, and plane extraction from a clip
//! matrix: hand-derived constructed cases. See `docs/topics/frustum/test-plan.md`.

use bunny_geom::{
    FixedAabb3, FixedFrustum, FixedPlane3, FixedSphere3, FrustumClass, FrustumSphereError,
};
use bunny_linalg::{FixedMat4, FixedVec3};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const UNIT: i64 = 1_i64 << 32;
const HALF: i64 = 1_i64 << 31;

use FrustumClass::{Inside, Intersecting, Outside};

fn raw(v: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(v)
}

fn pt(x: i64, y: i64, z: i64) -> FixedVec3 {
    FixedVec3::new(raw(x), raw(y), raw(z))
}

fn plane(nx: i64, ny: i64, nz: i64, offset: i64) -> FixedPlane3 {
    FixedPlane3::new(pt(nx, ny, nz), raw(offset))
}

fn bx(min: [i64; 3], max: [i64; 3]) -> FixedAabb3 {
    FixedAabb3::new(pt(min[0], min[1], min[2]), pt(max[0], max[1], max[2]))
}

fn ball(c: [i64; 3], r: i64) -> FixedSphere3 {
    FixedSphere3::new(pt(c[0], c[1], c[2]), raw(r))
}

/// The box frustum `-1 <= x <= 1`, `-1 <= y <= 1`, `-3 <= z <= -1`, unit normals.
fn cube() -> FixedFrustum {
    FixedFrustum::new([
        plane(UNIT, 0, 0, -UNIT),     // left: x + 1 >= 0
        plane(-UNIT, 0, 0, -UNIT),    // right: 1 - x >= 0
        plane(0, UNIT, 0, -UNIT),     // bottom: y + 1 >= 0
        plane(0, -UNIT, 0, -UNIT),    // top: 1 - y >= 0
        plane(0, 0, -UNIT, UNIT),     // near: -z - 1 >= 0
        plane(0, 0, UNIT, -3 * UNIT), // far: z + 3 >= 0
    ])
}

/// The view-space perspective frustum of `tan = 1`, aspect 1, near 1 and far 3, derived on
/// paper from the coordinate law's matrix: rows `r3 +- r0`, `r3 +- r1`, `r2`, `r3 - r2`.
fn pyramid() -> FixedFrustum {
    FixedFrustum::new([
        plane(UNIT, 0, -UNIT, 0),         // left: x - z >= 0
        plane(-UNIT, 0, -UNIT, 0),        // right: -x - z >= 0
        plane(0, UNIT, -UNIT, 0),         // bottom: y - z >= 0
        plane(0, -UNIT, -UNIT, 0),        // top: -y - z >= 0
        plane(0, 0, -3 * HALF, 3 * HALF), // near: -3/2 z - 3/2 >= 0
        plane(0, 0, HALF, -3 * HALF),     // far: z / 2 + 3/2 >= 0
    ])
}

/// A plane that has every point in front: zero normal, offset -1.
fn always_front() -> FixedPlane3 {
    plane(0, 0, 0, -UNIT)
}

/// A frustum whose only constraint is `p`, in the left slot.
fn only(p: FixedPlane3) -> FixedFrustum {
    let f = always_front();
    FixedFrustum::new([p, f, f, f, f, f])
}

/// The cube's six faces: (axis, face coordinate, outward direction).
const FACES: [(usize, i64, i64); 6] =
    [(0, -UNIT, -1), (0, UNIT, 1), (1, -UNIT, -1), (1, UNIT, 1), (2, -UNIT, 1), (2, -3 * UNIT, -1)];

/// The cube's centre, with one axis replaced.
fn centre_with(axis: usize, value: i64) -> [i64; 3] {
    let mut c = [0, 0, -2 * UNIT];
    c[axis] = value;
    c
}

/// A box over the inner half of the cube, with the given range on one axis (either order).
fn slab(axis: usize, a: i64, b: i64) -> FixedAabb3 {
    let mut min = [-HALF, -HALF, -5 * HALF];
    let mut max = [HALF, HALF, -3 * HALF];
    min[axis] = a.min(b);
    max[axis] = a.max(b);
    bx(min, max)
}

fn inverted(b: FixedAabb3) -> FixedAabb3 {
    FixedAabb3::new(b.max, b.min)
}

#[wasm_bindgen_test(unsupported = test)]
fn frustum_tp_001_planes_are_held_as_given_in_order() {
    let planes = cube().planes();
    let f = FixedFrustum::new(planes);
    assert_eq!(f.planes(), planes);
    assert_eq!([f.left, f.right, f.bottom, f.top, f.near, f.far], planes);
    // An empty six-plane set (x >= 1 and x <= -1) is accepted: every point is outside.
    let mut empty = cube();
    empty.left = plane(UNIT, 0, 0, UNIT);
    assert_eq!(empty.contains_point(pt(0, 0, -2 * UNIT)), Some(false));
    assert_eq!(empty.classify_aabb(slab(0, -UNIT, UNIT)), Some(Outside));
    // An unbounded one (every plane always in front) is accepted: everything is inside.
    let everything = FixedFrustum::new([always_front(); 6]);
    assert_eq!(everything.contains_point(pt(i64::MAX, i64::MIN, 0)), Some(true));
    assert_eq!(everything.classify_aabb(bx([i64::MIN; 3], [i64::MAX; 3])), Some(Inside));
    assert_eq!(everything.classify_sphere(ball([0; 3], i64::MAX)), Ok(Inside));
}

#[wasm_bindgen_test(unsupported = test)]
fn frustum_tp_002_boxes_inside_outside_intersecting_and_on_each_face() {
    let f = cube();
    assert_eq!(f.classify_aabb(slab(0, -HALF, HALF)), Some(Inside));
    assert_eq!(f.classify_aabb(bx([-UNIT, -UNIT, -3 * UNIT], [UNIT, UNIT, -UNIT])), Some(Inside));
    assert_eq!(f.classify_aabb(slab(0, 2 * UNIT, 3 * UNIT)), Some(Outside));
    assert_eq!(f.classify_aabb(slab(0, -2 * UNIT, 2 * UNIT)), Some(Intersecting));
    for (axis, c, s) in FACES {
        let case = |a: i64, b: i64| f.classify_aabb(slab(axis, a, b));
        let face = format!("axis {axis} face {c} outward {s}");
        // Outside, touching the face, one raw unit through it, one raw unit clear of it.
        assert_eq!(case(c, c + s * UNIT), Some(Outside), "{face}: touching from outside");
        assert_eq!(case(c - s, c + s * UNIT), Some(Intersecting), "{face}: one raw through");
        assert_eq!(case(c + s, c + s * UNIT), Some(Outside), "{face}: one raw clear");
        // Inside, touching the face, one raw unit past it, one raw unit short of it.
        assert_eq!(case(c - s * UNIT, c), Some(Inside), "{face}: touching from inside");
        assert_eq!(case(c - s * UNIT, c + s), Some(Intersecting), "{face}: one raw past");
        assert_eq!(case(c - s * UNIT, c - s), Some(Inside), "{face}: one raw short");
        // Points on the face, one raw unit outside and one inside.
        let point = |v: i64| {
            let p = centre_with(axis, v);
            f.contains_point(pt(p[0], p[1], p[2]))
        };
        assert_eq!(point(c), Some(true), "{face}: point on the face");
        assert_eq!(point(c + s), Some(false), "{face}: point one raw outside");
        assert_eq!(point(c - s), Some(true), "{face}: point one raw inside");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn frustum_tp_003_flat_inverted_and_zero_extent_boxes() {
    let f = cube();
    // A flat box lying in the face x = 1 is Coplanar with that plane: inside.
    assert_eq!(f.classify_aabb(slab(0, UNIT, UNIT)), Some(Inside));
    assert_eq!(f.classify_aabb(slab(0, UNIT + 1, UNIT + 1)), Some(Outside));
    assert_eq!(f.classify_aabb(slab(2, -UNIT, -UNIT)), Some(Inside));
    assert_eq!(f.classify_aabb(slab(2, -UNIT + 1, -UNIT + 1)), Some(Outside));
    // An inverted box is the box with the same eight corners.
    for (axis, c, s) in FACES {
        for (a, b) in
            [(c, c + s * UNIT), (c - s, c + s * UNIT), (c - s * UNIT, c), (c - s * UNIT, c + s)]
        {
            let b = slab(axis, a, b);
            assert_eq!(f.classify_aabb(inverted(b)), f.classify_aabb(b), "inverted {b:?}");
        }
    }
    // A zero-extent box is one point and gets the point rule's answer.
    for (axis, c, s) in FACES {
        for v in [c - s, c, c + s] {
            let p = centre_with(axis, v);
            let point = pt(p[0], p[1], p[2]);
            let want = f.contains_point(point).map(|inside| if inside { Inside } else { Outside });
            assert_eq!(f.classify_aabb(FixedAabb3::new(point, point)), want, "point {point:?}");
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn frustum_tp_004_the_six_plane_box_test_is_conservative() {
    // Near the edge where the left plane x = z meets the far plane z = -3, the box
    // x in [-5, -7/2], y in [-1/2, 1/2], z in [-4, -2] straddles both planes and is in front
    // of the other four, so the six-plane test reports Intersecting. It meets no point of
    // the frustum: a frustum point has z >= -3 and x >= z >= -3, but the box has x <= -7/2.
    let f = pyramid();
    let b = bx([-5 * UNIT, -HALF, -4 * UNIT], [-7 * HALF, HALF, -2 * UNIT]);
    assert_eq!(f.classify_aabb(b), Some(Intersecting));
    assert_eq!(only(f.left).classify_aabb(b), Some(Intersecting));
    assert_eq!(only(f.far).classify_aabb(b), Some(Intersecting));
    for p in [f.right, f.bottom, f.top, f.near] {
        assert_eq!(only(p).classify_aabb(b), Some(Inside));
    }
    // Supporting evidence: no point of a 13 x 5 x 9 grid over the box is inside.
    for i in 0..=12 {
        for j in 0..=4 {
            for k in 0..=8 {
                let p =
                    pt(-5 * UNIT + i * UNIT / 8, -HALF + j * UNIT / 4, -4 * UNIT + k * UNIT / 4);
                assert_eq!(f.contains_point(p), Some(false), "grid point {p:?}");
            }
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn frustum_tp_005_spheres_inside_outside_intersecting_and_on_each_face() {
    let f = cube();
    assert_eq!(f.classify_sphere(ball([0, 0, -2 * UNIT], HALF)), Ok(Inside));
    // Radius 1 touches all six faces from inside; one raw unit more crosses them.
    assert_eq!(f.classify_sphere(ball([0, 0, -2 * UNIT], UNIT)), Ok(Inside));
    assert_eq!(f.classify_sphere(ball([0, 0, -2 * UNIT], UNIT + 1)), Ok(Intersecting));
    assert_eq!(f.classify_sphere(ball([5 * UNIT, 0, -2 * UNIT], UNIT)), Ok(Outside));
    for (axis, c, s) in FACES {
        let case = |centre: i64| f.classify_sphere(ball(centre_with(axis, centre), HALF));
        let face = format!("axis {axis} face {c} outward {s}");
        let outside = c + s * HALF;
        assert_eq!(case(outside), Ok(Outside), "{face}: touching from outside");
        assert_eq!(case(outside - s), Ok(Intersecting), "{face}: one raw through");
        assert_eq!(case(outside + s), Ok(Outside), "{face}: one raw clear");
        let inside = c - s * HALF;
        assert_eq!(case(inside), Ok(Inside), "{face}: touching from inside");
        assert_eq!(case(inside + s), Ok(Intersecting), "{face}: one raw past");
        assert_eq!(case(inside - s), Ok(Inside), "{face}: one raw short");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn frustum_tp_006_sphere_radius_rules_and_non_unit_normals() {
    // The plane x - z >= 0 has |n| = sqrt 2; the centre (0, 0, -2) has scaled distance 2,
    // true distance sqrt 2, which is irrational: the largest raw radius that stays in front
    // is floor(sqrt 2 * 2^32) and one raw unit more straddles.
    let sqrt2 = i64::try_from((2_u128 << 64).isqrt()).unwrap();
    assert_eq!(sqrt2, 6_074_000_999);
    for scale in [1, 3] {
        let f = only(plane(scale * UNIT, 0, -scale * UNIT, 0));
        assert_eq!(f.classify_sphere(ball([0, 0, -2 * UNIT], sqrt2)), Ok(Inside), "scale {scale}");
        assert_eq!(f.classify_sphere(ball([0, 0, -2 * UNIT], sqrt2 + 1)), Ok(Intersecting));
        assert_eq!(f.classify_sphere(ball([0, 0, 2 * UNIT], sqrt2)), Ok(Outside));
        assert_eq!(f.classify_sphere(ball([0, 0, 2 * UNIT], sqrt2 + 1)), Ok(Intersecting));
    }
    // A zero radius is the point rule, including the far face's rounding tie (FRUSTUM-TP-010).
    let f = pyramid();
    for p in [
        [0, 0, -UNIT],
        [0, 0, -UNIT + 1],
        [0, 0, -3 * UNIT - 1],
        [0, 0, -3 * UNIT - 2],
        [-2 * UNIT - 1, 0, -2 * UNIT],
    ] {
        let want =
            f.contains_point(pt(p[0], p[1], p[2]))
                .map(|inside| if inside { Inside } else { Outside });
        assert_eq!(f.classify_sphere(ball(p, 0)).ok(), want, "zero radius at {p:?}");
    }
    // A negative radius is refused, wherever the centre is and whatever the planes give.
    assert_eq!(
        f.classify_sphere(ball([0, 0, -2 * UNIT], -1)),
        Err(FrustumSphereError::NegativeRadius)
    );
    assert_eq!(
        f.classify_sphere(ball([i64::MAX; 3], i64::MIN)),
        Err(FrustumSphereError::NegativeRadius)
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn frustum_tp_007_the_six_plane_sphere_test_is_conservative() {
    // The sphere at (-4, 0, -4) with radius 5/4 is at true distance 0 from the left plane
    // and 1 from the far plane, so it straddles both and the test reports Intersecting.
    // The frustum point nearest its centre is (-3, 0, -3), at distance sqrt 2 > 5/4.
    let f = pyramid();
    let s = ball([-4 * UNIT, 0, -4 * UNIT], 5 * UNIT / 4);
    assert_eq!(f.classify_sphere(s), Ok(Intersecting));
    assert_eq!(only(f.left).classify_sphere(s), Ok(Intersecting));
    assert_eq!(only(f.far).classify_sphere(s), Ok(Intersecting));
}

#[wasm_bindgen_test(unsupported = test)]
fn frustum_tp_008_a_certain_outside_wins_over_overflow() {
    // The left slot holds z * MAX >= 0, whose distance overflows for |z| > 1.
    let mut f = cube();
    f.left = plane(0, 0, i64::MAX, 0);
    let outside_right = pt(2 * UNIT, 0, -2 * UNIT);
    let inside = pt(0, 0, -2 * UNIT);
    assert_eq!(f.contains_point(outside_right), Some(false));
    assert_eq!(f.contains_point(inside), None);
    assert_eq!(f.classify_aabb(slab(0, 2 * UNIT, 3 * UNIT)), Some(Outside));
    assert_eq!(f.classify_aabb(slab(0, -HALF, HALF)), None);
    assert_eq!(f.classify_aabb(slab(0, -2 * UNIT, 2 * UNIT)), None);
    assert_eq!(f.classify_sphere(ball([3 * UNIT, 0, -2 * UNIT], HALF)), Ok(Outside));
    assert_eq!(f.classify_sphere(ball([0, 0, -2 * UNIT], HALF)), Err(FrustumSphereError::Overflow));
    // The widest sphere comparisons do not overflow: R = MAX against |n|^2 = 3 * 2^126.
    let wide = only(plane(i64::MIN, i64::MIN, i64::MIN, 0));
    assert_eq!(wide.classify_sphere(ball([0; 3], i64::MAX)), Ok(Intersecting));
    let far_front = only(plane(1, 0, 0, -i64::MAX));
    assert_eq!(far_front.classify_sphere(ball([0; 3], i64::MAX)), Ok(Inside));
    let far_behind = only(plane(1, 0, 0, i64::MAX));
    assert_eq!(far_behind.classify_sphere(ball([0; 3], i64::MAX)), Ok(Outside));
}

fn mat(rows: [[i64; 4]; 4]) -> FixedMat4 {
    let row = |r: [i64; 4]| (raw(r[0]), raw(r[1]), raw(r[2]), raw(r[3]));
    FixedMat4::from_rows(row(rows[0]), row(rows[1]), row(rows[2]), row(rows[3]))
}

#[wasm_bindgen_test(unsupported = test)]
fn frustum_tp_009_extracted_planes_are_exact_row_sums() {
    let one = raw(UNIT);
    let perspective = FixedMat4::try_perspective(one, one, one, raw(3 * UNIT)).unwrap();
    assert_eq!(FixedFrustum::from_clip_matrix(perspective), Some(pyramid()));
    let ortho = FixedMat4::try_orthographic(raw(2 * UNIT), one, one, raw(5 * UNIT)).unwrap();
    let q = UNIT / 4;
    assert_eq!(
        FixedFrustum::from_clip_matrix(ortho).unwrap().planes(),
        [
            plane(HALF, 0, 0, -UNIT),
            plane(-HALF, 0, 0, -UNIT),
            plane(0, UNIT, 0, -UNIT),
            plane(0, -UNIT, 0, -UNIT),
            plane(0, 0, -q, q),
            plane(0, 0, q, -5 * q),
        ]
    );
    // The identity's frustum is the NDC volume: |x| <= 1, |y| <= 1, 0 <= z <= 1.
    assert_eq!(
        FixedFrustum::from_clip_matrix(FixedMat4::IDENTITY).unwrap().planes(),
        [
            plane(UNIT, 0, 0, -UNIT),
            plane(-UNIT, 0, 0, -UNIT),
            plane(0, UNIT, 0, -UNIT),
            plane(0, -UNIT, 0, -UNIT),
            plane(0, 0, UNIT, 0),
            plane(0, 0, -UNIT, -UNIT),
        ]
    );
    // clip_from_world for a camera at (0, 0, 5) looking at the origin: the pyramid moved
    // by +5 in z (x >= z - 5, near z <= 4, far z >= 2).
    let view = FixedMat4::try_look_at(pt(0, 0, 5 * UNIT), pt(0, 0, 0), pt(0, UNIT, 0)).unwrap();
    let clip_from_world = perspective.checked_mul_mat4(view).unwrap();
    assert_eq!(
        FixedFrustum::from_clip_matrix(clip_from_world).unwrap().planes(),
        [
            plane(UNIT, 0, -UNIT, -5 * UNIT),
            plane(-UNIT, 0, -UNIT, -5 * UNIT),
            plane(0, UNIT, -UNIT, -5 * UNIT),
            plane(0, -UNIT, -UNIT, -5 * UNIT),
            plane(0, 0, -3 * HALF, -6 * UNIT),
            plane(0, 0, HALF, UNIT),
        ]
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn frustum_tp_010_extracted_faces_one_raw_unit_either_side_and_overflow() {
    let one = raw(UNIT);
    let f = FixedFrustum::from_clip_matrix(
        FixedMat4::try_perspective(one, one, one, raw(3 * UNIT)).unwrap(),
    )
    .unwrap();
    let at = |x: i64, y: i64, z: i64| f.contains_point(pt(x, y, z));
    // Side faces have integer normals: every product is exact, so each face is exact.
    for (x, y) in [(-2 * UNIT, 0), (2 * UNIT, 0), (0, -2 * UNIT), (0, 2 * UNIT)] {
        let (sx, sy) = (x.signum(), y.signum());
        assert_eq!(at(x, y, -2 * UNIT), Some(true), "on the face at ({x}, {y})");
        assert_eq!(at(x + sx, y + sy, -2 * UNIT), Some(false), "one raw outside ({x}, {y})");
        assert_eq!(at(x - sx, y - sy, -2 * UNIT), Some(true), "one raw inside ({x}, {y})");
    }
    // Near face, normal z -3/2: (-3/2) * (-1 -+ 2^-32) is 3/2 +- 3/2 raw, a tie that rounds
    // to even (+-2 raw), so both neighbours keep their side.
    assert_eq!(at(0, 0, -UNIT), Some(true));
    assert_eq!(at(0, 0, -UNIT - 1), Some(true));
    assert_eq!(at(0, 0, -UNIT + 1), Some(false));
    // Far face, normal z 1/2: (1/2) * (-3 - 2^-32) is -3/2 - 1/2 raw, a tie that rounds to
    // even, -3/2 exactly: one raw unit beyond the far face is still On, so inside. Two
    // raw units beyond is -1 raw, outside. Faces are not exact under rounding.
    assert_eq!(at(0, 0, -3 * UNIT), Some(true));
    assert_eq!(at(0, 0, -3 * UNIT + 1), Some(true));
    assert_eq!(at(0, 0, -3 * UNIT - 1), Some(true));
    assert_eq!(at(0, 0, -3 * UNIT - 2), Some(false));
    // Overflow: a row sum one raw unit past i64, or -d of i64::MIN, is None.
    let max = i64::MAX;
    let base = [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 0]];
    let with = |r: usize, c: usize, v: i64, w: i64| {
        let mut rows = base;
        rows[r][c] = v;
        rows[3][c] = w;
        FixedFrustum::from_clip_matrix(mat(rows))
    };
    assert!(with(0, 0, 0, max).is_some());
    assert!(with(0, 0, 1, max).is_none(), "r3 + r0 overflows");
    assert!(with(0, 0, -1, max).is_none(), "r3 - r0 overflows");
    assert!(with(1, 1, -1, max - 1).is_some());
    assert!(with(1, 1, -1, max).is_none(), "r3 - r1 overflows");
    assert!(with(2, 3, i64::MIN + 1, 0).is_some());
    assert!(with(2, 3, i64::MIN, 0).is_none(), "the near offset -d and r3 - r2 overflow");
    assert!(with(2, 2, -1, i64::MIN + 1).is_some());
    assert!(with(2, 2, 1, i64::MIN).is_none(), "r3 - r2 overflows");
}
