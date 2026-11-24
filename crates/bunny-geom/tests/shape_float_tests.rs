//! Float boundary constructors (`docs/topics/shape-library/test-plan.md`, SHAPE-TP-009, -010)
//! and the `ShapeError` surface (SHAPE-TP-011).
//!
//! The oracle for an accepted conversion is the fixed constructor applied to
//! `FixedQ32_32::try_from_f32` of each component, the documented ingress rule.

use bunny_geom::{
    FixedCapsule3, FixedLine3, FixedObb3, FixedPlane3, FixedSegment3, FixedTriangle3, ShapeError,
};
use bunny_linalg::{FixedMat3, FixedUnitVec3, FixedVec3, Vec3};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const NON_FINITE: [f32; 3] = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY];
/// 2^31 is the first f32 outside Q32.32; the largest f32 below it, 2^31 - 128, is inside.
const OUT_OF_RANGE: f32 = 2_147_483_648.0;
const IN_RANGE_EDGE: f32 = 2_147_483_520.0;

fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

fn fixed(value: Vec3) -> FixedVec3 {
    FixedVec3::try_from_float(value).expect("in range")
}

fn scalar(value: f32) -> FixedQ32_32 {
    FixedQ32_32::try_from_f32(value).expect("in range")
}

#[wasm_bindgen_test(unsupported = test)]
fn shape_tp_009_float_constructors_round_then_validate() {
    let (point, dir) = (v(1.5, -2.25, 1e-10), v(0.0, 3.0, 4.0));
    assert_eq!(
        FixedLine3::try_from_float(point, dir),
        FixedLine3::try_new(fixed(point), fixed(dir))
    );
    assert_eq!(
        FixedLine3::try_from_float(point, v(0.0, 0.0, 0.0)),
        Err(ShapeError::InvalidLineDirection)
    );
    // A direction below half a raw unit rounds to zero and is then refused.
    assert_eq!(
        FixedLine3::try_from_float(point, v(1e-10, 0.0, 0.0)),
        Err(ShapeError::InvalidLineDirection)
    );
    let corners = [v(0.0, 0.0, 0.0), v(1.0, 0.0, 0.0), v(0.0, 1.0, 0.0)];
    let fixed_corners = corners.map(fixed);
    let tri = FixedTriangle3::new(fixed_corners[0], fixed_corners[1], fixed_corners[2]);
    assert_eq!(FixedTriangle3::try_from_float(corners), Ok(tri));
    // A float triangle too small for Q32.32 rounds to three coincident vertices and is refused.
    let tiny = [v(0.0, 0.0, 0.0), v(1e-11, 0.0, 0.0), v(0.0, 1e-11, 0.0)];
    assert_eq!(FixedTriangle3::try_from_float(tiny), Err(ShapeError::DegenerateTriangle));
    let (a, b) = (v(-1.0, 0.0, 0.0), v(1.0, 0.0, 0.0));
    let segment = FixedSegment3::new(fixed(a), fixed(b));
    assert_eq!(
        FixedCapsule3::try_from_float(a, b, 0.5),
        Ok(FixedCapsule3::new(segment, scalar(0.5)))
    );
    assert_eq!(FixedCapsule3::try_from_float(a, a, 0.0).map(|c| c.radius), Ok(FixedQ32_32::ZERO));
    assert_eq!(FixedCapsule3::try_from_float(a, b, -0.0).map(|c| c.radius.raw()), Ok(0), "-0.0");
    let negative = Err(ShapeError::NegativeCapsuleRadius);
    assert_eq!(FixedCapsule3::try_from_float(a, b, -1e-12), negative, "refused before rounding");
    assert_eq!(FixedCapsule3::try_from_float(a, b, -1.0), negative);
    let axes = [v(1.0, 0.0, 0.0), v(0.0, 0.0, 1.0), v(0.0, -1.0, 0.0)];
    let expected = FixedMat3::from_rows(fixed(axes[0]), fixed(axes[1]), fixed(axes[2])).transpose();
    let obb = FixedObb3::try_from_float(point, axes, v(1.0, 0.0, 2.0)).expect("a rotation");
    assert_eq!(
        (obb.center(), obb.axes(), obb.half_extents()),
        (fixed(point), expected, fixed(v(1.0, 0.0, 2.0)))
    );
    assert_eq!(obb.axes().column1(), FixedUnitVec3::UNIT_Z.into_inner(), "axes are the columns");
    assert_eq!(
        FixedObb3::try_from_float(point, axes, v(1.0, -1e-12, 2.0)),
        Err(ShapeError::NegativeHalfExtent { axis: 1 })
    );
    assert!(FixedObb3::try_from_float(point, axes, v(-0.0, 0.0, 0.0)).is_ok(), "-0.0 half extent");
    let sheared = [v(1.0, 0.0, 0.0), v(0.5, 1.0, 0.0), v(0.0, 0.0, 1.0)];
    assert_eq!(
        FixedObb3::try_from_float(point, sheared, v(1.0, 1.0, 1.0)),
        Err(ShapeError::AxisLengthOutOfTolerance { column: 1 })
    );
    // Out of range: 2^31 refused, the largest f32 below it accepted, in every input.
    let far = v(OUT_OF_RANGE, 0.0, 0.0);
    let edge = v(IN_RANGE_EDGE, 0.0, 0.0);
    let range = Some(ShapeError::FixedValueOutOfRange);
    assert_eq!(FixedLine3::try_from_float(far, dir).err(), range);
    assert!(FixedLine3::try_from_float(edge, dir).is_ok());
    assert_eq!(FixedLine3::try_from_float(point, far).err(), range);
    assert_eq!(FixedTriangle3::try_from_float([corners[0], corners[1], far]).err(), range);
    assert_eq!(FixedCapsule3::try_from_float(a, far, 1.0).err(), range);
    assert_eq!(FixedCapsule3::try_from_float(a, b, OUT_OF_RANGE).err(), range);
    assert!(FixedCapsule3::try_from_float(a, b, IN_RANGE_EDGE).is_ok());
    assert_eq!(FixedObb3::try_from_float(far, axes, v(1.0, 1.0, 1.0)).err(), range);
    assert_eq!(
        FixedObb3::try_from_float(point, [far, axes[1], axes[2]], v(1.0, 1.0, 1.0)).err(),
        range
    );
    assert_eq!(FixedObb3::try_from_float(point, axes, far).err(), range);
    assert!(FixedObb3::try_from_float(point, axes, edge).is_ok());
    for bad in NON_FINITE {
        let nf = Some(ShapeError::NonFiniteCoordinate);
        let n = v(0.0, bad, 0.0);
        assert_eq!(FixedLine3::try_from_float(n, dir).err(), nf);
        // Non-finite is decided before range: an out-of-range point does not mask it.
        assert_eq!(FixedLine3::try_from_float(far, n).err(), nf);
        assert_eq!(FixedTriangle3::try_from_float([corners[0], corners[1], n]).err(), nf);
        assert_eq!(FixedCapsule3::try_from_float(n, b, 1.0).err(), nf);
        assert_eq!(FixedCapsule3::try_from_float(a, b, bad), Err(ShapeError::NonFiniteRadius));
        assert_eq!(FixedCapsule3::try_from_float(n, b, bad).err(), nf, "coordinates first");
        assert_eq!(FixedObb3::try_from_float(n, axes, v(1.0, 1.0, 1.0)).err(), nf);
        assert_eq!(
            FixedObb3::try_from_float(point, [axes[0], n, axes[2]], v(1.0, 1.0, 1.0)).err(),
            nf
        );
        assert_eq!(FixedObb3::try_from_float(point, axes, n).err(), nf);
        assert_eq!(
            FixedObb3::try_from_float(point, axes, v(-1.0, bad, 0.0)).err(),
            nf,
            "before sign"
        );
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn shape_tp_010_segments_and_planes_accept_their_degenerate_cases() {
    let point = FixedVec3::new(FixedQ32_32::ONE, FixedQ32_32::ZERO, FixedQ32_32::ONE);
    let zero = FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO);
    let dot = FixedSegment3::new(point, point);
    assert_eq!((dot.start, dot.end), (point, point), "a zero-length segment");
    let flat = FixedPlane3::new(zero, FixedQ32_32::ONE);
    assert_eq!(
        flat.signed_distance(point),
        Some(-FixedQ32_32::ONE),
        "a zero normal: -offset everywhere"
    );
    let (a, b) = (v(1.0, 0.0, 1.0), v(-2.0, 0.5, 0.0));
    assert_eq!(FixedSegment3::try_from_float(a, b), Ok(FixedSegment3::new(fixed(a), fixed(b))));
    assert_eq!(FixedSegment3::try_from_float(a, a), Ok(FixedSegment3::new(point, point)));
    assert_eq!(FixedPlane3::try_from_float(b, 0.25), Ok(FixedPlane3::new(fixed(b), scalar(0.25))));
    assert_eq!(FixedPlane3::try_from_float(v(0.0, 0.0, 0.0), 1.0), Ok(flat));
    let range = Some(ShapeError::FixedValueOutOfRange);
    assert_eq!(FixedSegment3::try_from_float(a, v(0.0, 0.0, OUT_OF_RANGE)).err(), range);
    assert_eq!(FixedPlane3::try_from_float(v(0.0, OUT_OF_RANGE, 0.0), 1.0).err(), range);
    assert_eq!(FixedPlane3::try_from_float(b, -OUT_OF_RANGE - 256.0).err(), range);
    assert!(FixedPlane3::try_from_float(b, -OUT_OF_RANGE).is_ok(), "-2^31 is i64::MIN raw");
    for bad in NON_FINITE {
        let nf = Some(ShapeError::NonFiniteCoordinate);
        assert_eq!(FixedSegment3::try_from_float(v(bad, 0.0, 0.0), b).err(), nf);
        assert_eq!(FixedSegment3::try_from_float(a, v(0.0, 0.0, bad)).err(), nf);
        assert_eq!(FixedPlane3::try_from_float(v(bad, 0.0, 0.0), 1.0).err(), nf);
        assert_eq!(FixedPlane3::try_from_float(b, bad).err(), nf);
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn shape_tp_011_shape_error_is_an_error_with_messages() {
    fn assert_error<E: core::error::Error>() {}
    assert_error::<ShapeError>();
    let messages = [
        (ShapeError::InvalidLineDirection, "line direction normalization failed"),
        (ShapeError::DegenerateTriangle, "triangle vertices are collinear"),
        (ShapeError::NegativeCapsuleRadius, "capsule radius is negative"),
        (ShapeError::NegativeHalfExtent { axis: 2 }, "box half extent 2 is negative"),
        (
            ShapeError::AxisLengthOutOfTolerance { column: 1 },
            "box axis 1 is not unit length within tolerance",
        ),
        (
            ShapeError::AxesNotOrthogonal { first: 0, second: 2 },
            "box axes 0 and 2 are not orthogonal within tolerance",
        ),
        (ShapeError::NonFiniteCoordinate, "coordinate is not finite"),
        (ShapeError::NonFiniteRadius, "radius is not finite"),
        (ShapeError::FixedValueOutOfRange, "value is outside the Q32.32 range"),
    ];
    for (error, message) in messages {
        assert_eq!(error.to_string(), message);
    }
}
