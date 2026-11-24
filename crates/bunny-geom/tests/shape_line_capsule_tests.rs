//! Lines and capsules (`docs/topics/shape-library/test-plan.md`, SHAPE-TP-001, -002, -005).
//!
//! A line's direction rule is defined as the ray's, so the oracle for a line refusal is
//! `FixedRay3::try_new` and the documented `FixedUnitVec3` band, checked here on constructed
//! vectors one raw unit either side of each threshold and on seeded vectors.

mod shape_oracle;

use bunny_geom::{FixedCapsule3, FixedLine3, FixedRay3, FixedSegment3, ShapeError};
use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::FixedQ32_32;
use shape_oracle::Rng;
use wasm_bindgen_test::wasm_bindgen_test;

const ONE: i64 = 1 << 32;
const LINE_SEED: u64 = 0x5EED_0115_0000_0001;
const LINE_CASES: usize = 20_000;
const ORIGIN: FixedVec3 = FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO);
const CONST_CAPSULE: Result<FixedCapsule3, ShapeError> =
    FixedCapsule3::try_new(FixedSegment3::new(ORIGIN, ORIGIN), FixedQ32_32::ONE);

fn raw(v: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(v)
}

fn pt(x: i64, y: i64, z: i64) -> FixedVec3 {
    FixedVec3::new(raw(x), raw(y), raw(z))
}

#[wasm_bindgen_test(unsupported = test)]
fn shape_tp_001_lines_keep_their_representation() {
    let point = pt(ONE, -2 * ONE, 3);
    let line = FixedLine3::new(point, FixedUnitVec3::UNIT_X);
    assert_eq!((line.point, line.direction), (point, FixedUnitVec3::UNIT_X));
    // try_new normalizes: an axis-aligned direction of any length gives the exact unit axis.
    assert_eq!(
        FixedLine3::try_new(point, pt(0, 0, 7 * ONE)).map(|l| l.direction),
        Ok(FixedUnitVec3::UNIT_Z)
    );
    assert_eq!(
        FixedLine3::try_new(point, pt(-5, 0, 0)),
        Ok(FixedLine3::new(point, FixedUnitVec3::NEG_UNIT_X))
    );
    // Representation equality: the same geometry through another point, or reversed, is not ==.
    let elsewhere = FixedLine3::new(pt(2 * ONE, -2 * ONE, 3), FixedUnitVec3::UNIT_X);
    let reversed = FixedLine3::new(point, FixedUnitVec3::NEG_UNIT_X);
    assert_ne!(line, elsewhere);
    assert_ne!(line, reversed);
    // The line and the ray share a representation: same point, same unit direction.
    let ray = FixedRay3::new(point, FixedUnitVec3::UNIT_X);
    assert_eq!((ray.origin, ray.direction), (line.point, line.direction));
}

#[wasm_bindgen_test(unsupported = test)]
fn shape_tp_002_line_directions_follow_the_ray_rule() {
    let p = pt(0, 0, 0);
    let refused = Err(ShapeError::InvalidLineDirection);
    // Zero, and one raw unit either side of it on each axis.
    assert_eq!(FixedLine3::try_new(p, pt(0, 0, 0)), refused);
    for d in [pt(1, 0, 0), pt(-1, 0, 0), pt(0, 1, 0), pt(0, 0, -1)] {
        assert!(FixedLine3::try_new(p, d).is_ok(), "{d:?}");
    }
    // (1, 1, 0) raw: floor sqrt(2) = 1 raw, so the normalized length is sqrt 2 and it is refused.
    assert_eq!(FixedLine3::try_new(p, pt(1, 1, 0)), refused);
    // The ends of the raw range: i64::MAX alone fits, its length does not when doubled up,
    // and i64::MIN has magnitude 2^63, one more than i64::MAX.
    assert!(FixedLine3::try_new(p, pt(i64::MAX, 0, 0)).is_ok());
    assert_eq!(FixedLine3::try_new(p, pt(i64::MAX, i64::MAX, 0)), refused);
    assert_eq!(FixedLine3::try_new(p, pt(i64::MIN, 0, 0)), refused);
    let constructed = [pt(1, 1, 0), pt(i64::MAX, 0, 0), pt(i64::MIN, 0, 0), pt(ONE, ONE, ONE)];
    for d in constructed {
        assert_same_rule_as_ray(d, "constructed");
    }
    let mut rng = Rng(LINE_SEED);
    let (mut accepted, mut rejected) = (0, 0);
    for case in 0..LINE_CASES {
        let scale = 1_i64 << (rng.next_u64() % 62);
        let d = pt(rng.within(scale), rng.within(scale), rng.within(scale));
        let context = format!("seed={LINE_SEED:#x} case={case} d={d:?}");
        if assert_same_rule_as_ray(d, &context) {
            accepted += 1;
        } else {
            rejected += 1;
        }
    }
    assert!(accepted >= 9_000 && rejected >= 5_000, "seed={LINE_SEED:#x}: {accepted} {rejected}");
}

/// The line accepts exactly what the ray accepts, with the same unit direction.
fn assert_same_rule_as_ray(d: FixedVec3, context: &str) -> bool {
    let line = FixedLine3::try_new(pt(1, 2, 3), d);
    let ray = FixedRay3::try_new(pt(1, 2, 3), d);
    assert_eq!(line.map(|l| l.direction).ok(), ray.map(|r| r.direction).ok(), "{context}");
    assert_eq!(line.ok().map(|l| l.direction), FixedUnitVec3::new(d), "{context}");
    line.is_ok()
}

#[wasm_bindgen_test(unsupported = test)]
fn shape_tp_005_capsule_radius_and_degenerate_capsules() {
    let segment = FixedSegment3::new(pt(-ONE, 0, 0), pt(ONE, 0, 0));
    let capsule = FixedCapsule3::try_new(segment, raw(ONE / 2)).expect("a valid capsule");
    assert_eq!((capsule.segment, capsule.radius), (segment, raw(ONE / 2)));
    // The radius threshold: one raw unit either side of zero, and the ends of the range.
    assert_eq!(FixedCapsule3::try_new(segment, raw(-1)), Err(ShapeError::NegativeCapsuleRadius));
    assert_eq!(
        FixedCapsule3::try_new(segment, raw(i64::MIN)),
        Err(ShapeError::NegativeCapsuleRadius)
    );
    assert!(FixedCapsule3::try_new(segment, raw(0)).is_ok(), "zero radius: the segment");
    assert!(FixedCapsule3::try_new(segment, raw(1)).is_ok());
    assert!(FixedCapsule3::try_new(segment, raw(i64::MAX)).is_ok());
    // A zero-length segment is a sphere; with zero radius too, a point.
    let point = FixedSegment3::new(pt(5, 6, 7), pt(5, 6, 7));
    assert!(FixedCapsule3::try_new(point, raw(ONE)).is_ok(), "zero length: a sphere");
    assert!(FixedCapsule3::try_new(point, raw(0)).is_ok(), "a point");
    // new does not validate, as FixedSphere3::new does not.
    assert_eq!(FixedCapsule3::new(segment, raw(-1)).radius, raw(-1));
    // Endpoint order is part of the representation.
    let flipped = FixedSegment3::new(segment.end, segment.start);
    assert_ne!(FixedCapsule3::new(segment, raw(1)), FixedCapsule3::new(flipped, raw(1)));
    assert!(CONST_CAPSULE.is_ok(), "try_new is usable in constant expressions");
}
