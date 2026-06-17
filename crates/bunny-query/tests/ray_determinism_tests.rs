//! Integration tests.

use bunny_geom::{FixedAabb3, FixedRay3, FixedSphere3};
use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::FixedQ32_32;
use bunny_query::{ray_intersects_aabb, ray_intersects_sphere, ray_intersects_triangle};
use wasm_bindgen_test::wasm_bindgen_test;

const RAY_DETERMINISM_CORPUS_SEED: u64 = 0xB002_0002_C0DE_0001;
const ONE_RAW: i64 = bunny_num::fixed_q32_32::ONE_RAW;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SeededAxisCorpus {
    sphere_center: i32,
    aabb_min: i32,
    aabb_max: i32,
    triangle_plane: i32,
}

fn q32(value: i32) -> FixedQ32_32 {
    FixedQ32_32::from_raw(i64::from(value) * ONE_RAW)
}

fn vec3(x: i32, y: i32, z: i32) -> FixedVec3 {
    FixedVec3::new(q32(x), q32(y), q32(z))
}

fn raw(value: i32) -> i64 {
    i64::from(value) * ONE_RAW
}

fn raw3(v: FixedVec3) -> [i64; 3] {
    [v.x.to_raw(), v.y.to_raw(), v.z.to_raw()]
}

fn next_seed(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state
}

fn bounded_offset(state: &mut u64, bound: u32) -> i32 {
    let value = next_seed(state) % u64::from(bound);
    i32::try_from(value).expect("bounded corpus value should fit i32")
}

fn seeded_axis_corpus(seed: u64) -> SeededAxisCorpus {
    let mut state = seed;
    let sphere_center = 4 + bounded_offset(&mut state, 3);
    let aabb_min = 2 + bounded_offset(&mut state, 3);
    let aabb_max = aabb_min + 2 + bounded_offset(&mut state, 2);
    let triangle_plane = 3 + bounded_offset(&mut state, 4);

    SeededAxisCorpus { sphere_center, aabb_min, aabb_max, triangle_plane }
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

fn assert_triangle_raw(ray: &FixedRay3, triangle: [FixedVec3; 3], expected: Option<[i64; 3]>) {
    let [v0, v1, v2] = triangle;
    let actual = ray_intersects_triangle(ray, v0, v1, v2).map(raw3);
    assert_eq!(actual, expected);
}

#[wasm_bindgen_test(unsupported = test)]
fn ray_query_raw_output_determinism_corpus() {
    let forward_ray = FixedRay3::new(vec3(0, 0, 0), FixedUnitVec3::UNIT_Z);
    let horizontal_ray = FixedRay3::new(vec3(0, 0, 0), FixedUnitVec3::UNIT_X);
    let tangent_z_ray = FixedRay3::new(vec3(1, 0, 0), FixedUnitVec3::UNIT_Z);
    let side_x_ray = FixedRay3::new(vec3(2, 0, 5), FixedUnitVec3::NEG_UNIT_X);
    let inside_sphere_ray = FixedRay3::new(vec3(0, 0, 5), FixedUnitVec3::UNIT_Z);

    let sphere = FixedSphere3::new(vec3(0, 0, 5), q32(1));
    let large_sphere = FixedSphere3::new(vec3(0, 0, 5), q32(2));
    assert_sphere_raw(&forward_ray, &sphere, Some(([0, 0, 4 * ONE_RAW], [0, 0, -ONE_RAW])));
    assert_sphere_raw(&horizontal_ray, &sphere, None);
    assert_sphere_raw(&tangent_z_ray, &sphere, Some(([ONE_RAW, 0, 5 * ONE_RAW], [ONE_RAW, 0, 0])));
    assert_sphere_raw(
        &inside_sphere_ray,
        &large_sphere,
        Some(([0, 0, 7 * ONE_RAW], [0, 0, ONE_RAW])),
    );

    let aabb = FixedAabb3::new(vec3(-1, -1, 4), vec3(1, 1, 6));
    assert_aabb_raw(&forward_ray, &aabb, Some((4 * ONE_RAW, 6 * ONE_RAW)));
    assert_aabb_raw(&horizontal_ray, &aabb, None);
    assert_aabb_raw(&side_x_ray, &aabb, Some((ONE_RAW, 3 * ONE_RAW)));

    let v0 = vec3(-1, -1, 5);
    let v1 = vec3(1, -1, 5);
    let v2 = vec3(0, 1, 5);
    let vertex_ray = FixedRay3::new(vec3(1, -1, 0), FixedUnitVec3::UNIT_Z);
    let outside_ray = FixedRay3::new(vec3(2, 0, 0), FixedUnitVec3::UNIT_Z);
    assert_triangle_raw(&forward_ray, [v0, v1, v2], Some([0, 0, 5 * ONE_RAW]));
    assert_triangle_raw(&vertex_ray, [v0, v1, v2], Some([ONE_RAW, -ONE_RAW, 5 * ONE_RAW]));
    assert_triangle_raw(&outside_ray, [v0, v1, v2], None);

    let seeded = seeded_axis_corpus(RAY_DETERMINISM_CORPUS_SEED);
    assert_eq!(
        seeded,
        SeededAxisCorpus { sphere_center: 4, aabb_min: 2, aabb_max: 4, triangle_plane: 4 }
    );

    let seeded_sphere = FixedSphere3::new(vec3(0, 0, seeded.sphere_center), q32(1));
    assert_sphere_raw(
        &forward_ray,
        &seeded_sphere,
        Some(([0, 0, raw(seeded.sphere_center - 1)], [0, 0, -ONE_RAW])),
    );

    let seeded_aabb = FixedAabb3::new(vec3(-1, -1, seeded.aabb_min), vec3(1, 1, seeded.aabb_max));
    assert_aabb_raw(&forward_ray, &seeded_aabb, Some((raw(seeded.aabb_min), raw(seeded.aabb_max))));

    assert_triangle_raw(
        &forward_ray,
        [
            vec3(-1, -1, seeded.triangle_plane),
            vec3(1, -1, seeded.triangle_plane),
            vec3(0, 1, seeded.triangle_plane),
        ],
        Some([0, 0, raw(seeded.triangle_plane)]),
    );
}
