//! Projection matrix contract tests: golden vectors, NDC depth range, typed
//! errors, and seeded properties. Every golden value is hand-computable from
//! powers of two and is compared as exact raw Q32.32.

use bunny_linalg::{FixedMat4, ProjectionError};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const ONE_RAW: i64 = bunny_num::fixed_q32_32::ONE_RAW;
const HALF_RAW: i64 = ONE_RAW / 2;
const QUARTER_RAW: i64 = ONE_RAW / 4;
const SEED: u64 = 0xB002_0201_0000_0001;
const CASES: usize = 96;

fn q(value: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(value * ONE_RAW)
}

fn raw(value: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(value)
}

fn rows_raw(matrix: FixedMat4) -> [[i64; 4]; 4] {
    let row = |r: (FixedQ32_32, FixedQ32_32, FixedQ32_32, FixedQ32_32)| {
        [r.0.raw(), r.1.raw(), r.2.raw(), r.3.raw()]
    };
    [row(matrix.row0()), row(matrix.row1()), row(matrix.row2()), row(matrix.row3())]
}

fn dot(row: (FixedQ32_32, FixedQ32_32, FixedQ32_32, FixedQ32_32), p: [FixedQ32_32; 3]) -> i64 {
    let x = row.0.checked_mul(p[0]).expect("x term");
    let y = row.1.checked_mul(p[1]).expect("y term");
    let z = row.2.checked_mul(p[2]).expect("z term");
    let sum = x.checked_add(y).and_then(|s| s.checked_add(z)).expect("sum");
    sum.checked_add(row.3).expect("w term").raw()
}

/// Applies `matrix` to the point `(x, y, z, 1)` and divides by `w`.
fn ndc(matrix: FixedMat4, point: [FixedQ32_32; 3]) -> [i64; 3] {
    let w = raw(dot(matrix.row3(), point));
    let divide = |row| {
        raw(dot(row, point)).checked_div(w).expect("w is nonzero for points in front").raw()
    };
    [divide(matrix.row0()), divide(matrix.row1()), divide(matrix.row2())]
}

fn ortho(hw: i64, hh: i64, near: i64, far: i64) -> FixedMat4 {
    FixedMat4::try_orthographic(q(hw), q(hh), q(near), q(far)).expect("valid orthographic")
}

fn persp(tan: FixedQ32_32, aspect: FixedQ32_32, near: i64, far: i64) -> FixedMat4 {
    FixedMat4::try_perspective(tan, aspect, q(near), q(far)).expect("valid perspective")
}

#[wasm_bindgen_test(unsupported = test)]
fn mt_tp_015_orthographic_matrix_has_exact_golden_rows() {
    // half extents 2 x 1, near 1, far 3: depth span 2, so 1/2 and -1/2 are exact.
    let matrix = ortho(2, 1, 1, 3);
    assert_eq!(
        rows_raw(matrix),
        [
            [HALF_RAW, 0, 0, 0],
            [0, ONE_RAW, 0, 0],
            [0, 0, -HALF_RAW, -HALF_RAW],
            [0, 0, 0, ONE_RAW],
        ]
    );
    // near 0 is allowed for an orthographic volume: m23 is zero.
    let flush = ortho(1, 1, 0, 4);
    assert_eq!(rows_raw(flush)[2], [0, 0, -QUARTER_RAW, 0]);
}

#[wasm_bindgen_test(unsupported = test)]
fn mt_tp_016_orthographic_frustum_corners_map_to_exact_ndc() {
    let matrix = ortho(2, 1, 1, 3);
    for (sx, sy) in [(-1, -1), (-1, 1), (1, -1), (1, 1)] {
        let near_corner = [q(2 * sx), q(sy), q(-1)];
        let far_corner = [q(2 * sx), q(sy), q(-3)];
        assert_eq!(ndc(matrix, near_corner), [sx * ONE_RAW, sy * ONE_RAW, 0]);
        assert_eq!(ndc(matrix, far_corner), [sx * ONE_RAW, sy * ONE_RAW, ONE_RAW]);
    }
    assert_eq!(ndc(matrix, [q(0), q(0), q(-2)]), [0, 0, HALF_RAW]);
}

#[wasm_bindgen_test(unsupported = test)]
fn mt_tp_017_perspective_matrix_has_exact_golden_rows() {
    // tan 1, aspect 2, near 1, far 3.
    let wide = persp(q(1), q(2), 1, 3);
    assert_eq!(
        rows_raw(wide),
        [
            [HALF_RAW, 0, 0, 0],
            [0, ONE_RAW, 0, 0],
            [0, 0, -3 * HALF_RAW, -3 * HALF_RAW],
            [0, 0, -ONE_RAW, 0],
        ]
    );
    // tan 1/2, aspect 1/2 (below 1), near 2, far 6: m00 = 4, m11 = 2.
    let narrow = persp(raw(HALF_RAW), raw(HALF_RAW), 2, 6);
    assert_eq!(
        rows_raw(narrow),
        [
            [4 * ONE_RAW, 0, 0, 0],
            [0, 2 * ONE_RAW, 0, 0],
            [0, 0, -3 * HALF_RAW, -3 * ONE_RAW],
            [0, 0, -ONE_RAW, 0],
        ]
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn mt_tp_018_perspective_frustum_corners_map_to_exact_ndc() {
    let matrix = persp(q(1), q(2), 1, 3);
    for (sx, sy) in [(-1, -1), (-1, 1), (1, -1), (1, 1)] {
        // At distance d the half extents are d * 2 (x) and d * 1 (y).
        let near_corner = [q(2 * sx), q(sy), q(-1)];
        let far_corner = [q(6 * sx), q(3 * sy), q(-3)];
        assert_eq!(ndc(matrix, near_corner), [sx * ONE_RAW, sy * ONE_RAW, 0]);
        assert_eq!(ndc(matrix, far_corner), [sx * ONE_RAW, sy * ONE_RAW, ONE_RAW]);
    }
    // Depth is nonlinear: z_view = -2 gives (3 - 1.5) / 2 = 3/4.
    assert_eq!(ndc(matrix, [q(0), q(0), q(-2)]), [0, 0, 3 * QUARTER_RAW]);
    // Aspect below 1: near half extents are 2 * 1/2 * 1/2 = 1/2 (x) and 1 (y).
    let narrow = persp(raw(HALF_RAW), raw(HALF_RAW), 2, 6);
    assert_eq!(ndc(narrow, [raw(HALF_RAW), q(1), q(-2)]), [ONE_RAW, ONE_RAW, 0]);
    // At distance 6 the half extents are 6 * 1/2 * 1/2 = 3/2 (x) and 3 (y).
    assert_eq!(ndc(narrow, [raw(-3 * HALF_RAW), q(-3), q(-6)]), [-ONE_RAW, -ONE_RAW, ONE_RAW]);
}

#[wasm_bindgen_test(unsupported = test)]
fn cl_tp_004_near_plane_maps_to_zero_and_far_plane_to_one() {
    let o = ortho(1, 1, 2, 6);
    let p = persp(q(1), q(1), 2, 6);
    assert_eq!(ndc(o, [q(0), q(0), q(-2)])[2], 0);
    assert_eq!(ndc(o, [q(0), q(0), q(-6)])[2], ONE_RAW);
    assert_eq!(ndc(p, [q(0), q(0), q(-2)])[2], 0);
    assert_eq!(ndc(p, [q(0), q(0), q(-6)])[2], ONE_RAW);
}

#[wasm_bindgen_test(unsupported = test)]
fn cl_tp_005_camera_looks_down_negative_z_with_positive_y_up() {
    let p = persp(q(1), q(1), 1, 3);
    // +x and +y in view space stay positive in NDC.
    let [x, y, _] = ndc(p, [q(1), q(1), q(-2)]);
    assert_eq!([x, y], [HALF_RAW, HALF_RAW]);
    // A point in front is inside [0, 1]; a point behind the camera has w < 0
    // (w = -z_view), so it is not in front of the camera.
    let in_front = ndc(p, [q(0), q(0), q(-2)])[2];
    assert!((0..=ONE_RAW).contains(&in_front));
    assert!(raw(dot(p.row3(), [q(0), q(0), q(2)])) < FixedQ32_32::ZERO);
}

#[wasm_bindgen_test(unsupported = test)]
fn mt_tp_019_orthographic_rejects_invalid_parameters() {
    let try_o = FixedMat4::try_orthographic;
    let (zero, neg) = (FixedQ32_32::ZERO, q(-1));
    assert_eq!(try_o(zero, q(1), q(1), q(3)), Err(ProjectionError::NonPositiveExtent));
    assert_eq!(try_o(q(1), neg, q(1), q(3)), Err(ProjectionError::NonPositiveExtent));
    assert_eq!(try_o(q(1), q(1), neg, q(3)), Err(ProjectionError::InvalidNear));
    assert_eq!(try_o(q(1), q(1), q(3), q(3)), Err(ProjectionError::InvalidDepthRange));
    assert_eq!(try_o(q(1), q(1), q(4), q(3)), Err(ProjectionError::InvalidDepthRange));
    // 1 / (1 raw unit) = 2^32, which is outside the Q32.32 range.
    assert_eq!(try_o(raw(1), q(1), q(1), q(3)), Err(ProjectionError::Overflow));
    assert_eq!(try_o(q(1), raw(1), q(1), q(3)), Err(ProjectionError::Overflow));
    // near 0 is valid: the error set is exactly the invalid cases above.
    assert!(try_o(q(1), q(1), zero, q(3)).is_ok());
}

#[wasm_bindgen_test(unsupported = test)]
fn mt_tp_020_perspective_rejects_invalid_parameters() {
    let try_p = FixedMat4::try_perspective;
    let (zero, neg) = (FixedQ32_32::ZERO, q(-1));
    assert_eq!(try_p(zero, q(1), q(1), q(3)), Err(ProjectionError::NonPositiveExtent));
    assert_eq!(try_p(neg, q(1), q(1), q(3)), Err(ProjectionError::NonPositiveExtent));
    assert_eq!(try_p(q(1), zero, q(1), q(3)), Err(ProjectionError::NonPositiveExtent));
    assert_eq!(try_p(q(1), neg, q(1), q(3)), Err(ProjectionError::NonPositiveExtent));
    assert_eq!(try_p(q(1), q(1), zero, q(3)), Err(ProjectionError::InvalidNear));
    assert_eq!(try_p(q(1), q(1), neg, q(3)), Err(ProjectionError::InvalidNear));
    assert_eq!(try_p(q(1), q(1), q(3), q(3)), Err(ProjectionError::InvalidDepthRange));
    assert_eq!(try_p(q(1), q(1), q(4), q(3)), Err(ProjectionError::InvalidDepthRange));
    // far * near = 3e9 exceeds the Q32.32 maximum of about 2.1e9.
    assert_eq!(try_p(q(1), q(1), q(50_000), q(60_000)), Err(ProjectionError::Overflow));
    // The same product just inside the range succeeds.
    assert!(try_p(q(1), q(1), q(40_000), q(50_000)).is_ok());
    // tan * aspect rounds to zero, so its reciprocal does not exist.
    assert_eq!(try_p(raw(1), raw(1), q(1), q(3)), Err(ProjectionError::Overflow));
    // 1 / tan overflows.
    assert_eq!(try_p(raw(1), q(1), q(1), q(3)), Err(ProjectionError::Overflow));
}

#[wasm_bindgen_test(unsupported = test)]
fn mt_tp_021_projection_errors_have_stable_messages() {
    assert_eq!(
        ProjectionError::InvalidDepthRange.to_string(),
        "projection near distance must be less than far distance"
    );
    assert_eq!(
        ProjectionError::Overflow.to_string(),
        "projection arithmetic is outside the Q32.32 range"
    );
}

fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state >> 33
}

fn bounded(state: &mut u64, lo: u64, hi: u64) -> i64 {
    i64::try_from(lo + next(state) % (hi - lo + 1)).expect("bounded")
}

#[wasm_bindgen_test(unsupported = test)]
fn mt_tp_022_random_valid_parameters_keep_center_and_depth_endpoints() {
    let mut state = SEED;
    for case in 0..CASES {
        let near = bounded(&mut state, 1, 16);
        let far = near + bounded(&mut state, 1, 48);
        let (hw, hh) = (bounded(&mut state, 1, 8), bounded(&mut state, 1, 8));
        let tan = raw(bounded(&mut state, 1, 16) * QUARTER_RAW);
        let aspect = raw(bounded(&mut state, 1, 12) * QUARTER_RAW);
        // m22 and m23 each carry at most half a raw unit of rounding, and an
        // orthographic matrix multiplies m22 by the depth (w is 1), so the
        // depth error is at most far / 2 + 1 raw units; two more are slack.
        let tolerance = far / 2 + 3;
        for matrix in [ortho(hw, hh, near, far), persp(tan, aspect, near, far)] {
            let at_near = ndc(matrix, [q(0), q(0), q(-near)]);
            let at_far = ndc(matrix, [q(0), q(0), q(-far)]);
            assert_eq!(&at_near[..2], &[0, 0], "center at near; case={case}");
            assert_eq!(&at_far[..2], &[0, 0], "center at far; case={case}");
            assert!(at_near[2].abs() <= tolerance, "near depth; case={case}");
            assert!((at_far[2] - ONE_RAW).abs() <= tolerance, "far depth; case={case}");
        }
    }
}
