//! Look-at view matrix contract tests: golden vectors, handedness against the
//! coordinate law, typed errors, and a seeded orthonormality bound. Axis-aligned
//! goldens are exact; diagonal goldens were cross-checked against an independent
//! decimal reference that rounds each unit component to nearest.

use bunny_linalg::{FixedMat4, FixedVec3, LookAtError};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const ONE_RAW: i64 = bunny_num::fixed_q32_32::ONE_RAW;
const HALF_RAW: i64 = ONE_RAW / 2;
const SQRT_HALF_RAW: i64 = 3_037_000_500;
const SEED: u64 = 0xB002_0200_0000_0001;
const CASES: usize = 400;
/// Documented bound: each basis vector's length is within 2 raw units of one.
const LENGTH_BOUND_RAW: i128 = 2;
/// Documented bound: each pairwise dot product is within 2 raw units of zero.
const DOT_BOUND_RAW: i128 = 2;

fn raw(value: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(value)
}

fn v(x: i64, y: i64, z: i64) -> FixedVec3 {
    FixedVec3::new(raw(x * ONE_RAW), raw(y * ONE_RAW), raw(z * ONE_RAW))
}

fn raw_vec(x: i64, y: i64, z: i64) -> FixedVec3 {
    FixedVec3::new(raw(x), raw(y), raw(z))
}

fn rows_raw(matrix: FixedMat4) -> [[i64; 4]; 4] {
    let row = |r: (FixedQ32_32, FixedQ32_32, FixedQ32_32, FixedQ32_32)| {
        [r.0.raw(), r.1.raw(), r.2.raw(), r.3.raw()]
    };
    [row(matrix.row0()), row(matrix.row1()), row(matrix.row2()), row(matrix.row3())]
}

fn look(eye: FixedVec3, target: FixedVec3, up: FixedVec3) -> FixedMat4 {
    FixedMat4::try_look_at(eye, target, up).expect("valid look-at")
}

/// Applies the affine part of `matrix` to `point` with exact i128 arithmetic,
/// returning raw Q32.32 view coordinates rounded toward zero.
fn to_view(matrix: FixedMat4, point: FixedVec3) -> [i128; 3] {
    let p = [point.x.raw(), point.y.raw(), point.z.raw()].map(i128::from);
    let apply = |r: (FixedQ32_32, FixedQ32_32, FixedQ32_32, FixedQ32_32)| {
        let sum = i128::from(r.0.raw()) * p[0]
            + i128::from(r.1.raw()) * p[1]
            + i128::from(r.2.raw()) * p[2];
        (sum >> 32) + i128::from(r.3.raw())
    };
    [apply(matrix.row0()), apply(matrix.row1()), apply(matrix.row2())]
}

fn basis_rows(matrix: FixedMat4) -> [[i128; 3]; 3] {
    let rows = rows_raw(matrix);
    let first3 = |r: [i64; 4]| [r[0], r[1], r[2]].map(i128::from);
    [first3(rows[0]), first3(rows[1]), first3(rows[2])]
}

fn dot_raw(a: [i128; 3], b: [i128; 3]) -> i128 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[wasm_bindgen_test(unsupported = test)]
fn mt_tp_023_axis_aligned_views_have_exact_golden_rows() {
    let y = v(0, 1, 0);
    // Default camera: at the origin looking down -Z is the identity.
    assert_eq!(
        rows_raw(look(v(0, 0, 0), v(0, 0, -1), y)),
        [[ONE_RAW, 0, 0, 0], [0, ONE_RAW, 0, 0], [0, 0, ONE_RAW, 0], [0, 0, 0, ONE_RAW]]
    );
    // Moving the eye back to z = 5 translates the world by -5 in z.
    assert_eq!(rows_raw(look(v(0, 0, 5), v(0, 0, 0), y))[2], [0, 0, ONE_RAW, -5 * ONE_RAW]);
    // Looking along +Z turns the camera around: x and z flip.
    assert_eq!(
        rows_raw(look(v(0, 0, 0), v(0, 0, 1), y)),
        [[-ONE_RAW, 0, 0, 0], [0, ONE_RAW, 0, 0], [0, 0, -ONE_RAW, 0], [0, 0, 0, ONE_RAW]]
    );
    // Looking along +X: right is +Z, back is -X.
    assert_eq!(
        rows_raw(look(v(0, 0, 0), v(4, 0, 0), y)),
        [[0, 0, ONE_RAW, 0], [0, ONE_RAW, 0, 0], [-ONE_RAW, 0, 0, 0], [0, 0, 0, ONE_RAW]]
    );
    // Looking straight down with up = -Z: forward -Y, right +X, true up -Z.
    assert_eq!(
        rows_raw(look(v(0, 3, 0), v(0, 0, 0), v(0, 0, -1))),
        [
            [ONE_RAW, 0, 0, 0],
            [0, 0, -ONE_RAW, 0],
            [0, ONE_RAW, 0, -3 * ONE_RAW],
            [0, 0, 0, ONE_RAW]
        ]
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn mt_tp_024_diagonal_views_have_pinned_golden_rows() {
    // Eye at the origin, target (1, 0, -1): forward is (1/sqrt 2, 0, -1/sqrt 2).
    assert_eq!(
        rows_raw(look(v(0, 0, 0), v(1, 0, -1), v(0, 1, 0))),
        [
            [SQRT_HALF_RAW, 0, SQRT_HALF_RAW, 0],
            [0, ONE_RAW, 0, 0],
            [-SQRT_HALF_RAW, 0, SQRT_HALF_RAW, 0],
            [0, 0, 0, ONE_RAW],
        ]
    );
    // The classic isometric camera from (1, 1, 1) toward the origin.
    assert_eq!(
        rows_raw(look(v(1, 1, 1), v(0, 0, 0), v(0, 1, 0))),
        [
            [SQRT_HALF_RAW, 0, -SQRT_HALF_RAW, 0],
            [-1_753_413_056, 3_506_826_112, -1_753_413_056, 0],
            [2_479_700_525, 2_479_700_525, 2_479_700_525, -7_439_101_575],
            [0, 0, 0, ONE_RAW],
        ]
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn mt_tp_025_view_axis_point_composes_with_projection_per_coordinate_law() {
    let (near, far) = (raw(ONE_RAW), raw(3 * ONE_RAW));
    let proj = FixedMat4::try_perspective(raw(ONE_RAW), raw(2 * ONE_RAW), near, far)
        .expect("valid perspective");
    // Three cameras, each with the point two units straight ahead (distance 2).
    let cases = [
        (v(1, 2, 3), v(1, 2, -1), v(1, 2, 1)),
        (v(0, 0, 0), v(4, 0, 0), v(2, 0, 0)),
        (v(0, 0, 0), v(0, 0, 1), v(0, 0, 2)),
    ];
    for (eye, target, point) in cases {
        let clip = proj.checked_mul_mat4(look(eye, target, v(0, 1, 0))).expect("composes");
        let rows = [clip.row0(), clip.row1(), clip.row2(), clip.row3()];
        let dot = |r: (FixedQ32_32, FixedQ32_32, FixedQ32_32, FixedQ32_32)| {
            let sum = i128::from(r.0.raw()) * i128::from(point.x.raw())
                + i128::from(r.1.raw()) * i128::from(point.y.raw())
                + i128::from(r.2.raw()) * i128::from(point.z.raw());
            raw(i64::try_from((sum >> 32) + i128::from(r.3.raw())).expect("fits"))
        };
        let w = dot(rows[3]);
        // w_clip = -z_view = 2, and depth (3 - 1.5) / 2 = 3/4 per the coordinate law.
        assert_eq!(w.raw(), 2 * ONE_RAW);
        assert_eq!(dot(rows[0]).checked_div(w).expect("x").raw(), 0);
        assert_eq!(dot(rows[1]).checked_div(w).expect("y").raw(), 0);
        assert_eq!(dot(rows[2]).checked_div(w).expect("z").raw(), 3 * HALF_RAW / 2);
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn mt_tp_026_camera_right_up_and_forward_follow_the_right_handed_law() {
    let matrix = look(v(0, 0, 0), v(0, 0, -1), v(0, 1, 0));
    // A point to the camera's right, above it, and in front of it.
    assert_eq!(to_view(matrix, v(1, 0, 0)), [i128::from(ONE_RAW), 0, 0]);
    assert_eq!(to_view(matrix, v(0, 1, 0)), [0, i128::from(ONE_RAW), 0]);
    assert_eq!(to_view(matrix, v(0, 0, -2)), [0, 0, -2 * i128::from(ONE_RAW)]);
    // Eye maps to the origin, and the target lies on the -Z axis, for a diagonal view.
    let diag = look(v(1, 1, 1), v(0, 0, 0), v(0, 1, 0));
    assert_eq!(to_view(diag, v(1, 1, 1)), [0, 0, 0]);
    let [x, y, z] = to_view(diag, v(0, 0, 0));
    assert!(x.abs() <= 2 && y.abs() <= 2, "target off axis: {x} {y}");
    assert!(z < 0, "target must be in front: {z}");
}

#[wasm_bindgen_test(unsupported = test)]
fn mt_tp_027_degenerate_inputs_return_typed_errors() {
    let try_look = FixedMat4::try_look_at;
    let y = v(0, 1, 0);
    assert_eq!(try_look(v(1, 2, 3), v(1, 2, 3), y), Err(LookAtError::CoincidentEyeTarget));
    assert_eq!(try_look(v(0, 0, 0), v(0, 5, 0), y), Err(LookAtError::ParallelUp));
    assert_eq!(try_look(v(0, 0, 0), v(0, -5, 0), y), Err(LookAtError::ParallelUp));
    assert_eq!(try_look(v(0, 0, 0), v(0, 0, -1), v(0, 0, 0)), Err(LookAtError::ParallelUp));
    // A scaled copy of the view direction is parallel too.
    assert_eq!(try_look(v(0, 0, 0), v(1, 1, 0), v(3, 3, 0)), Err(LookAtError::ParallelUp));
    // One raw unit apart is a valid direction, not a coincidence.
    assert!(try_look(raw_vec(0, 0, 0), raw_vec(0, 0, -1), y).is_ok());
}

#[wasm_bindgen_test(unsupported = test)]
fn mt_tp_030_exactly_parallel_up_is_refused_and_near_parallel_is_accepted() {
    let try_look = FixedMat4::try_look_at;
    let origin = v(0, 0, 0);
    // Exactly parallel in whole units and in raw units, with the eye at the origin.
    for scale in [ONE_RAW, 1] {
        let direction = raw_vec(scale, 2 * scale, 3 * scale);
        assert_eq!(try_look(origin, direction, direction), Err(LookAtError::ParallelUp));
        // Antiparallel counts as parallel.
        let reversed = raw_vec(-scale, -2 * scale, -3 * scale);
        assert_eq!(try_look(origin, direction, reversed), Err(LookAtError::ParallelUp));
    }
    // A different scale of the same direction, and a moved eye.
    assert_eq!(
        try_look(v(5, 5, 5), v(6, 7, 8), raw_vec(7 * ONE_RAW, 14 * ONE_RAW, 21 * ONE_RAW)),
        Err(LookAtError::ParallelUp)
    );
    // Raw differences need 65 bits: eye all MIN, target all MAX, up all MAX.
    let (low, high) =
        (raw_vec(i64::MIN, i64::MIN, i64::MIN), raw_vec(i64::MAX, i64::MAX, i64::MAX));
    assert_eq!(try_look(low, high, high), Err(LookAtError::ParallelUp));
    assert_eq!(try_look(low, high, low), Err(LookAtError::ParallelUp));
    // One raw unit off parallel is accepted: only exact parallelism is refused.
    let target = raw_vec(1_000_000, 2_000_000, 3_000_000);
    assert!(try_look(origin, target, raw_vec(1_000_000, 2_000_000, 3_000_001)).is_ok());
}

#[wasm_bindgen_test(unsupported = test)]
fn mt_tp_031_representable_translation_is_kept_and_real_overflow_is_refused() {
    let y = v(0, 1, 0);
    // dot(back, eye) = +2^31 units does not fit, but its negation -2^31 does.
    let far_left = raw_vec(i64::MIN, 0, 0);
    assert_eq!(
        rows_raw(look(far_left, v(0, 0, 0), y)),
        [[0, 0, ONE_RAW, 0], [0, ONE_RAW, 0, 0], [-ONE_RAW, 0, 0, i64::MIN], [0, 0, 0, ONE_RAW]]
    );
    // A diagonal view from far away: the finished entry is 1.41 * 2^63 raw, outside the range.
    let far = raw_vec(i64::MAX, i64::MAX, 0);
    assert_eq!(FixedMat4::try_look_at(far, v(0, 0, 0), v(0, 0, 1)), Err(LookAtError::Overflow));
}

#[wasm_bindgen_test(unsupported = test)]
fn mt_tp_028_look_at_error_messages_are_stable() {
    assert_eq!(
        LookAtError::CoincidentEyeTarget.to_string(),
        "look-at eye and target are the same point"
    );
    assert_eq!(
        LookAtError::ParallelUp.to_string(),
        "look-at up vector is zero or parallel to the view direction"
    );
    assert_eq!(LookAtError::Overflow.to_string(), "look-at arithmetic is outside the Q32.32 range");
}

fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state >> 33
}

/// 62 random bits from two draws of 31.
fn next62(state: &mut u64) -> u64 {
    (next(state) << 31) | next(state)
}

/// A signed value with a random magnitude spanning 1 raw unit to about 2^38.
fn spread(state: &mut u64) -> i64 {
    let bits = next(state) % 38;
    let magnitude = i64::try_from(next62(state) & ((1 << (bits + 1)) - 1)).expect("fits");
    if next(state).is_multiple_of(2) {
        magnitude
    } else {
        -magnitude
    }
}

fn random_vec(state: &mut u64) -> FixedVec3 {
    raw_vec(spread(state), spread(state), spread(state))
}

#[wasm_bindgen_test(unsupported = test)]
fn mt_tp_029_random_views_are_orthonormal_within_the_documented_bound() {
    let mut state = SEED;
    let one = i128::from(ONE_RAW);
    let mut checked = 0;
    for case in 0..CASES {
        let (eye, target, up) =
            (random_vec(&mut state), random_vec(&mut state), random_vec(&mut state));
        let Ok(matrix) = FixedMat4::try_look_at(eye, target, up) else {
            continue; // coincident or parallel by chance; typed, never a panic
        };
        checked += 1;
        let basis = basis_rows(matrix);
        for row in basis {
            // |len - 2^32| <= B  implies |len^2 - 2^64| <= 2 * 2^32 * B + B^2.
            let excess = (dot_raw(row, row) - one * one).abs();
            let allowed = 2 * one * LENGTH_BOUND_RAW + LENGTH_BOUND_RAW * LENGTH_BOUND_RAW;
            assert!(excess <= allowed, "length; case={case} row={row:?}");
        }
        for (a, b) in [(0, 1), (0, 2), (1, 2)] {
            let dot = dot_raw(basis[a], basis[b]) / one;
            assert!(dot.abs() <= DOT_BOUND_RAW, "dot {a}.{b}; case={case} dot={dot}");
        }
        // The eye is the origin of view space, within the checked rounding of three terms.
        let [x, y, z] = to_view(matrix, eye);
        assert!(x.abs().max(y.abs()).max(z.abs()) <= 3, "eye; case={case}");
    }
    assert!(checked > CASES / 2, "too many degenerate draws: {checked}");
}
