//! Integration tests.

use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const ONE_RAW: i64 = bunny_num::fixed_q32_32::ONE_RAW;
const DIAGONAL_UNIT_RAW: i64 = 3_037_000_500;
const DIAGONAL_XY: Option<FixedUnitVec3> = FixedUnitVec3::try_from_unit(FixedVec3::new(
    FixedQ32_32::from_raw(DIAGONAL_UNIT_RAW),
    FixedQ32_32::from_raw(DIAGONAL_UNIT_RAW),
    FixedQ32_32::ZERO,
));

fn q32(value: i32) -> FixedQ32_32 {
    FixedQ32_32::from_raw(i64::from(value) * ONE_RAW)
}

fn vec3(x: i32, y: i32, z: i32) -> FixedVec3 {
    FixedVec3::new(q32(x), q32(y), q32(z))
}

#[wasm_bindgen_test(unsupported = test)]
fn fixed_vector_raw_golden_values_are_stable() {
    let length = vec3(3, 4, 0).length().expect("3-4-5 length should compute");
    assert_eq!(length.to_raw(), 5 * ONE_RAW);

    let unit_z = FixedUnitVec3::UNIT_Z.into_inner();
    assert_eq!(unit_z, vec3(0, 0, 1));

    let diagonal = DIAGONAL_XY.expect("canonical diagonal proof should validate").into_inner();
    assert_eq!(diagonal.x.to_raw(), DIAGONAL_UNIT_RAW);
    assert_eq!(diagonal.y.to_raw(), DIAGONAL_UNIT_RAW);
    assert_eq!(diagonal.z.to_raw(), 0);
}
