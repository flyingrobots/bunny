//! Deterministic affine transform contract tests for Bunny fixed-point linear algebra.

use bunny_linalg::{FixedAffine2, FixedAffine3, FixedMat2, FixedMat3, FixedVec2, FixedVec3};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const ONE_RAW: i64 = bunny_num::fixed_q32_32::ONE_RAW;

fn q32(value: i32) -> FixedQ32_32 {
    FixedQ32_32::from_raw(i64::from(value) * ONE_RAW)
}

fn vec2(x: i32, y: i32) -> FixedVec2 {
    FixedVec2::new(q32(x), q32(y))
}

fn vec3(x: i32, y: i32, z: i32) -> FixedVec3 {
    FixedVec3::new(q32(x), q32(y), q32(z))
}

fn scale2(x: i32, y: i32) -> FixedMat2 {
    FixedMat2::new(q32(x), FixedQ32_32::ZERO, FixedQ32_32::ZERO, q32(y))
}

fn scale3(x: i32, y: i32, z: i32) -> FixedMat3 {
    FixedMat3::from_rows(vec3(x, 0, 0), vec3(0, y, 0), vec3(0, 0, z))
}

#[wasm_bindgen_test(unsupported = test)]
fn mt_tp_007_fixed_affine2_points_translate_but_vectors_do_not() {
    let affine = FixedAffine2::from_parts(scale2(2, 4), vec2(5, -7));

    assert_eq!(affine.checked_transform_point(vec2(11, 13)), Some(vec2(27, 45)));
    assert_eq!(affine.checked_transform_vector(vec2(11, 13)), Some(vec2(22, 52)));
}

#[wasm_bindgen_test(unsupported = test)]
fn mt_tp_008_fixed_affine2_composition_is_right_to_left() {
    let scale = FixedAffine2::from_parts(
        scale2(2, 3),
        FixedVec2::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO),
    );
    let translate = FixedAffine2::from_parts(FixedMat2::IDENTITY, vec2(5, 7));

    let combined = translate.checked_mul_affine(scale).expect("composition fits");

    assert_eq!(combined.checked_transform_point(vec2(2, 3)), Some(vec2(9, 16)));
    assert_eq!(combined.checked_transform_vector(vec2(2, 3)), Some(vec2(4, 9)));
}

#[wasm_bindgen_test(unsupported = test)]
fn mt_tp_009_fixed_affine_inverse_round_trips_points_and_vectors() {
    let affine = FixedAffine3::from_parts(scale3(2, 4, 8), vec3(5, -7, 11));
    let inverse = affine.try_inverse().expect("scale plus translation is invertible");
    let point = vec3(3, 5, 7);
    let vector = vec3(3, 5, 7);

    let moved_point = affine.checked_transform_point(point).expect("point transform fits");
    let moved_vector = affine.checked_transform_vector(vector).expect("vector transform fits");

    assert_eq!(moved_point, vec3(11, 13, 67));
    assert_eq!(moved_vector, vec3(6, 20, 56));
    assert_eq!(inverse.checked_transform_point(moved_point), Some(point));
    assert_eq!(inverse.checked_transform_vector(moved_vector), Some(vector));
}

#[wasm_bindgen_test(unsupported = test)]
fn mt_tp_010_affine_transform_overflow_and_singular_inverse_return_none() {
    let max = FixedQ32_32::from_raw(i64::MAX);
    let overflowing =
        FixedAffine2::from_parts(FixedMat2::IDENTITY, FixedVec2::new(max, FixedQ32_32::ZERO));
    let singular = FixedAffine3::from_parts(
        FixedMat3::from_rows(vec3(1, 0, 0), vec3(1, 0, 0), vec3(0, 0, 1)),
        vec3(0, 0, 0),
    );

    assert_eq!(overflowing.checked_transform_point(vec2(1, 0)), None);
    assert_eq!(singular.try_inverse(), None);
}
