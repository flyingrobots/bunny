use bunny_geom::{FixedAabb3, FixedSphere3};
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use bunny_query::{
    aabb_intersects_sphere, closest_point_aabb, closest_point_triangle, closest_points_segments,
};
use wasm_bindgen_test::wasm_bindgen_test;

const ONE_RAW: i64 = bunny_num::fixed_q32_32::ONE_RAW;
const HALF_RAW: i64 = ONE_RAW / 2;

fn q32(value: i32) -> FixedQ32_32 {
    FixedQ32_32::from_raw(i64::from(value) * ONE_RAW)
}

fn vec3(x: i32, y: i32, z: i32) -> FixedVec3 {
    FixedVec3::new(q32(x), q32(y), q32(z))
}

fn raw3(v: FixedVec3) -> [i64; 3] {
    [v.x.to_raw(), v.y.to_raw(), v.z.to_raw()]
}

#[wasm_bindgen_test(unsupported = test)]
fn closest_point_raw_q32_32_outputs() {
    let aabb = FixedAabb3::new(vec3(-1, -1, -1), vec3(1, 1, 1));
    assert_eq!(
        raw3(closest_point_aabb(&aabb, vec3(3, -2, 0))),
        [ONE_RAW, -ONE_RAW, 0]
    );

    let sphere = FixedSphere3::new(
        FixedVec3::new(FixedQ32_32::from_raw(ONE_RAW + HALF_RAW), q32(0), q32(0)),
        q32(1),
    );
    assert_eq!(
        aabb_intersects_sphere(&aabb, &sphere).map(raw3),
        Some([ONE_RAW, 0, 0])
    );

    let sphere_far = FixedSphere3::new(vec3(3, 0, 0), q32(1));
    assert_eq!(aabb_intersects_sphere(&aabb, &sphere_far), None);

    let v0 = vec3(-1, -1, 0);
    let v1 = vec3(1, -1, 0);
    let v2 = vec3(0, 1, 0);
    assert_eq!(
        raw3(closest_point_triangle(v0, v1, v2, vec3(0, 0, 2))),
        [0, 0, 0]
    );
    assert_eq!(
        raw3(closest_point_triangle(v0, v1, v2, vec3(0, 2, 0))),
        [0, ONE_RAW, 0]
    );
    assert_eq!(
        raw3(closest_point_triangle(
            v0,
            v1,
            v2,
            FixedVec3::new(FixedQ32_32::from_raw(HALF_RAW), q32(-2), q32(0)),
        )),
        [HALF_RAW, -ONE_RAW, 0]
    );

    let (parallel_a, parallel_b) =
        closest_points_segments(vec3(0, 0, 0), vec3(2, 0, 0), vec3(0, 2, 0), vec3(2, 2, 0));
    assert_eq!(raw3(parallel_a), [0, 0, 0]);
    assert_eq!(raw3(parallel_b), [0, 2 * ONE_RAW, 0]);

    let (crossing_a, crossing_b) =
        closest_points_segments(vec3(0, 0, 0), vec3(2, 0, 0), vec3(1, -1, 0), vec3(1, 1, 0));
    assert_eq!(raw3(crossing_a), [ONE_RAW, 0, 0]);
    assert_eq!(raw3(crossing_b), [ONE_RAW, 0, 0]);
}
