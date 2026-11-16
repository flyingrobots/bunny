//! Evidence for `bunny-shape-frame/1`: `ShapeFrame::to_object` and `ShapeFrame::to_world`
//! against an exact rational oracle written here, named geometric cases, ties and refusals.
//!
//! The oracle (`nearest_even`) takes each output as a fraction over `2^64` of exact integers
//! (the raw axes times the raw inputs, summed) and picks the nearest multiple of `2^32` by
//! comparing twice the remainder with the step, ties to the even quotient. It shares no code
//! with the kernel. Generated frames are rotations from integer quaternions, whose matrices
//! are exact rationals, rounded to Q32.32; generated cases print their seed on failure.

use bunny_geom::FixedRay3;
use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::FixedQ32_32;
use bunny_query::{
    ShapeFrame, ShapeFrameError, TraceRay, SHAPE_FRAME_AXIS_DOT_LIMIT_RAW, SHAPE_FRAME_ID,
    TRACE_COORD_LIMIT_RAW, TRACE_DIRECTION_LIMIT_RAW, TRACE_UNIT_DIRECTION_LIMIT_RAW,
};
use wasm_bindgen_test::wasm_bindgen_test;

const ORACLE_SEED: u64 = 0x5AFE_F4A3_0357_0001;
const ORACLE_FRAMES: usize = 300;
const RAYS_PER_FRAME: usize = 40;
const ONE: i64 = 1 << 32;

type Raw3 = [i64; 3];

struct Seeds(u64);

impl Seeds {
    fn next(&mut self) -> u64 {
        self.0 =
            self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    /// A value in `-bound..=bound`.
    fn signed(&mut self, bound: i64) -> i64 {
        let span = u64::try_from(2 * bound + 1).expect("bound is positive");
        i64::try_from(self.next() % span).expect("below the span") - bound
    }

    fn triple(&mut self, bound: i64) -> Raw3 {
        [self.signed(bound), self.signed(bound), self.signed(bound)]
    }
}

fn vec3(raw: Raw3) -> FixedVec3 {
    let [x, y, z] = raw.map(FixedQ32_32::from_raw);
    FixedVec3::new(x, y, z)
}

fn raw3(v: FixedVec3) -> Raw3 {
    [v.x.raw(), v.y.raw(), v.z.raw()]
}

fn unit(raw: Raw3) -> FixedUnitVec3 {
    FixedUnitVec3::try_from_unit(vec3(raw)).expect("a unit vector")
}

/// The multiple of `2^32` nearest `numerator`, ties to the even multiple, as a raw Q32.32
/// value: the oracle's one rounding. `None` when it does not fit `i64`.
fn nearest_even(numerator: i128) -> Option<i64> {
    let step = 1_i128 << 32;
    let quotient = numerator.div_euclid(step);
    let twice_rest = 2 * numerator.rem_euclid(step);
    let up = twice_rest > step || (twice_rest == step && quotient.rem_euclid(2) == 1);
    i64::try_from(if up { quotient + 1 } else { quotient }).ok()
}

/// The exact sum of `a[i] * b[i]`.
fn exact_dot(a: Raw3, b: [i128; 3]) -> i128 {
    (0..3).map(|i| i128::from(a[i]) * b[i]).sum()
}

/// What `to_object` must return for a frame with raw `axes` and `origin`.
fn oracle_to_object(
    axes: [Raw3; 3],
    origin: Raw3,
    ray: (Raw3, Raw3),
) -> Result<(Raw3, Raw3), ShapeFrameError> {
    let offset = [0, 1, 2].map(|i| i128::from(ray.0[i]) - i128::from(origin[i]));
    let direction = ray.1.map(i128::from);
    let object = |v: [i128; 3]| axes.map(|axis| nearest_even(exact_dot(axis, v)));
    let o = object(offset);
    if o.iter().any(|c| c.is_none_or(|c| c.unsigned_abs() >= TRACE_COORD_LIMIT_RAW.unsigned_abs()))
    {
        return Err(ShapeFrameError::RayOutsideTraceDomain);
    }
    let d = object(direction);
    let limit = TRACE_UNIT_DIRECTION_LIMIT_RAW.unsigned_abs();
    if d.iter().any(|c| c.is_none_or(|c| c.unsigned_abs() > limit)) || d == [Some(0); 3] {
        return Err(ShapeFrameError::DirectionOutsideTraceDomain);
    }
    Ok((o.map(Option::unwrap), d.map(Option::unwrap)))
}

/// What `to_world` must return.
fn oracle_to_world(axes: [Raw3; 3], normal: Raw3) -> Result<Raw3, ShapeFrameError> {
    let n = normal.map(i128::from);
    let world = [0, 1, 2].map(|j| nearest_even(exact_dot([axes[0][j], axes[1][j], axes[2][j]], n)));
    match world {
        [Some(x), Some(y), Some(z)] => Ok([x, y, z]),
        _ => Err(ShapeFrameError::NormalOutOfRange),
    }
}

/// The rows of the rotation matrix of the integer quaternion `q`, each entry the exact
/// rational over `|q|^2` rounded to the nearest raw unit; `None` when a row is not accepted
/// as a unit vector.
#[allow(clippy::many_single_char_names, reason = "the quaternion's conventional names")]
fn quaternion_axes(q: [i64; 4]) -> Option<[Raw3; 3]> {
    let [w, x, y, z] = q.map(i128::from);
    let n = w * w + x * x + y * y + z * z;
    let m = [
        [w * w + x * x - y * y - z * z, 2 * (x * y - w * z), 2 * (x * z + w * y)],
        [2 * (x * y + w * z), w * w - x * x + y * y - z * z, 2 * (y * z - w * x)],
        [2 * (x * z - w * y), 2 * (y * z + w * x), w * w - x * x - y * y + z * z],
    ];
    let round = |e: i128| -> i64 {
        let scaled = e << 32;
        let q = scaled.div_euclid(n);
        let r = scaled.rem_euclid(n);
        i64::try_from(if 2 * r >= n { q + 1 } else { q }).expect("an entry is at most 1")
    };
    let rows = m.map(|row| row.map(round));
    rows.iter().all(|&r| FixedUnitVec3::try_from_unit(vec3(r)).is_some()).then_some(rows)
}

fn frame(axes: [Raw3; 3], origin: Raw3) -> ShapeFrame {
    ShapeFrame::new(axes.map(unit), vec3(origin)).expect("a valid frame")
}

fn object_ray(result: Result<TraceRay, ShapeFrameError>) -> Result<(Raw3, Raw3), ShapeFrameError> {
    result.map(|ray| (raw3(ray.origin()), raw3(ray.direction())))
}

#[wasm_bindgen_test]
#[test]
fn sf_tp_009_contract_identity_is_versioned() {
    assert_eq!(SHAPE_FRAME_ID, "bunny-shape-frame/1");
}

/// SF-TP-001: every result of both maps on generated rotations, translations,
/// rays and normals equals the exact rational oracle, refusals included.
#[wasm_bindgen_test]
#[test]
fn sf_tp_001_maps_equal_the_exact_rational_oracle() {
    let mut seeds = Seeds(ORACLE_SEED);
    let (mut frames, mut refusals, mut maps) = (0, 0, 0);
    while frames < ORACLE_FRAMES {
        let q = [0; 4].map(|_| seeds.signed(1 << 15));
        let Some(axes) = quaternion_axes(q) else { continue };
        frames += 1;
        let origin = seeds.triple(TRACE_COORD_LIMIT_RAW - 1);
        let shape = frame(axes, origin);
        for case in 0..RAYS_PER_FRAME {
            let ray_origin = seeds.triple(TRACE_COORD_LIMIT_RAW - 1);
            let direction = seeds.triple(TRACE_DIRECTION_LIMIT_RAW >> (case % 33));
            let Some(ray) = TraceRay::new(vec3(ray_origin), vec3(direction)) else { continue };
            let expected = oracle_to_object(axes, origin, (ray_origin, direction));
            refusals += usize::from(expected.is_err());
            maps += usize::from(expected.is_ok());
            let normal = seeds.triple(i64::MAX >> (1 + case % 40));
            let context = format!("seed {ORACLE_SEED:#x}, frame {frames}, case {case}");
            assert_eq!(object_ray(shape.to_object(&ray)), expected, "to_object, {context}");
            assert_eq!(
                shape.to_world(vec3(normal)).map(raw3),
                oracle_to_world(axes, normal),
                "to_world, {context}"
            );
        }
    }
    assert!(refusals > 0 && maps > 1000, "seed {ORACLE_SEED:#x}: {maps} maps, {refusals} refusals");
}

/// SF-TP-003: a grazing ray, travelling in the object's z = 0 plane at height zero, maps to
/// an object ray with zero height and zero vertical direction exactly; a nearly grazing one
/// under a generated rotation equals the oracle.
#[wasm_bindgen_test]
#[test]
fn sf_tp_003_grazing_ray_stays_in_the_object_plane() {
    // x -> world y, y -> world z, z -> world x: right-handed.
    let axes = [[0, ONE, 0], [0, 0, ONE], [ONE, 0, 0]];
    let origin = [3 * ONE, -2 * ONE, 5 * ONE];
    let shape = frame(axes, origin);
    let ray = TraceRay::new(vec3([3 * ONE, ONE, 7 * ONE]), vec3([0, 1 << 31, -(1 << 30)]))
        .expect("a trace ray");
    let (o, d) = object_ray(shape.to_object(&ray)).expect("in the domain");
    assert_eq!((o, d), ([3 * ONE, 2 * ONE, 0], [1 << 31, -(1 << 30), 0]));
    let axes = quaternion_axes([3, -7, 11, 5]).expect("unit rows");
    let shape = frame(axes, origin);
    let world = |v: Raw3| oracle_to_world(axes, v).expect("small");
    let (start, along) = (world([ONE, 0, 1]), world([ONE >> 1, ONE >> 2, 1]));
    let start = [0, 1, 2].map(|i| start[i] + origin[i]);
    let ray = TraceRay::new(vec3(start), vec3(along)).expect("a trace ray");
    assert_eq!(object_ray(shape.to_object(&ray)), oracle_to_object(axes, origin, (start, along)));
}

/// SF-TP-004: a ray starting at the frame's origin has the object origin zero exactly.
#[wasm_bindgen_test]
#[test]
fn sf_tp_004_ray_from_the_frame_origin_starts_at_object_zero() {
    let axes = quaternion_axes([2, 1, -4, 9]).expect("unit rows");
    let origin = [-100 * ONE, 17 * ONE + 3, 255 * ONE];
    let ray =
        TraceRay::new(vec3(origin), vec3([1 << 31, -(1 << 31), 1 << 20])).expect("a trace ray");
    let (o, d) = object_ray(frame(axes, origin).to_object(&ray)).expect("in the domain");
    assert_eq!(o, [0; 3]);
    assert_eq!(
        Ok((o, d)),
        oracle_to_object(axes, origin, (origin, [1 << 31, -(1 << 31), 1 << 20]))
    );
}

/// SF-TP-005: a ray whose frame origin is behind it maps to an object ray that also moves
/// away from object zero: under an exact (axis-permuting) frame the object origin and
/// direction are the world offset and direction permuted, so `origin . direction > 0` holds
/// in both spaces.
#[wasm_bindgen_test]
#[test]
fn sf_tp_005_frame_origin_behind_the_ray_stays_behind() {
    let axes = [[0, 0, ONE], [ONE, 0, 0], [0, ONE, 0]];
    let origin = [ONE, ONE, ONE];
    let ray =
        TraceRay::new(vec3([4 * ONE, ONE, -ONE]), vec3([ONE, 0, -(1 << 31)])).expect("a trace ray");
    let (o, d) = object_ray(frame(axes, origin).to_object(&ray)).expect("in the domain");
    assert_eq!((o, d), ([-2 * ONE, 3 * ONE, 0], [-(1 << 31), ONE, 0]));
    assert!(exact_dot(o, d.map(i128::from)) > 0, "the object ray moves away from zero");
}

/// SF-TP-006: ties round to even. The x axis `(0.5, sqrt(3)/2, 0)` (rounded) has the raw x
/// component `2^31`, so a raw offset of 1 or 3 along world x is half a raw unit or one and
/// a half: 0 and 2.
#[wasm_bindgen_test]
#[test]
fn sf_tp_006_ties_round_to_even() {
    let s = 3_719_550_787; // round(sqrt(3) / 2 * 2^32)
    let axes = [[1 << 31, s, 0], [-s, 1 << 31, 0], [0, 0, ONE]];
    let shape = frame(axes, [0; 3]);
    for (offset, expected) in [(1, 0), (3, 2), (-1, 0), (-3, -2)] {
        let ray = TraceRay::new(vec3([offset, 0, 0]), vec3([0, 0, ONE])).expect("a trace ray");
        let (o, _) = object_ray(shape.to_object(&ray)).expect("in the domain");
        assert_eq!(o[0], expected, "offset {offset}");
        assert_eq!(Ok(o[0]), nearest_even(i128::from(offset) << 31).ok_or(()), "offset {offset}");
        let world = shape.to_world(vec3([offset, 0, 0])).map(raw3).expect("small");
        assert_eq!(world[0], expected, "to_world, offset {offset}");
    }
}

/// SF-TP-007: frames outside the trace domain, with non-orthogonal or left-handed axes are
/// refused with their typed error.
#[wasm_bindgen_test]
#[test]
fn sf_tp_007_invalid_frames_are_refused() {
    let [x, y, z] = [FixedUnitVec3::UNIT_X, FixedUnitVec3::UNIT_Y, FixedUnitVec3::UNIT_Z];
    let at = |c: i64| vec3([0, c, 0]);
    assert_eq!(
        ShapeFrame::new([x, y, z], at(TRACE_COORD_LIMIT_RAW)),
        Err(ShapeFrameError::OriginOutsideTraceDomain)
    );
    assert_eq!(
        ShapeFrame::new([x, y, z], at(-TRACE_COORD_LIMIT_RAW)),
        Err(ShapeFrameError::OriginOutsideTraceDomain)
    );
    assert!(ShapeFrame::new([x, y, z], at(TRACE_COORD_LIMIT_RAW - 1)).is_ok());
    assert_eq!(ShapeFrame::new([x, z, y], at(0)), Err(ShapeFrameError::AxesLeftHanded));
    assert_eq!(ShapeFrame::new([x, x, z], at(0)), Err(ShapeFrameError::AxesNotOrthogonal));
    // A y axis tilted toward x until the exact dot is just over, then exactly at, the limit.
    let tilt = |dx: i64| unit([dx, ONE, 0]);
    let at_limit = i64::try_from(SHAPE_FRAME_AXIS_DOT_LIMIT_RAW >> 32).expect("small");
    assert_eq!(
        ShapeFrame::new([x, tilt(at_limit + 1), z], at(0)),
        Err(ShapeFrameError::AxesNotOrthogonal)
    );
    assert!(ShapeFrame::new([x, tilt(at_limit), z], at(0)).is_ok());
}

/// SF-TP-008: object rays outside the trace domain and world normals outside Q32.32 are
/// refused with their typed error; nothing wraps or panics.
#[wasm_bindgen_test]
#[test]
fn sf_tp_008_unrepresentable_results_are_refused() {
    let near_edge = TRACE_COORD_LIMIT_RAW - 1;
    let shape = ShapeFrame::translation(vec3([-near_edge, 0, 0])).expect("in the domain");
    let ray = TraceRay::new(vec3([near_edge, 0, 0]), vec3([ONE, 0, 0])).expect("a trace ray");
    assert_eq!(shape.to_object(&ray), Err(ShapeFrameError::RayOutsideTraceDomain));
    let ray = TraceRay::new(vec3([-1, 0, 0]), vec3([ONE, 0, 0])).expect("a trace ray");
    assert!(shape.to_object(&ray).is_ok());
    // Rotated 45 degrees about z, the direction (1, 1, 0) has object x of sqrt(2).
    let h = 3_037_000_500; // round(sqrt(2) / 2 * 2^32)
    let axes = [[h, h, 0], [-h, h, 0], [0, 0, ONE]];
    let shape = frame(axes, [0; 3]);
    let ray = TraceRay::new(vec3([0; 3]), vec3([ONE, ONE, 0])).expect("a trace ray");
    assert_eq!(shape.to_object(&ray), Err(ShapeFrameError::DirectionOutsideTraceDomain));
    // The longest unit direction, `2^32 + 1` raw, maps to itself under the world axes and is
    // accepted; under an x axis of that length it becomes `2^32 + 2` raw and is refused.
    let longest = unit([TRACE_UNIT_DIRECTION_LIMIT_RAW, 0, 0]);
    let ray = TraceRay::from_ray(&FixedRay3::new(vec3([0; 3]), longest)).expect("a trace ray");
    let identity = ShapeFrame::translation(vec3([0; 3])).expect("in the domain");
    let mapped = object_ray(identity.to_object(&ray)).expect("at the limit");
    assert_eq!(mapped.1, [TRACE_UNIT_DIRECTION_LIMIT_RAW, 0, 0]);
    let long_x = frame([[TRACE_UNIT_DIRECTION_LIMIT_RAW, 0, 0], [0, ONE, 0], [0, 0, ONE]], [0; 3]);
    assert_eq!(long_x.to_object(&ray), Err(ShapeFrameError::DirectionOutsideTraceDomain));
    let big = shape.to_world(vec3([i64::MAX, i64::MAX, 0]));
    assert_eq!(big, Err(ShapeFrameError::NormalOutOfRange));
    assert_eq!(big.map(raw3), oracle_to_world(axes, [i64::MAX, i64::MAX, 0]));
}
