//! Integration tests.

use bunny_geom::{FixedAabb3, FixedRay3, FixedSphere3};
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use bunny_query::{
    aabb_intersects_sphere, closest_point_aabb, closest_point_triangle, closest_points_segments,
    ray_intersects_aabb, ray_intersects_sphere, ray_intersects_triangle,
};
use wasm_bindgen_test::wasm_bindgen_test;

const ONE_RAW: i64 = bunny_num::fixed_q32_32::ONE_RAW;

fn raw(value: i32) -> i64 {
    i64::from(value) * ONE_RAW
}

fn assert_q32(value: FixedQ32_32, expected: i32) {
    assert_eq!(value.to_raw(), raw(expected));
}

#[wasm_bindgen_test(unsupported = test)]
fn test_ray_sphere_intersection() {
    let zero_vec = FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO);
    let sphere = FixedSphere3::new(
        FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::from_f32(5.0)),
        FixedQ32_32::from_f32(1.0),
    );

    // Hit center
    let ray1 = FixedRay3::try_new(
        zero_vec,
        FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ONE),
    )
    .unwrap();
    let (hit, normal) = ray_intersects_sphere(&ray1, &sphere).expect("should hit");
    assert_q32(hit.z, 4);
    assert_q32(normal.z, -1);

    // Miss
    let ray2 = FixedRay3::try_new(
        zero_vec,
        FixedVec3::new(FixedQ32_32::ONE, FixedQ32_32::ZERO, FixedQ32_32::ZERO),
    )
    .unwrap();
    assert!(ray_intersects_sphere(&ray2, &sphere).is_none());

    // Inside sphere
    let ray3 = FixedRay3::try_new(
        FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::from_f32(4.5)),
        FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ONE),
    )
    .unwrap();
    let (hit3, normal3) = ray_intersects_sphere(&ray3, &sphere).expect("should hit from inside");
    assert_q32(hit3.z, 6);
    assert_q32(normal3.z, 1);
}

#[wasm_bindgen_test(unsupported = test)]
fn test_ray_aabb_intersection() {
    let zero_vec = FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO);
    let aabb = FixedAabb3::new(
        FixedVec3::new(
            FixedQ32_32::from_f32(-1.0),
            FixedQ32_32::from_f32(-1.0),
            FixedQ32_32::from_f32(4.0),
        ),
        FixedVec3::new(
            FixedQ32_32::from_f32(1.0),
            FixedQ32_32::from_f32(1.0),
            FixedQ32_32::from_f32(6.0),
        ),
    );

    // Hit front
    let ray1 = FixedRay3::try_new(
        zero_vec,
        FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ONE),
    )
    .unwrap();
    let (t_enter, t_exit) = ray_intersects_aabb(&ray1, &aabb).expect("should hit");
    assert_q32(t_enter, 4);
    assert_q32(t_exit, 6);

    // Miss
    let ray2 = FixedRay3::try_new(
        zero_vec,
        FixedVec3::new(FixedQ32_32::from_f32(2.0), FixedQ32_32::ZERO, FixedQ32_32::ONE),
    )
    .unwrap();
    assert!(ray_intersects_aabb(&ray2, &aabb).is_none());
}

#[wasm_bindgen_test(unsupported = test)]
fn test_ray_triangle_intersection() {
    let zero_vec = FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO);
    let v0 = FixedVec3::new(
        FixedQ32_32::from_f32(-1.0),
        FixedQ32_32::from_f32(-1.0),
        FixedQ32_32::from_f32(5.0),
    );
    let v1 = FixedVec3::new(
        FixedQ32_32::from_f32(1.0),
        FixedQ32_32::from_f32(-1.0),
        FixedQ32_32::from_f32(5.0),
    );
    let v2 = FixedVec3::new(
        FixedQ32_32::from_f32(0.0),
        FixedQ32_32::from_f32(1.0),
        FixedQ32_32::from_f32(5.0),
    );

    // Hit inside
    let ray1 = FixedRay3::try_new(
        zero_vec,
        FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ONE),
    )
    .unwrap();
    let hit1 = ray_intersects_triangle(&ray1, v0, v1, v2).expect("should hit");
    assert_q32(hit1.z, 5);
    assert_q32(hit1.x, 0);
    assert_q32(hit1.y, 0);

    // Miss outside edges
    let ray2 = FixedRay3::try_new(
        zero_vec,
        FixedVec3::new(FixedQ32_32::from_f32(2.0), FixedQ32_32::ZERO, FixedQ32_32::ONE),
    )
    .unwrap();
    assert!(ray_intersects_triangle(&ray2, v0, v1, v2).is_none());
}

#[wasm_bindgen_test(unsupported = test)]
fn test_aabb_sphere_closest_point_and_overlap() {
    let aabb = FixedAabb3::new(
        FixedVec3::new(
            FixedQ32_32::from_f32(-1.0),
            FixedQ32_32::from_f32(-1.0),
            FixedQ32_32::from_f32(-1.0),
        ),
        FixedVec3::new(
            FixedQ32_32::from_f32(1.0),
            FixedQ32_32::from_f32(1.0),
            FixedQ32_32::from_f32(1.0),
        ),
    );

    // Center outside AABB, but sphere overlaps
    let sphere = FixedSphere3::new(
        FixedVec3::new(FixedQ32_32::from_f32(1.5), FixedQ32_32::ZERO, FixedQ32_32::ZERO),
        FixedQ32_32::from_f32(1.0),
    );
    let closest = aabb_intersects_sphere(&aabb, &sphere).expect("should overlap");
    assert_q32(closest.x, 1);
    assert_q32(closest.y, 0);
    assert_q32(closest.z, 0);

    // Sphere does not overlap
    let sphere_far = FixedSphere3::new(
        FixedVec3::new(FixedQ32_32::from_f32(2.5), FixedQ32_32::ZERO, FixedQ32_32::ZERO),
        FixedQ32_32::from_f32(1.0),
    );
    assert!(aabb_intersects_sphere(&aabb, &sphere_far).is_none());
    let closest_far = closest_point_aabb(&aabb, sphere_far.center);
    assert_q32(closest_far.x, 1);
}

#[wasm_bindgen_test(unsupported = test)]
fn test_closest_point_triangle() {
    let v0 =
        FixedVec3::new(FixedQ32_32::from_f32(-1.0), FixedQ32_32::from_f32(-1.0), FixedQ32_32::ZERO);
    let v1 =
        FixedVec3::new(FixedQ32_32::from_f32(1.0), FixedQ32_32::from_f32(-1.0), FixedQ32_32::ZERO);
    let v2 =
        FixedVec3::new(FixedQ32_32::from_f32(0.0), FixedQ32_32::from_f32(1.0), FixedQ32_32::ZERO);

    // Target point directly above face
    let p_face = FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::from_f32(2.0));
    let cp_face = closest_point_triangle(v0, v1, v2, p_face);
    assert_q32(cp_face.x, 0);
    assert_q32(cp_face.y, 0);
    assert_q32(cp_face.z, 0);

    // Target point closest to v2 vertex
    let p_vertex = FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::from_f32(2.0), FixedQ32_32::ZERO);
    let cp_vertex = closest_point_triangle(v0, v1, v2, p_vertex);
    assert_q32(cp_vertex.x, 0);
    assert_q32(cp_vertex.y, 1);
}

#[wasm_bindgen_test(unsupported = test)]
fn test_closest_points_segments() {
    // Two parallel segments:
    // Seg 1: (0,0,0) to (2,0,0)
    // Seg 2: (0,2,0) to (2,2,0)
    let p1 = FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO);
    let q1 = FixedVec3::new(FixedQ32_32::from_f32(2.0), FixedQ32_32::ZERO, FixedQ32_32::ZERO);
    let p2 = FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::from_f32(2.0), FixedQ32_32::ZERO);
    let q2 =
        FixedVec3::new(FixedQ32_32::from_f32(2.0), FixedQ32_32::from_f32(2.0), FixedQ32_32::ZERO);

    let (c1, c2) = closest_points_segments(p1, q1, p2, q2);
    assert_q32(c1.x, 0);
    assert_q32(c1.y, 0);
    assert_q32(c2.x, 0);
    assert_q32(c2.y, 2);

    // Intersecting segments
    let p3 = FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO);
    let q3 = FixedVec3::new(FixedQ32_32::from_f32(2.0), FixedQ32_32::ZERO, FixedQ32_32::ZERO);
    let p4 =
        FixedVec3::new(FixedQ32_32::from_f32(1.0), FixedQ32_32::from_f32(-1.0), FixedQ32_32::ZERO);
    let q4 =
        FixedVec3::new(FixedQ32_32::from_f32(1.0), FixedQ32_32::from_f32(1.0), FixedQ32_32::ZERO);
    let (c3, c4) = closest_points_segments(p3, q3, p4, q4);
    assert_q32(c3.x, 1);
    assert_q32(c3.y, 0);
    assert_q32(c4.x, 1);
    assert_q32(c4.y, 0);
}
