use bunny_geom::{FixedAabb3, FixedRay3, FixedSphere3};
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use bunny_query::{
    aabb_intersects_sphere, closest_point_aabb, closest_point_triangle, closest_points_segments,
    ray_intersects_aabb, ray_intersects_sphere, ray_intersects_triangle,
};
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test(unsupported = test)]
fn test_ray_sphere_intersection() {
    let zero_vec = FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO);
    let sphere = FixedSphere3::new(
        FixedVec3::new(
            FixedQ32_32::ZERO,
            FixedQ32_32::ZERO,
            FixedQ32_32::from_f32(5.0),
        ),
        FixedQ32_32::from_f32(1.0),
    );

    // Hit center
    let ray1 = FixedRay3::new(
        zero_vec,
        FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ONE),
    );
    let (hit, normal) = ray_intersects_sphere(&ray1, &sphere).expect("should hit");
    assert_eq!(hit.z.to_f32(), 4.0);
    assert_eq!(normal.z.to_f32(), -1.0);

    // Miss
    let ray2 = FixedRay3::new(
        zero_vec,
        FixedVec3::new(FixedQ32_32::ONE, FixedQ32_32::ZERO, FixedQ32_32::ZERO),
    );
    assert!(ray_intersects_sphere(&ray2, &sphere).is_none());

    // Inside sphere
    let ray3 = FixedRay3::new(
        FixedVec3::new(
            FixedQ32_32::ZERO,
            FixedQ32_32::ZERO,
            FixedQ32_32::from_f32(4.5),
        ),
        FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ONE),
    );
    let (hit3, normal3) = ray_intersects_sphere(&ray3, &sphere).expect("should hit from inside");
    assert_eq!(hit3.z.to_f32(), 6.0);
    assert_eq!(normal3.z.to_f32(), 1.0);
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
    let ray1 = FixedRay3::new(
        zero_vec,
        FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ONE),
    );
    let (t_enter, t_exit) = ray_intersects_aabb(&ray1, &aabb).expect("should hit");
    assert_eq!(t_enter.to_f32(), 4.0);
    assert_eq!(t_exit.to_f32(), 6.0);

    // Miss
    let ray2 = FixedRay3::new(
        zero_vec,
        FixedVec3::new(
            FixedQ32_32::from_f32(2.0),
            FixedQ32_32::ZERO,
            FixedQ32_32::ONE,
        ),
    );
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
    let ray1 = FixedRay3::new(
        zero_vec,
        FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ONE),
    );
    let hit1 = ray_intersects_triangle(&ray1, v0, v1, v2).expect("should hit");
    assert_eq!(hit1.z.to_f32(), 5.0);
    assert_eq!(hit1.x.to_f32(), 0.0);
    assert_eq!(hit1.y.to_f32(), 0.0);

    // Miss outside edges
    let ray2 = FixedRay3::new(
        zero_vec,
        FixedVec3::new(
            FixedQ32_32::from_f32(2.0),
            FixedQ32_32::ZERO,
            FixedQ32_32::ONE,
        ),
    );
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
        FixedVec3::new(
            FixedQ32_32::from_f32(1.5),
            FixedQ32_32::ZERO,
            FixedQ32_32::ZERO,
        ),
        FixedQ32_32::from_f32(1.0),
    );
    let closest = aabb_intersects_sphere(&aabb, &sphere).expect("should overlap");
    assert_eq!(closest.x.to_f32(), 1.0);
    assert_eq!(closest.y.to_f32(), 0.0);
    assert_eq!(closest.z.to_f32(), 0.0);

    // Sphere does not overlap
    let sphere_far = FixedSphere3::new(
        FixedVec3::new(
            FixedQ32_32::from_f32(2.5),
            FixedQ32_32::ZERO,
            FixedQ32_32::ZERO,
        ),
        FixedQ32_32::from_f32(1.0),
    );
    assert!(aabb_intersects_sphere(&aabb, &sphere_far).is_none());
    let closest_far = closest_point_aabb(&aabb, sphere_far.center);
    assert_eq!(closest_far.x.to_f32(), 1.0);
}

#[wasm_bindgen_test(unsupported = test)]
fn test_closest_point_triangle() {
    let v0 = FixedVec3::new(
        FixedQ32_32::from_f32(-1.0),
        FixedQ32_32::from_f32(-1.0),
        FixedQ32_32::ZERO,
    );
    let v1 = FixedVec3::new(
        FixedQ32_32::from_f32(1.0),
        FixedQ32_32::from_f32(-1.0),
        FixedQ32_32::ZERO,
    );
    let v2 = FixedVec3::new(
        FixedQ32_32::from_f32(0.0),
        FixedQ32_32::from_f32(1.0),
        FixedQ32_32::ZERO,
    );

    // Target point directly above face
    let p_face = FixedVec3::new(
        FixedQ32_32::ZERO,
        FixedQ32_32::ZERO,
        FixedQ32_32::from_f32(2.0),
    );
    let cp_face = closest_point_triangle(v0, v1, v2, p_face);
    assert_eq!(cp_face.x.to_f32(), 0.0);
    assert_eq!(cp_face.y.to_f32(), 0.0);
    assert_eq!(cp_face.z.to_f32(), 0.0);

    // Target point closest to v2 vertex
    let p_vertex = FixedVec3::new(
        FixedQ32_32::ZERO,
        FixedQ32_32::from_f32(2.0),
        FixedQ32_32::ZERO,
    );
    let cp_vertex = closest_point_triangle(v0, v1, v2, p_vertex);
    assert_eq!(cp_vertex.x.to_f32(), 0.0);
    assert_eq!(cp_vertex.y.to_f32(), 1.0);
}

#[wasm_bindgen_test(unsupported = test)]
fn test_closest_points_segments() {
    // Two parallel segments:
    // Seg 1: (0,0,0) to (2,0,0)
    // Seg 2: (0,2,0) to (2,2,0)
    let p1 = FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO);
    let q1 = FixedVec3::new(
        FixedQ32_32::from_f32(2.0),
        FixedQ32_32::ZERO,
        FixedQ32_32::ZERO,
    );
    let p2 = FixedVec3::new(
        FixedQ32_32::ZERO,
        FixedQ32_32::from_f32(2.0),
        FixedQ32_32::ZERO,
    );
    let q2 = FixedVec3::new(
        FixedQ32_32::from_f32(2.0),
        FixedQ32_32::from_f32(2.0),
        FixedQ32_32::ZERO,
    );

    let (c1, c2) = closest_points_segments(p1, q1, p2, q2);
    assert_eq!(c1.x.to_f32(), 0.0);
    assert_eq!(c1.y.to_f32(), 0.0);
    assert_eq!(c2.x.to_f32(), 0.0);
    assert_eq!(c2.y.to_f32(), 2.0);

    // Intersecting segments
    let p3 = FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO);
    let q3 = FixedVec3::new(
        FixedQ32_32::from_f32(2.0),
        FixedQ32_32::ZERO,
        FixedQ32_32::ZERO,
    );
    let p4 = FixedVec3::new(
        FixedQ32_32::from_f32(1.0),
        FixedQ32_32::from_f32(-1.0),
        FixedQ32_32::ZERO,
    );
    let q4 = FixedVec3::new(
        FixedQ32_32::from_f32(1.0),
        FixedQ32_32::from_f32(1.0),
        FixedQ32_32::ZERO,
    );
    let (c3, c4) = closest_points_segments(p3, q3, p4, q4);
    assert_eq!(c3.x.to_f32(), 1.0);
    assert_eq!(c3.y.to_f32(), 0.0);
    assert_eq!(c4.x.to_f32(), 1.0);
    assert_eq!(c4.y.to_f32(), 0.0);
}
