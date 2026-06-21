//! Coordinate-law convention tests.

use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const ONE_RAW: i64 = bunny_num::fixed_q32_32::ONE_RAW;

fn q32(units: i32) -> FixedQ32_32 {
    FixedQ32_32::from_raw(i64::from(units) * ONE_RAW)
}

fn vec3(x: i32, y: i32, z: i32) -> FixedVec3 {
    FixedVec3::new(q32(x), q32(y), q32(z))
}

#[wasm_bindgen_test(unsupported = test)]
fn cl_tp_001_canonical_basis_is_right_handed() {
    let unit_x = vec3(1, 0, 0);
    let unit_y = vec3(0, 1, 0);
    let unit_z = vec3(0, 0, 1);

    assert_eq!(unit_x.cross(unit_y), unit_z);
    assert_eq!(unit_y.cross(unit_z), unit_x);
    assert_eq!(unit_z.cross(unit_x), unit_y);

    assert_eq!(unit_y.cross(unit_x), -unit_z);
    assert_eq!(unit_z.cross(unit_y), -unit_x);
    assert_eq!(unit_x.cross(unit_z), -unit_y);
}

#[wasm_bindgen_test(unsupported = test)]
fn cl_tp_002_xy_counter_clockwise_winding_points_toward_positive_z() {
    let a = vec3(0, 0, 0);
    let b = vec3(1, 0, 0);
    let c = vec3(0, 1, 0);

    let counter_clockwise_normal = (b - a).cross(c - a);
    let clockwise_normal = (c - a).cross(b - a);

    assert_eq!(counter_clockwise_normal, vec3(0, 0, 1));
    assert_eq!(clockwise_normal, vec3(0, 0, -1));
}

#[wasm_bindgen_test(unsupported = test)]
fn cl_tp_003_bunny_units_are_unitless_fixed_raw_values() {
    let one_unit = q32(1);
    let coordinate = vec3(2, -3, 5);

    assert_eq!(one_unit.raw(), ONE_RAW);
    assert_eq!(coordinate.x.raw(), 2_i64 * ONE_RAW);
    assert_eq!(coordinate.y.raw(), -3_i64 * ONE_RAW);
    assert_eq!(coordinate.z.raw(), 5_i64 * ONE_RAW);
}
