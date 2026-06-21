//! Deterministic matrix contract tests for Bunny fixed-point linear algebra.

use bunny_linalg::{FixedMat2, FixedMat3, FixedMat4, FixedVec2, FixedVec3};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const ONE_RAW: i64 = bunny_num::fixed_q32_32::ONE_RAW;
const HALF_RAW: i64 = ONE_RAW / 2;

fn q32(value: i32) -> FixedQ32_32 {
    FixedQ32_32::from_raw(i64::from(value) * ONE_RAW)
}

fn q32_raw(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

fn vec2(x: i32, y: i32) -> FixedVec2 {
    FixedVec2::new(q32(x), q32(y))
}

fn vec3(x: i32, y: i32, z: i32) -> FixedVec3 {
    FixedVec3::new(q32(x), q32(y), q32(z))
}

fn raw2(value: FixedVec2) -> [i64; 2] {
    [value.x.raw(), value.y.raw()]
}

fn raw3(value: FixedVec3) -> [i64; 3] {
    [value.x.raw(), value.y.raw(), value.z.raw()]
}

#[wasm_bindgen_test(unsupported = test)]
fn fixed_mat2_layout_transpose_and_raw_vector_multiply_are_stable() {
    let matrix = FixedMat2::new(q32(1), q32(2), q32(3), q32(4));

    assert_eq!(matrix.row0(), vec2(1, 2));
    assert_eq!(matrix.row1(), vec2(3, 4));
    assert_eq!(matrix.column0(), vec2(1, 3));
    assert_eq!(matrix.column1(), vec2(2, 4));

    let transpose = matrix.transpose();
    assert_eq!(transpose.row0(), vec2(1, 3));
    assert_eq!(transpose.row1(), vec2(2, 4));

    let product = matrix.checked_mul_vec2(vec2(5, 6)).expect("matrix-vector multiply fits");
    assert_eq!(raw2(product), [17 * ONE_RAW, 39 * ONE_RAW]);
}

#[wasm_bindgen_test(unsupported = test)]
fn fixed_mat2_determinant_inverse_and_matrix_multiply_are_stable() {
    let matrix = FixedMat2::new(q32(2), q32(1), q32(1), q32(1));

    assert_eq!(matrix.determinant(), Some(q32(1)));

    let inverse = matrix.try_inverse().expect("matrix is invertible");
    assert_eq!(inverse.row0(), vec2(1, -1));
    assert_eq!(inverse.row1(), vec2(-1, 2));

    let identity = matrix.checked_mul_mat2(inverse).expect("matrix multiply fits");
    assert_eq!(identity, FixedMat2::IDENTITY);
}

#[wasm_bindgen_test(unsupported = test)]
fn fixed_mat3_determinant_inverse_and_raw_vector_multiply_are_stable() {
    let matrix = FixedMat3::from_rows(vec3(1, 2, 3), vec3(0, 1, 4), vec3(5, 6, 0));

    assert_eq!(matrix.determinant(), Some(q32(1)));

    let product = matrix.checked_mul_vec3(vec3(1, 2, 3)).expect("matrix-vector multiply fits");
    assert_eq!(raw3(product), [14 * ONE_RAW, 14 * ONE_RAW, 17 * ONE_RAW]);

    let inverse = matrix.try_inverse().expect("matrix is invertible");
    assert_eq!(inverse.row0(), vec3(-24, 18, 5));
    assert_eq!(inverse.row1(), vec3(20, -15, -4));
    assert_eq!(inverse.row2(), vec3(-5, 4, 1));

    let identity = matrix.checked_mul_mat3(inverse).expect("matrix multiply fits");
    assert_eq!(identity, FixedMat3::IDENTITY);
}

#[wasm_bindgen_test(unsupported = test)]
fn fixed_mat4_identity_transpose_and_matrix_multiply_are_stable() {
    let scale_translate = FixedMat4::from_rows(
        (q32(2), q32(0), q32(0), q32(8)),
        (q32(0), q32(2), q32(0), q32(10)),
        (q32(0), q32(0), q32(2), q32(12)),
        (q32(0), q32(0), q32(0), q32(1)),
    );

    assert_eq!(scale_translate.determinant(), Some(q32(8)));
    assert_eq!(FixedMat4::IDENTITY.checked_mul_mat4(scale_translate), Some(scale_translate));
    assert_eq!(scale_translate.checked_mul_mat4(FixedMat4::IDENTITY), Some(scale_translate));

    let transpose = scale_translate.transpose();
    assert_eq!(transpose.column3(), FixedVec3::new(q32(0), q32(0), q32(0)));
    assert_eq!(transpose.row3_xyz(), vec3(8, 10, 12));

    let inverse = scale_translate.try_inverse().expect("matrix is invertible");
    assert_eq!(
        inverse,
        FixedMat4::from_rows(
            (q32_raw(HALF_RAW), q32(0), q32(0), q32(-4)),
            (q32(0), q32_raw(HALF_RAW), q32(0), q32(-5)),
            (q32(0), q32(0), q32_raw(HALF_RAW), q32(-6)),
            (q32(0), q32(0), q32(0), q32(1)),
        )
    );
    assert_eq!(scale_translate.checked_mul_mat4(inverse), Some(FixedMat4::IDENTITY));
}

#[wasm_bindgen_test(unsupported = test)]
fn matrix_inverse_returns_none_for_degenerate_or_overflowing_cases() {
    let singular_2 = FixedMat2::new(q32(1), q32(2), q32(2), q32(4));
    assert_eq!(singular_2.determinant(), Some(FixedQ32_32::ZERO));
    assert_eq!(singular_2.try_inverse(), None);

    let singular_3 = FixedMat3::from_rows(vec3(1, 2, 3), vec3(1, 2, 3), vec3(4, 5, 6));
    assert_eq!(singular_3.determinant(), Some(FixedQ32_32::ZERO));
    assert_eq!(singular_3.try_inverse(), None);

    let singular_4 = FixedMat4::from_rows(
        (q32(1), q32(2), q32(3), q32(4)),
        (q32(1), q32(2), q32(3), q32(4)),
        (q32(5), q32(6), q32(7), q32(8)),
        (q32(0), q32(0), q32(0), q32(1)),
    );
    assert_eq!(singular_4.determinant(), Some(FixedQ32_32::ZERO));
    assert_eq!(singular_4.try_inverse(), None);

    let max = FixedQ32_32::from_raw(i64::MAX);
    let overflowing = FixedMat2::new(max, max, max, max);
    assert_eq!(overflowing.checked_mul_vec2(FixedVec2::new(q32(2), q32(2))), None);
}

#[wasm_bindgen_test(unsupported = test)]
fn fixed_mat2_fractional_inverse_uses_q32_32_raw_outputs() {
    let matrix = FixedMat2::new(q32(2), FixedQ32_32::ZERO, FixedQ32_32::ZERO, q32(2));

    let inverse = matrix.try_inverse().expect("scale matrix is invertible");

    assert_eq!(inverse.row0(), FixedVec2::new(q32_raw(HALF_RAW), FixedQ32_32::ZERO));
    assert_eq!(inverse.row1(), FixedVec2::new(FixedQ32_32::ZERO, q32_raw(HALF_RAW)));
}

#[wasm_bindgen_test(unsupported = test)]
fn mt_tp_012_fixed_mat2_inverse_divides_min_off_diagonal_before_negating() {
    let matrix = FixedMat2::new(q32(2), FixedQ32_32::from_raw(i64::MIN), q32(0), q32(2));

    let inverse = matrix.try_inverse().expect("inverse is representable after division");

    assert_eq!(inverse.m00, q32_raw(HALF_RAW));
    assert_eq!(inverse.m01, q32_raw(-(i64::MIN / 4)));
    assert_eq!(inverse.m10, q32(0));
    assert_eq!(inverse.m11, q32_raw(HALF_RAW));
}

#[wasm_bindgen_test(unsupported = test)]
fn mt_tp_013_fixed_mat3_inverse_divides_min_cofactor_before_negating() {
    let matrix = FixedMat3::from_rows(
        vec3(2, 0, 0),
        FixedVec3::new(FixedQ32_32::from_raw(i64::MIN), q32(1), q32(0)),
        vec3(0, 0, 1),
    );

    let inverse = matrix.try_inverse().expect("inverse is representable after division");

    assert_eq!(inverse.row0(), FixedVec3::new(q32_raw(HALF_RAW), q32(0), q32(0)));
    assert_eq!(inverse.row1(), FixedVec3::new(q32_raw(-(i64::MIN / 2)), q32(1), q32(0)));
    assert_eq!(inverse.row2(), vec3(0, 0, 1));
}

#[wasm_bindgen_test(unsupported = test)]
fn mt_tp_014_fixed_mat4_inverse_divides_min_cofactor_before_negating() {
    let matrix = FixedMat4::from_rows(
        (q32(2), q32(0), q32(0), q32(0)),
        (FixedQ32_32::from_raw(i64::MIN), q32(1), q32(0), q32(0)),
        (q32(0), q32(0), q32(1), q32(0)),
        (q32(0), q32(0), q32(0), q32(1)),
    );

    let inverse = matrix.try_inverse().expect("inverse is representable after division");

    assert_eq!(
        inverse,
        FixedMat4::from_rows(
            (q32_raw(HALF_RAW), q32(0), q32(0), q32(0)),
            (q32_raw(-(i64::MIN / 2)), q32(1), q32(0), q32(0)),
            (q32(0), q32(0), q32(1), q32(0)),
            (q32(0), q32(0), q32(0), q32(1)),
        )
    );
}
