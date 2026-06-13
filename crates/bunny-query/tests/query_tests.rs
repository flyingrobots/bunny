use bunny_geom::{FixedAabb3, FixedRay3, FixedSphere3};
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use bunny_query::{ray_intersects_aabb, ray_intersects_sphere, ray_intersects_triangle};

#[test]
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

#[test]
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

#[test]
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
