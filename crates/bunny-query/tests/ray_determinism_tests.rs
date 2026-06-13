use bunny_geom::{FixedAabb3, FixedRay3, FixedSphere3};
use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::FixedQ32_32;
use bunny_query::{ray_intersects_aabb, ray_intersects_sphere, ray_intersects_triangle};
use wasm_bindgen_test::wasm_bindgen_test;

const RAY_DETERMINISM_CORPUS_SEED: u64 = 0xB002_0002_C0DE_0001;
const ONE_RAW: i64 = bunny_num::fixed_q32_32::ONE_RAW;

fn q32(value: i32) -> FixedQ32_32 {
    FixedQ32_32::from_raw(i64::from(value) * ONE_RAW)
}

fn vec3(x: i32, y: i32, z: i32) -> FixedVec3 {
    FixedVec3::new(q32(x), q32(y), q32(z))
}

fn raw3(v: FixedVec3) -> [i64; 3] {
    [v.x.to_raw(), v.y.to_raw(), v.z.to_raw()]
}

fn assert_sphere_raw(
    ray: &FixedRay3,
    sphere: &FixedSphere3,
    expected: Option<([i64; 3], [i64; 3])>,
) {
    let actual = ray_intersects_sphere(ray, sphere).map(|(hit, normal)| (raw3(hit), raw3(normal)));
    assert_eq!(actual, expected);
}

fn assert_aabb_raw(ray: &FixedRay3, aabb: &FixedAabb3, expected: Option<(i64, i64)>) {
    let actual =
        ray_intersects_aabb(ray, aabb).map(|(enter, exit)| (enter.to_raw(), exit.to_raw()));
    assert_eq!(actual, expected);
}

fn assert_triangle_raw(
    ray: &FixedRay3,
    v0: FixedVec3,
    v1: FixedVec3,
    v2: FixedVec3,
    expected: Option<[i64; 3]>,
) {
    let actual = ray_intersects_triangle(ray, v0, v1, v2).map(raw3);
    assert_eq!(actual, expected);
}

#[wasm_bindgen_test(unsupported = test)]
fn ray_query_raw_output_determinism_corpus() {
    assert_eq!(RAY_DETERMINISM_CORPUS_SEED, 0xB002_0002_C0DE_0001);

    let unit_z_ray = FixedRay3::new(vec3(0, 0, 0), FixedUnitVec3::UNIT_Z);
    let unit_x_ray = FixedRay3::new(vec3(0, 0, 0), FixedUnitVec3::UNIT_X);
    let tangent_z_ray = FixedRay3::new(vec3(1, 0, 0), FixedUnitVec3::UNIT_Z);
    let side_x_ray = FixedRay3::new(vec3(2, 0, 5), FixedUnitVec3::NEG_UNIT_X);
    let inside_sphere_ray = FixedRay3::new(vec3(0, 0, 5), FixedUnitVec3::UNIT_Z);

    let sphere = FixedSphere3::new(vec3(0, 0, 5), q32(1));
    let large_sphere = FixedSphere3::new(vec3(0, 0, 5), q32(2));
    assert_sphere_raw(
        &unit_z_ray,
        &sphere,
        Some(([0, 0, 4 * ONE_RAW], [0, 0, -ONE_RAW])),
    );
    assert_sphere_raw(&unit_x_ray, &sphere, None);
    assert_sphere_raw(
        &tangent_z_ray,
        &sphere,
        Some(([ONE_RAW, 0, 5 * ONE_RAW], [ONE_RAW, 0, 0])),
    );
    assert_sphere_raw(
        &inside_sphere_ray,
        &large_sphere,
        Some(([0, 0, 7 * ONE_RAW], [0, 0, ONE_RAW])),
    );

    let aabb = FixedAabb3::new(vec3(-1, -1, 4), vec3(1, 1, 6));
    assert_aabb_raw(&unit_z_ray, &aabb, Some((4 * ONE_RAW, 6 * ONE_RAW)));
    assert_aabb_raw(&unit_x_ray, &aabb, None);
    assert_aabb_raw(&side_x_ray, &aabb, Some((ONE_RAW, 3 * ONE_RAW)));

    let v0 = vec3(-1, -1, 5);
    let v1 = vec3(1, -1, 5);
    let v2 = vec3(0, 1, 5);
    let vertex_ray = FixedRay3::new(vec3(1, -1, 0), FixedUnitVec3::UNIT_Z);
    let outside_ray = FixedRay3::new(vec3(2, 0, 0), FixedUnitVec3::UNIT_Z);
    assert_triangle_raw(&unit_z_ray, v0, v1, v2, Some([0, 0, 5 * ONE_RAW]));
    assert_triangle_raw(
        &vertex_ray,
        v0,
        v1,
        v2,
        Some([ONE_RAW, -ONE_RAW, 5 * ONE_RAW]),
    );
    assert_triangle_raw(&outside_ray, v0, v1, v2, None);
}
