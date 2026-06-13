use bunny_linalg::{FixedVec2, FixedVec3, Vec2, Vec3};
use bunny_num::FixedQ32_32;

#[test]
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

#[test]
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

#[test]
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

    let vf2 = Vec2::new(5.5, 6.5);
    let vfx2 = FixedVec2::from(vf2);
    let vf2_back = Vec2::from(vfx2);
    #[allow(clippy::float_cmp)]
    {
        assert_eq!(vf2_back.x, 5.5);
        assert_eq!(vf2_back.y, 6.5);
    }
}

#[test]
fn test_tiny_vector_length() {
    // A vector with components so small that squaring them in Q32.32
    // would round to zero if we round back to Q32.32 before taking the square root.
    let tiny_v2 = FixedVec2::new(FixedQ32_32::from_raw(1), FixedQ32_32::from_raw(0));
    let len_v2 = tiny_v2
        .length()
        .expect("Tiny FixedVec2 length should be computed");
    assert_eq!(len_v2.to_raw(), 1);

    let norm_v2 = tiny_v2
        .normalize()
        .expect("Tiny FixedVec2 normalize should succeed");
    assert_eq!(norm_v2.x.to_raw(), 1);
    assert_eq!(norm_v2.y.to_raw(), 0);

    let tiny_v3 = FixedVec3::new(
        FixedQ32_32::from_raw(0),
        FixedQ32_32::from_raw(1),
        FixedQ32_32::from_raw(0),
    );
    let len_v3 = tiny_v3
        .length()
        .expect("Tiny FixedVec3 length should be computed");
    assert_eq!(len_v3.to_raw(), 1);

    let norm_v3 = tiny_v3
        .normalize()
        .expect("Tiny FixedVec3 normalize should succeed");
    assert_eq!(norm_v3.x.to_raw(), 0);
    assert_eq!(norm_v3.y.to_raw(), 1);
    assert_eq!(norm_v3.z.to_raw(), 0);
}
