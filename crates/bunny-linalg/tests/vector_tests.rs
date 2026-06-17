//! Integration tests.

use bunny_linalg::{FixedVec2, FixedVec3, Vec2, Vec3};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test(unsupported = test)]
fn test_fixed_vec2_operations() {
    let a = FixedVec2::new(FixedQ32_32::from_f32(3.0), FixedQ32_32::from_f32(4.0));
    let b = FixedVec2::new(FixedQ32_32::from_f32(1.0), FixedQ32_32::from_f32(2.0));

    let sum = a + b;
    #[allow(clippy::float_cmp)]
    {
        assert_eq!(sum.x.to_f32(), 4.0);
        assert_eq!(sum.y.to_f32(), 6.0);
    }

    let diff = a - b;
    #[allow(clippy::float_cmp)]
    {
        assert_eq!(diff.x.to_f32(), 2.0);
        assert_eq!(diff.y.to_f32(), 2.0);
    }

    let dot_prod = a.dot(b);
    #[allow(clippy::float_cmp)]
    {
        assert_eq!(dot_prod.to_f32(), 11.0);
    }

    let len_sq = a.length_squared();
    #[allow(clippy::float_cmp)]
    {
        assert_eq!(len_sq.to_f32(), 25.0);
    }

    let len = a.length().expect("length should compute");
    #[allow(clippy::float_cmp)]
    {
        assert_eq!(len.to_f32(), 5.0);
    }

    let norm = a.normalize().expect("should normalize");
    #[allow(clippy::float_cmp)]
    {
        assert_eq!(norm.x.to_f32(), 3.0 / 5.0);
        assert_eq!(norm.y.to_f32(), 4.0 / 5.0);
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn test_fixed_vec3_operations() {
    let a = FixedVec3::new(
        FixedQ32_32::from_f32(1.0),
        FixedQ32_32::from_f32(2.0),
        FixedQ32_32::from_f32(3.0),
    );
    let b = FixedVec3::new(
        FixedQ32_32::from_f32(4.0),
        FixedQ32_32::from_f32(5.0),
        FixedQ32_32::from_f32(6.0),
    );

    let sum = a + b;
    #[allow(clippy::float_cmp)]
    {
        assert_eq!(sum.x.to_f32(), 5.0);
        assert_eq!(sum.y.to_f32(), 7.0);
        assert_eq!(sum.z.to_f32(), 9.0);
    }

    let scale = a * FixedQ32_32::from_f32(2.0);
    #[allow(clippy::float_cmp)]
    {
        assert_eq!(scale.x.to_f32(), 2.0);
        assert_eq!(scale.y.to_f32(), 4.0);
        assert_eq!(scale.z.to_f32(), 6.0);
    }

    let cross_prod = a.cross(b);
    // [1,2,3] x [4,5,6] = [2*6 - 3*5, 3*4 - 1*6, 1*5 - 2*4] = [-3, 6, -3]
    #[allow(clippy::float_cmp)]
    {
        assert_eq!(cross_prod.x.to_f32(), -3.0);
        assert_eq!(cross_prod.y.to_f32(), 6.0);
        assert_eq!(cross_prod.z.to_f32(), -3.0);
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn test_conversions() {
    let vf = Vec3::new(1.0, 2.0, 3.0);
    let vfx = FixedVec3::from(vf);
    let vf_back = Vec3::from(vfx);
    #[allow(clippy::float_cmp)]
    {
        assert_eq!(vf_back.x, 1.0);
        assert_eq!(vf_back.y, 2.0);
        assert_eq!(vf_back.z, 3.0);
    }

    let source_vec2 = Vec2::new(5.5, 6.5);
    let fixed_vec2 = FixedVec2::from(source_vec2);
    let roundtrip_vec2 = Vec2::from(fixed_vec2);
    #[allow(clippy::float_cmp)]
    {
        assert_eq!(roundtrip_vec2.x, 5.5);
        assert_eq!(roundtrip_vec2.y, 6.5);
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn test_tiny_vector_length() {
    // A vector with components so small that squaring them in Q32.32
    // would round to zero if we round back to Q32.32 before taking the square root.
    let tiny_v2 = FixedVec2::new(FixedQ32_32::from_raw(1), FixedQ32_32::from_raw(0));
    let len_v2 = tiny_v2.length().expect("Tiny FixedVec2 length should be computed");
    assert_eq!(len_v2.to_raw(), 1);

    let norm_v2 = tiny_v2.normalize().expect("Tiny FixedVec2 normalize should succeed");
    assert_eq!(norm_v2.x, FixedQ32_32::ONE);
    assert_eq!(norm_v2.y, FixedQ32_32::ZERO);

    let tiny_v3 = FixedVec3::new(
        FixedQ32_32::from_raw(0),
        FixedQ32_32::from_raw(1),
        FixedQ32_32::from_raw(0),
    );
    let len_v3 = tiny_v3.length().expect("Tiny FixedVec3 length should be computed");
    assert_eq!(len_v3.to_raw(), 1);

    let norm_v3 = tiny_v3.normalize().expect("Tiny FixedVec3 normalize should succeed");
    assert_eq!(norm_v3.x, FixedQ32_32::ZERO);
    assert_eq!(norm_v3.y, FixedQ32_32::ONE);
    assert_eq!(norm_v3.z, FixedQ32_32::ZERO);
}

#[wasm_bindgen_test(unsupported = test)]
fn test_vector_saturation_and_boundaries() {
    let max_val = FixedQ32_32::from_raw(i64::MAX);
    let min_val = FixedQ32_32::from_raw(i64::MIN);
    let one = FixedQ32_32::ONE;
    let two = FixedQ32_32::from_f32(2.0);
    let neg_two = FixedQ32_32::from_f32(-2.0);

    let v2_max = FixedVec2::new(max_val, max_val);
    let v2_min = FixedVec2::new(min_val, min_val);

    // Addition saturation
    let v2_add_overflow = v2_max + FixedVec2::new(one, one);
    assert_eq!(v2_add_overflow.x, max_val);
    assert_eq!(v2_add_overflow.y, max_val);

    let v2_add_underflow = v2_min + FixedVec2::new(-one, -one);
    assert_eq!(v2_add_underflow.x, min_val);
    assert_eq!(v2_add_underflow.y, min_val);

    // Subtraction saturation
    let v2_sub_overflow = v2_max - FixedVec2::new(-one, -one);
    assert_eq!(v2_sub_overflow.x, max_val);
    assert_eq!(v2_sub_overflow.y, max_val);

    let v2_sub_underflow = v2_min - FixedVec2::new(one, one);
    assert_eq!(v2_sub_underflow.x, min_val);
    assert_eq!(v2_sub_underflow.y, min_val);

    // Multiplication saturation
    let v2_mul_overflow = v2_max * two;
    assert_eq!(v2_mul_overflow.x, max_val);
    assert_eq!(v2_mul_overflow.y, max_val);

    let v2_mul_underflow = v2_max * neg_two;
    assert_eq!(v2_mul_underflow.x, min_val);
    assert_eq!(v2_mul_underflow.y, min_val);

    // Division saturation (div by zero & overflow)
    let v2_div_zero = v2_max / FixedQ32_32::ZERO;
    assert_eq!(v2_div_zero.x, max_val);
    assert_eq!(v2_div_zero.y, max_val);

    let v2_div_overflow = v2_max / FixedQ32_32::from_raw(1);
    assert_eq!(v2_div_overflow.x, max_val);
    assert_eq!(v2_div_overflow.y, max_val);

    // Dot product saturation
    let dot_overflow = v2_max.dot(v2_max);
    assert_eq!(dot_overflow, max_val);

    // Length and normalization reject unrepresentable magnitudes.
    let len_sq_overflow = v2_max.length_squared();
    assert_eq!(len_sq_overflow, max_val);

    assert_eq!(v2_max.length(), None);
    assert_eq!(v2_max.normalize(), None);

    // FixedVec3 saturation tests
    let v3_max = FixedVec3::new(max_val, max_val, max_val);
    let v3_min = FixedVec3::new(min_val, min_val, min_val);

    let v3_add_overflow = v3_max + FixedVec3::new(one, one, one);
    assert_eq!(v3_add_overflow.x, max_val);
    assert_eq!(v3_add_overflow.y, max_val);
    assert_eq!(v3_add_overflow.z, max_val);

    let v3_add_underflow = v3_min + FixedVec3::new(-one, -one, -one);
    assert_eq!(v3_add_underflow.x, min_val);
    assert_eq!(v3_add_underflow.y, min_val);
    assert_eq!(v3_add_underflow.z, min_val);

    let v3_mul_overflow = v3_max * two;
    assert_eq!(v3_mul_overflow.x, max_val);

    let v3_div_zero = v3_max / FixedQ32_32::ZERO;
    assert_eq!(v3_div_zero.x, max_val);

    let v3_dot_overflow = v3_max.dot(v3_max);
    assert_eq!(v3_dot_overflow, max_val);

    assert_eq!(v3_max.length(), None);
    assert_eq!(v3_max.normalize(), None);

    // Cross product saturation
    let v3_cross_overflow =
        v3_max.cross(FixedVec3::new(max_val * two, max_val * neg_two, max_val * two));
    assert_eq!(v3_cross_overflow.x, max_val);
}

#[wasm_bindgen_test(unsupported = test)]
fn test_fixed_unit_vectors() {
    use bunny_linalg::{FixedUnitVec2, FixedUnitVec3};

    // 1. FixedUnitVec2
    let v2_valid = FixedVec2::new(FixedQ32_32::from_f32(3.0), FixedQ32_32::from_f32(4.0));
    let uv2 = FixedUnitVec2::new(v2_valid).expect("should normalize");
    let inner2 = uv2.into_inner();
    #[allow(clippy::float_cmp)]
    {
        assert_eq!(inner2.x.to_f32(), 3.0 / 5.0);
        assert_eq!(inner2.y.to_f32(), 4.0 / 5.0);
    }

    let uv2_zero = FixedUnitVec2::new(FixedVec2::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO));
    assert!(uv2_zero.is_none());
    assert!(FixedUnitVec2::new(FixedVec2::new(
        FixedQ32_32::from_raw(i64::MAX),
        FixedQ32_32::from_raw(i64::MAX),
    ))
    .is_none());

    // 2. FixedUnitVec3
    let v3_valid = FixedVec3::new(
        FixedQ32_32::from_f32(2.0),
        FixedQ32_32::from_f32(3.0),
        FixedQ32_32::from_f32(6.0),
    );
    let uv3 = FixedUnitVec3::new(v3_valid).expect("should normalize");
    let inner3 = uv3.into_inner();
    assert!((inner3.x.to_f32() - (2.0 / 7.0)).abs() < 1e-6);
    assert!((inner3.y.to_f32() - (3.0 / 7.0)).abs() < 1e-6);
    assert!((inner3.z.to_f32() - (6.0 / 7.0)).abs() < 1e-6);

    let uv3_zero =
        FixedUnitVec3::new(FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO));
    assert!(uv3_zero.is_none());
    assert!(FixedUnitVec3::new(FixedVec3::new(
        FixedQ32_32::from_raw(i64::MAX),
        FixedQ32_32::from_raw(i64::MAX),
        FixedQ32_32::from_raw(i64::MAX),
    ))
    .is_none());
    assert!(FixedUnitVec2::new(
        FixedVec2::new(FixedQ32_32::from_raw(1), FixedQ32_32::from_raw(1),)
    )
    .is_none());
    assert!(FixedUnitVec3::new(FixedVec3::new(
        FixedQ32_32::from_raw(1),
        FixedQ32_32::from_raw(1),
        FixedQ32_32::ZERO,
    ))
    .is_none());
}
