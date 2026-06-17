//! Integration tests.

use bunny_geom::{FixedAabb3, FixedRay3, FixedSphere3};
use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::FixedQ32_32;
use bunny_query::{closest_point_aabb, ray_intersects_aabb, ray_intersects_sphere};
use wasm_bindgen_test::wasm_bindgen_test;

const ONE_RAW: i64 = bunny_num::fixed_q32_32::ONE_RAW;

fn q32(value: i32) -> FixedQ32_32 {
    FixedQ32_32::from_raw(i64::from(value) * ONE_RAW)
}

fn vec3(x: i32, y: i32, z: i32) -> FixedVec3 {
    FixedVec3::new(q32(x), q32(y), q32(z))
}

fn raw3(value: FixedVec3) -> [i64; 3] {
    [value.x.to_raw(), value.y.to_raw(), value.z.to_raw()]
}

#[wasm_bindgen_test(unsupported = test)]
fn query_raw_output_golden_vectors_are_stable() {
    let ray = FixedRay3::new(vec3(0, 0, 0), FixedUnitVec3::UNIT_Z);
    let sphere = FixedSphere3::new(vec3(0, 0, 5), q32(1));
    let sphere_hit =
        ray_intersects_sphere(&ray, &sphere).map(|(hit, normal)| (raw3(hit), raw3(normal)));
    assert_eq!(sphere_hit, Some(([0, 0, 4 * ONE_RAW], [0, 0, -ONE_RAW])));

    let aabb = FixedAabb3::new(vec3(-1, -1, 4), vec3(1, 1, 6));
    let aabb_hit =
        ray_intersects_aabb(&ray, &aabb).map(|(enter, exit)| (enter.to_raw(), exit.to_raw()));
    assert_eq!(aabb_hit, Some((4 * ONE_RAW, 6 * ONE_RAW)));

    let closest = closest_point_aabb(&aabb, vec3(3, -2, 5));
    assert_eq!(raw3(closest), [ONE_RAW, -ONE_RAW, 5 * ONE_RAW]);
}
