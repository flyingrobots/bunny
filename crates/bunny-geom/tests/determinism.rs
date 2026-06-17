//! Integration tests.

use bunny_geom::{FixedAabb3, FixedRay3, FixedSphere3, GeomError};
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const ONE_RAW: i64 = bunny_num::fixed_q32_32::ONE_RAW;

fn q32(value: i32) -> FixedQ32_32 {
    FixedQ32_32::from_raw(i64::from(value) * ONE_RAW)
}

fn vec3(x: i32, y: i32, z: i32) -> FixedVec3 {
    FixedVec3::new(q32(x), q32(y), q32(z))
}

#[wasm_bindgen_test(unsupported = test)]
fn fixed_geometry_degenerate_boundaries_are_stable() {
    let inverted_aabb = FixedAabb3::try_new(vec3(1, 0, 0), vec3(0, 0, 0));
    assert_eq!(inverted_aabb, Err(GeomError::InvalidAabbBounds));

    let zero_sphere =
        FixedSphere3::try_new(vec3(0, 0, 0), FixedQ32_32::ZERO).expect("zero radius is valid");
    assert_eq!(zero_sphere.center, vec3(0, 0, 0));
    assert_eq!(zero_sphere.radius.to_raw(), 0);

    let zero_direction = FixedRay3::try_new(vec3(0, 0, 0), vec3(0, 0, 0));
    assert_eq!(zero_direction, Err(GeomError::InvalidRayDirection));
}
