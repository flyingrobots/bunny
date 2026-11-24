//! CHARACTERIZATION tests for flyingrobots/bunny#116: what each public `bunny-geom` shape
//! constructor and query does TODAY with each degenerate input that applies to it.
//!
//! These tests pin current behaviour, whatever it is. They are not an endorsed contract: a
//! cell pinned here can be inconsistent with a neighbouring cell, and several are. Where a
//! pinned behaviour is a known defect, the test's doc comment names the issue. Changing a
//! pinned cell is a behaviour change to a public function, which the repository owner decides
//! (see `docs/topics/degeneracy/README.md`); update the test and the inventory together.
//!
//! The degenerate inputs are those of the inventory: an inverted box (`min > max`), a
//! zero-size box, a negative or zero radius, a zero-length segment, a zero direction,
//! collinear or repeated triangle vertices, an empty point set, coincident planes, and values
//! at `i64::MIN` and `i64::MAX` raw.

use bunny_geom::{
    barycentric2, barycentric3_coplanar, orientation2, orientation3, BoundsError, FixedAabb3,
    FixedCapsule3, FixedFrustum, FixedLine3, FixedObb3, FixedPlane3, FixedRay3, FixedSegment3,
    FixedSphere3, FixedTriangle3, FrustumClass, FrustumSphereError, GeomError, Orientation2,
    Orientation3, PlaneSide, PlaneSpan, PolygonClip, PolygonClipError, SegmentClip, ShapeError,
    TransformError,
};
use bunny_linalg::{FixedAffine3, FixedMat3, FixedMat4, FixedUnitVec3, FixedVec2, FixedVec3};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const MAX: i64 = i64::MAX;
const MIN: i64 = i64::MIN;
const ONE: i64 = 1 << 32;

type Raw3 = [i64; 3];

fn q(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

fn v(raw: Raw3) -> FixedVec3 {
    FixedVec3::new(q(raw[0]), q(raw[1]), q(raw[2]))
}

/// Whole units, as raw.
fn u(units: Raw3) -> FixedVec3 {
    v(units.map(|c| c * ONE))
}

fn raw(p: FixedVec3) -> Raw3 {
    [p.x.raw(), p.y.raw(), p.z.raw()]
}

fn aabb(min: Raw3, max: Raw3) -> FixedAabb3 {
    FixedAabb3::new(v(min), v(max))
}

fn plane(normal: Raw3, offset: i64) -> FixedPlane3 {
    FixedPlane3::new(v(normal), q(offset))
}

fn zero_mat3() -> FixedMat3 {
    let zero = v([0; 3]);
    FixedMat3::from_rows(zero, zero, zero)
}

/// Constructors: `new` keeps an inverted box as given, `try_new` refuses it; a zero-size box
/// and the full raw range are accepted by both.
#[wasm_bindgen_test(unsupported = test)]
fn degen_tp_001_characterization_box_constructors() {
    let inverted = aabb([5, 0, 0], [1, 1, 1]);
    assert_eq!((raw(inverted.min), raw(inverted.max)), ([5, 0, 0], [1, 1, 1]), "kept as given");
    assert_eq!(FixedAabb3::try_new(v([5, 0, 0]), v([1, 1, 1])), Err(GeomError::InvalidAabbBounds));
    assert!(FixedAabb3::try_new(v([3; 3]), v([3; 3])).is_ok(), "zero-size accepted");
    assert!(FixedAabb3::try_new(v([MIN; 3]), v([MAX; 3])).is_ok(), "full range accepted");
}

/// Radii: `FixedSphere3::new` and `FixedCapsule3::new` keep a negative radius; `try_new`
/// refuses it and accepts zero. A zero-length capsule segment is accepted.
#[wasm_bindgen_test(unsupported = test)]
fn degen_tp_002_characterization_radius_constructors() {
    let centre = v([0; 3]);
    assert_eq!(FixedSphere3::new(centre, q(-1)).radius.raw(), -1);
    assert_eq!(FixedSphere3::try_new(centre, q(-1)), Err(GeomError::NegativeSphereRadius));
    assert!(FixedSphere3::try_new(centre, q(0)).is_ok());
    assert!(FixedSphere3::try_new(centre, q(MAX)).is_ok());

    let point = FixedSegment3::new(centre, centre);
    assert_eq!(FixedCapsule3::new(point, q(-1)).radius.raw(), -1);
    assert_eq!(FixedCapsule3::try_new(point, q(-1)), Err(ShapeError::NegativeCapsuleRadius));
    assert!(FixedCapsule3::try_new(point, q(0)).is_ok(), "a point capsule is accepted");
}

/// Zero direction: every ray and line constructor that normalizes refuses it, and so does a
/// direction too short to normalize (`(1, 1, 0)` raw). `FixedRay3::new` and
/// `FixedLine3::new` take a `FixedUnitVec3`, which cannot be zero.
#[wasm_bindgen_test(unsupported = test)]
fn degen_tp_003_characterization_zero_direction() {
    let origin = v([0; 3]);
    assert_eq!(FixedUnitVec3::new(v([0; 3])), None);
    assert_eq!(FixedRay3::try_new(origin, v([0; 3])), Err(GeomError::InvalidRayDirection));
    assert_eq!(FixedRay3::try_new(origin, v([1, 1, 0])), Err(GeomError::InvalidRayDirection));
    assert_eq!(FixedLine3::try_new(origin, v([0; 3])), Err(ShapeError::InvalidLineDirection));
    assert!(FixedRay3::try_new(origin, v([1, 0, 0])).is_ok(), "one raw unit normalizes");
    assert!(FixedRay3::try_new(v([MAX; 3]), v([MAX, 0, 0])).is_ok(), "extreme origin and axis");
    // A nonzero direction at `i64::MIN` raw is refused: its length, 2^63 raw, exceeds i64::MAX.
    assert_eq!(FixedRay3::try_new(origin, v([MIN, 0, 0])), Err(GeomError::InvalidRayDirection));
}

/// Triangles: `new` accepts collinear and repeated vertices, `try_new` refuses them, and
/// `is_degenerate` is exact at the ends of the raw range.
#[wasm_bindgen_test(unsupported = test)]
fn degen_tp_004_characterization_triangle_constructors() {
    let (a, b, c) = (u([0, 0, 0]), u([1, 0, 0]), u([2, 0, 0]));
    assert!(FixedTriangle3::new(a, b, c).is_degenerate(), "collinear");
    assert!(FixedTriangle3::new(a, a, b).is_degenerate(), "repeated");
    assert!(FixedTriangle3::new(a, a, a).is_degenerate(), "one point");
    assert_eq!(FixedTriangle3::try_new(a, b, c), Err(ShapeError::DegenerateTriangle));
    assert_eq!(FixedTriangle3::try_new(a, a, b), Err(ShapeError::DegenerateTriangle));
    let (lo, hi) = (v([MIN, MIN, MIN]), v([MAX, MAX, MAX]));
    // (MIN, MIN, MIN), (MAX, MAX, MAX) and (-1, -1, -1) lie on the line x = y = z.
    assert!(FixedTriangle3::new(lo, hi, v([-1, -1, -1])).is_degenerate());
    assert!(!FixedTriangle3::new(lo, hi, v([-1, -1, 0])).is_degenerate(), "one raw unit off");
}

/// Oriented boxes: a zero half extent is accepted, a negative one refused, and a zero axis is
/// refused as out of length tolerance.
#[wasm_bindgen_test(unsupported = test)]
fn degen_tp_005_characterization_obb_constructor() {
    let centre = v([0; 3]);
    assert!(FixedObb3::try_new(centre, FixedMat3::IDENTITY, v([0; 3])).is_ok(), "a point");
    assert_eq!(
        FixedObb3::try_new(centre, FixedMat3::IDENTITY, v([0, -1, 0])),
        Err(ShapeError::NegativeHalfExtent { axis: 1 })
    );
    assert_eq!(
        FixedObb3::try_new(centre, zero_mat3(), v([0; 3])),
        Err(ShapeError::AxisLengthOutOfTolerance { column: 0 })
    );
}

/// `bunny-bounds/1` helpers: an empty point set and an inverted box are refused; a degenerate
/// triangle and a zero radius are accepted; results outside the raw range are refused.
#[wasm_bindgen_test(unsupported = test)]
fn degen_tp_006_characterization_bounds_helpers() {
    assert_eq!(FixedAabb3::from_points(&[]), Err(BoundsError::NoPoints));
    let collinear = FixedAabb3::from_triangle([u([0, 0, 0]), u([1, 0, 0]), u([2, 0, 0])]);
    assert_eq!((raw(collinear.min), raw(collinear.max)), ([0; 3], [2 * ONE, 0, 0]));

    let ordered = aabb([0; 3], [2; 3]);
    let inverted = aabb([5, 0, 0], [1, 1, 1]);
    assert_eq!(ordered.merge(&inverted), Err(BoundsError::InvertedBox));
    assert_eq!(inverted.expand(q(0)), Err(BoundsError::InvertedBox));
    assert_eq!(inverted.bounding_sphere(), Err(BoundsError::InvertedBox));

    let negative = FixedSphere3::new(v([0; 3]), q(-1));
    assert_eq!(FixedAabb3::from_sphere(&negative), Err(BoundsError::NegativeRadius));
    let point = FixedAabb3::from_sphere(&FixedSphere3::new(v([7; 3]), q(0)));
    assert_eq!(point.map(|b| (raw(b.min), raw(b.max))), Ok(([7; 3], [7; 3])));

    let full = aabb([MIN; 3], [MAX; 3]);
    assert_eq!(full.expand(q(1)), Err(BoundsError::OutOfRange));
    assert_eq!(full.bounding_sphere(), Err(BoundsError::OutOfRange));
    assert_eq!(ordered.merge(&full), Ok(full));
}

/// A zero normal is a valid plane: every point has signed distance `-offset`, so every point
/// is on the same side.
#[wasm_bindgen_test(unsupported = test)]
fn degen_tp_007_characterization_zero_normal_plane() {
    let on = plane([0; 3], 0);
    let behind = plane([0; 3], 5);
    for p in [v([0; 3]), v([MIN; 3]), v([MAX; 3])] {
        assert_eq!(on.signed_distance(p), Some(q(0)));
        assert_eq!(on.classify(p), Some(PlaneSide::On));
        assert_eq!(behind.classify(p), Some(PlaneSide::Behind));
    }
    let segment = FixedSegment3::new(u([-1; 3]), u([1; 3]));
    assert_eq!(on.clip_segment(segment), Some(SegmentClip::Kept(segment)));
    assert_eq!(behind.clip_segment(segment), Some(SegmentClip::Outside));
}

/// Coincident planes facing each other, `z >= 0` and `-z >= 0`, keep exactly the plane
/// `z = 0`: a point on it is `On` for both, and a frustum made of such pairs is the closed
/// zero-thickness slab.
#[wasm_bindgen_test(unsupported = test)]
fn degen_tp_008_characterization_coincident_planes() {
    let up = plane([0, 0, ONE], 0);
    let down = plane([0, 0, -ONE], 0);
    let on = u([3, 4, 0]);
    assert_eq!((up.classify(on), down.classify(on)), (Some(PlaneSide::On), Some(PlaneSide::On)));
    let slab = FixedFrustum::new([up, down, up, down, up, down]);
    assert_eq!(slab.contains_point(on), Some(true));
    assert_eq!(slab.contains_point(u([3, 4, 1])), Some(false));
    let flat = aabb([0; 3], [ONE, ONE, 0]);
    assert_eq!(slab.classify_aabb(flat), Some(FrustumClass::Inside), "a flat box on the slab");
}

/// Segments against a plane: a zero-length segment takes its point's side, and the touching
/// case of flyingrobots/bunny#472 returns `None` when `end - start` overflows.
///
/// Known defect, flyingrobots/bunny#472: with `start` exactly on the plane and `end` behind
/// it, the exact answer is the on-plane endpoint, but `clip_segment` interpolates and returns
/// `None` when the difference overflows. One raw unit inside the threshold it succeeds.
#[wasm_bindgen_test(unsupported = test)]
fn degen_tp_009_characterization_segment_clip_and_classify() {
    let z = plane([0, 0, ONE], 0);
    let front = FixedSegment3::new(u([1, 1, 1]), u([1, 1, 1]));
    let behind = FixedSegment3::new(u([1, 1, -1]), u([1, 1, -1]));
    let touching = FixedSegment3::new(u([1, 1, 0]), u([1, 1, 0]));
    assert_eq!(z.clip_segment(front), Some(SegmentClip::Kept(front)));
    assert_eq!(z.clip_segment(behind), Some(SegmentClip::Outside));
    assert_eq!(z.clip_segment(touching), Some(SegmentClip::Kept(touching)));
    assert_eq!(z.classify_segment(front), Some(PlaneSpan::Front));
    assert_eq!(z.classify_segment(touching), Some(PlaneSpan::Coplanar));

    let z2 = plane([0, 0, 2 * ONE], 0);
    let overflowing = FixedSegment3::new(v([MAX, 0, 0]), v([-2, 0, -ONE]));
    assert_eq!(z2.clip_segment(overflowing), None, "flyingrobots/bunny#472");
    let inside = FixedSegment3::new(v([MAX, 0, 0]), v([-1, 0, -ONE]));
    let kept = FixedSegment3::new(v([MAX, 0, 0]), v([MAX, 0, 0]));
    assert_eq!(z2.clip_segment(inside), Some(SegmentClip::Kept(kept)));
    // A signed distance that leaves the raw range is `None`, not saturated.
    assert_eq!(plane([ONE, 0, 0], 0).signed_distance(v([MAX, 0, 0])), Some(q(MAX)));
    assert_eq!(plane([2 * ONE, 0, 0], 0).signed_distance(v([MAX, 0, 0])), None);
}

/// Inverted boxes against planes and frusta are read as the box with the same eight corners:
/// the same answer as the swapped box.
#[wasm_bindgen_test(unsupported = test)]
fn degen_tp_010_characterization_inverted_box_against_planes() {
    let swapped = aabb([0, -ONE, -ONE], [ONE, ONE, ONE]);
    let inverted = aabb([ONE, -ONE, -ONE], [0, ONE, ONE]);
    let x_half = plane([ONE, 0, 0], ONE / 2);
    assert_eq!(x_half.classify_aabb(swapped), Some(PlaneSpan::Straddling));
    assert_eq!(x_half.classify_aabb(inverted), Some(PlaneSpan::Straddling));
    let x_far = plane([ONE, 0, 0], 2 * ONE);
    assert_eq!(x_far.classify_aabb(inverted), Some(PlaneSpan::Behind));

    let all = plane([0; 3], 0);
    let frustum = FixedFrustum::new([x_half, all, all, all, all, all]);
    assert_eq!(frustum.classify_aabb(swapped), Some(FrustumClass::Intersecting));
    assert_eq!(frustum.classify_aabb(inverted), Some(FrustumClass::Intersecting));
}

/// Polygons: fewer than three vertices, the empty polygon included, are refused; repeated and
/// collinear vertices are clipped like any others and stay as given.
#[wasm_bindgen_test(unsupported = test)]
fn degen_tp_011_characterization_polygon_clip() {
    let z = plane([0, 0, ONE], 0);
    let mut out = [v([0; 3]); 8];
    assert_eq!(z.clip_polygon(&[], &mut out), Err(PolygonClipError::TooFewVertices));
    let two = [u([0; 3]), u([1, 0, 0])];
    assert_eq!(z.clip_polygon(&two, &mut out), Err(PolygonClipError::TooFewVertices));

    let collinear = [u([0, 0, 1]), u([1, 0, 1]), u([2, 0, 1])];
    assert_eq!(z.clip_polygon(&collinear, &mut out), Ok(PolygonClip::Kept(3)));
    assert_eq!(out[..3], collinear, "returned unchanged");
    let repeated = [u([0, 0, 1]), u([0, 0, 1]), u([0, 0, 1])];
    assert_eq!(z.clip_polygon(&repeated, &mut out), Ok(PolygonClip::Kept(3)));
    let on_plane_point = [u([0; 3]), u([0; 3]), u([0; 3])];
    assert_eq!(z.clip_polygon(&on_plane_point, &mut out), Ok(PolygonClip::Kept(3)));
}

/// Frustum spheres: a negative radius is refused and a zero radius is the point rule. A frustum
/// extracted from the zero matrix has six zero-normal planes and contains every point.
#[wasm_bindgen_test(unsupported = test)]
fn degen_tp_012_characterization_frustum_degenerates() {
    let z = plane([0, 0, ONE], 0);
    let frustum = FixedFrustum::new([z; 6]);
    let negative = FixedSphere3::new(u([0, 0, 5]), q(-1));
    assert_eq!(frustum.classify_sphere(negative), Err(FrustumSphereError::NegativeRadius));
    let point_on = FixedSphere3::new(u([0, 0, 0]), q(0));
    let point_behind = FixedSphere3::new(u([0, 0, -1]), q(0));
    assert_eq!(frustum.classify_sphere(point_on), Ok(FrustumClass::Inside));
    assert_eq!(frustum.classify_sphere(point_behind), Ok(FrustumClass::Outside));

    let zero = (q(0), q(0), q(0), q(0));
    let everything = FixedFrustum::from_clip_matrix(FixedMat4::from_rows(zero, zero, zero, zero));
    let everything = everything.expect("the zero matrix gives six planes");
    for p in [v([0; 3]), v([MIN; 3]), v([MAX; 3])] {
        assert_eq!(everything.contains_point(p), Some(true));
    }
}

/// Transforms: an inverted box is refused, a zero-size box maps to a point box, and a ray under
/// the zero linear part is refused as a degenerate direction.
#[wasm_bindgen_test(unsupported = test)]
fn degen_tp_013_characterization_transforms() {
    let identity = FixedAffine3::IDENTITY;
    let inverted = aabb([ONE, 0, 0], [0, ONE, ONE]);
    assert_eq!(inverted.checked_transform(&identity), Err(TransformError::InvertedBox));
    let point = aabb([7; 3], [7; 3]);
    assert_eq!(point.checked_transform(&identity), Ok(point));

    let collapse = FixedAffine3::from_parts(zero_mat3(), v([0; 3]));
    let ray = FixedRay3::new(v([0; 3]), FixedUnitVec3::UNIT_X);
    assert_eq!(ray.checked_transform(&collapse), Err(TransformError::DegenerateDirection));
}

/// Exact predicates: collinear, coplanar and repeated points are `Collinear` / `Coplanar` at
/// every magnitude, and a degenerate triangle has no barycentric coordinates.
#[wasm_bindgen_test(unsupported = test)]
fn degen_tp_014_characterization_predicates() {
    let p2 = |x: i64, y: i64| FixedVec2::new(q(x), q(y));
    let (lo, hi, mid) = (p2(MIN, MIN), p2(MAX, MAX), p2(-1, -1));
    assert_eq!(orientation2(lo, hi, mid), Orientation2::Collinear);
    assert_eq!(orientation2(lo, lo, hi), Orientation2::Collinear, "repeated");
    assert_eq!(orientation2(lo, hi, p2(-1, 0)), Orientation2::CounterClockwise);

    let (a, b, c) = (v([MIN; 3]), v([MAX, MIN, MIN]), v([MIN, MAX, MIN]));
    assert_eq!(orientation3(a, b, c, v([0, 0, MIN])), Orientation3::Coplanar);
    assert_eq!(orientation3(a, a, b, c), Orientation3::Coplanar, "repeated");

    assert_eq!(barycentric2(p2(0, 0), p2(0, 0), p2(1, 1), p2(2, 2)), None, "collinear");
    assert_eq!(barycentric2(p2(0, 0), p2(0, 0), p2(0, 0), p2(1, 0)), None, "repeated");
    let collinear3 = [u([0; 3]), u([1, 0, 0]), u([2, 0, 0])];
    assert_eq!(
        barycentric3_coplanar(u([1, 0, 0]), collinear3[0], collinear3[1], collinear3[2]),
        None
    );
}
