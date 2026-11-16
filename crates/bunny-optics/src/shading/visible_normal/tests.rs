//! The rounding of each step of the visible-normal construction, pinned on values derived by
//! hand in each case. Oracle: the arithmetic written in the comments.

use bunny_linalg::FixedUnitVec3;
use bunny_num::{FixedAngle, FixedQ32_32};
use wasm_bindgen_test::wasm_bindgen_test;

use super::{disk_point, floor_root, incidence_axes, rounded_cosine, stretched, tilt, warp, View};
use crate::shading::frame::ShadingFrame;
use crate::shading::wide::ONE;

const HALF: i128 = 1 << 31;
/// `2^32 * 3/5 = 2 576 980 377.6` and `2^32 * 4/5 = 3 435 973 836.8`, to nearest.
const THREE_FIFTHS: i128 = 2_576_980_378;
const FOUR_FIFTHS: i128 = 3_435_973_837;
/// `2^32 / sqrt(2) = 3 037 000 499.97...` to nearest, and floored.
const HALF_SQRT2: i128 = 3_037_000_500;
const HALF_SQRT2_FLOOR: i128 = 3_037_000_499;
/// `2^32 sqrt(3) / 2 = 3 719 550 786.6...`, floored.
const HALF_SQRT3_FLOOR: i128 = 3_719_550_786;

/// The frame of the normal `+Y`: tangent `+X`, bitangent `-Z`, exactly.
fn ground() -> ShadingFrame {
    ShadingFrame::new(FixedUnitVec3::UNIT_Y)
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_150_the_cosine_is_rounded_to_nearest_ties_to_even_and_clamped() {
    let cosine = |a: i128, b: i128| rounded_cosine([a, 0, 0], [b, 0, 0]);
    // a * b / 2^32 with a = 1: b = 2^31 + 1 is just above one half, so 1 (a floor gives 0).
    assert_eq!(cosine(1, HALF + 1), 1);
    assert_eq!(cosine(1, HALF - 1), 0);
    // Ties: 0.5 -> 0, 1.5 -> 2, 2.5 -> 2 (half up gives 1, 2, 3; half down 0, 1, 2).
    assert_eq!([1, 3, 5].map(|k| cosine(1, k * HALF)), [0, 2, 2]);
    // Clamped to 0..=1.0: a negative product, and (2^32 + 1)^2 / 2^32 above 1.0.
    assert_eq!(cosine(-1, 3 * HALF), 0);
    assert_eq!(cosine(ONE + 1, ONE + 1), ONE);
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_151_the_disk_radius_is_floored_and_its_products_are_rounded_to_even() {
    let point = |u: i64, v: i64| disk_point([u, v].map(FixedQ32_32::from_raw));
    // u = 1/2 at angle zero (cos = 1.0 exactly): sqrt(1/2) = 3 037 000 499.97..., floored.
    assert_eq!(point(1 << 31, 0), [HALF_SQRT2_FLOOR, 0]);
    // The premise of the next two: at an eighth of a turn sin = cos = 3 037 000 500 raw.
    let (sin, cos) = FixedAngle::from_turns(FixedQ32_32::from_raw(1 << 29)).sin_cos();
    assert_eq!([sin.raw(), cos.raw()].map(i128::from), [HALF_SQRT2; 2]);
    // u = 1/64: the radius is 2^29 exactly, and 2^29 * 3 037 000 500 / 2^32 =
    // 379 625 062.5, a tie: to even, 379 625 062 (half up gives ...063).
    assert_eq!(point(1 << 26, 1 << 29), [379_625_062; 2]);
    // u = 9/64: the radius is 3 * 2^29, the product 1 138 875 187.5: to even, ...188 (a floor
    // gives ...187).
    assert_eq!(point(9 << 26, 1 << 29), [1_138_875_188; 2]);
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_152_the_incidence_axes_are_the_horizontal_view_and_its_perpendicular() {
    // d = (3, -4, 0) / 8 onto the ground: the horizontal part of -d is along -X, and
    // d x n = (0, 0, 3/8) is along +Z; (-X, +Z, +Y) is right-handed.
    let axes = incidence_axes([3 << 29, -(4 << 29), 0], &ground());
    assert_eq!(axes, ([-ONE, 0, 0], [0, 0, ONE]));
    // d = (1, -1, 1): the horizontal part of -d is (-1, 0, -1) / sqrt(2), each component
    // -3 037 000 499.97..., to nearest -3 037 000 500; d x n = (-1, 0, 1), normalized alike.
    let axes = incidence_axes([ONE, -ONE, ONE], &ground());
    assert_eq!(axes, ([-HALF_SQRT2, 0, -HALF_SQRT2], [-HALF_SQRT2, 0, HALF_SQRT2]));
    // Parallel to the normal: (-bitangent, tangent) = (+Z, +X).
    assert_eq!(incidence_axes([0, -ONE, 0], &ground()), ([0, 0, ONE], [ONE, 0, 0]));
    assert_eq!(incidence_axes([0, -5, 0], &ground()), ([0, 0, ONE], [ONE, 0, 0]));
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_153_the_view_holds_the_cosine_and_the_sine_of_the_incoming_direction() {
    // d = (3, -4, 0) / 8: v = (-3/5, 4/5, 0), so the cosine is 4/5 and the sine 3/5, each to
    // nearest (floors give ...836 and ...377).
    let view = View::new([3 << 29, -(4 << 29), 0], &ground()).expect("above the horizon");
    assert_eq!((view.cosine(), view.sine), (FOUR_FIFTHS, THREE_FIFTHS));
    assert_eq!(view.cosine_against([-ONE, 0, 0]), THREE_FIFTHS);
    // Along the surface and from below: the cosine is zero, so there is no view.
    assert!(View::new([ONE, 0, 0], &ground()).is_none());
    assert!(View::new([0, ONE, 0], &ground()).is_none());
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_154_the_stretched_view_is_normalized_to_nearest() {
    // (3, 0, 4) at any scale: (3/5, 4/5), to nearest.
    assert_eq!(stretched(3, 4), (THREE_FIFTHS, FOUR_FIFTHS));
    assert_eq!(stretched(3 << 62, 4 << 62), (THREE_FIFTHS, FOUR_FIFTHS));
    assert_eq!(stretched(0, ONE), (0, ONE));
    assert_eq!(stretched(0, 0), (0, ONE));
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_155_the_warp_floors_its_roots_and_rounds_its_mean_to_even() {
    // kappa = 0 and t1 = 1/2: q = floor(sqrt(3/4)) = 3 719 550 786 (even), and
    // t2 = (q + t2') / 2.
    // t2' = 0: t2 = 1 859 775 393 exactly; were q rounded to nearest (...787) it would be
    // ...393.5 and round to ...394. Then 1 - t1^2 - t2^2 = 10 376 293 542 873 859 263 / 2^64,
    // between 3 221 225 472^2 = 10 376 293 541 461 622 784 and the next square (6 442 450 945
    // more), so t3 = 3 221 225 472.
    assert_eq!(warp(0, HALF, 0), (1_859_775_393, 3_221_225_472));
    // t2' = 1 and t2' = 3: (q + 1) / 2 = ...393.5 and (q + 3) / 2 = ...394.5, both ties, both
    // to the even ...394 (half up gives ...394 and ...395; a floor ...393 and ...394). Then
    // 1 - t1^2 - t2^2 = 10 376 293 539 154 308 476 / 2^64, just below 3 221 225 472^2, so t3
    // is 3 221 225 471.
    assert_eq!(warp(0, HALF, 1), (1_859_775_394, 3_221_225_471));
    assert_eq!(warp(0, HALF, 3), (1_859_775_394, 3_221_225_471));
    // kappa = 1.0 and t1 = 0: t2 = t2' = 1/2 and t3 = floor(sqrt(3/4)) (to nearest: ...787).
    assert_eq!(warp(ONE, 0, HALF), (HALF, HALF_SQRT3_FLOOR));
    // kappa = 1.0 and the disk's rim: 1 - t1^2 - t2^2 = 0, and below zero it is taken as 0.
    assert_eq!(warp(ONE, 0, ONE), (ONE, 0));
    assert_eq!(floor_root(-1), 0);
    assert_eq!(floor_root(24), 4);
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_156_the_tilt_rounds_both_coordinates_to_even_and_keeps_the_height_non_negative() {
    let half = (HALF, HALF);
    // sigma = kappa = 1/2: along = (t3 - t2) / 2 and up = (t3 + t2) / 2.
    // (t2, t3) = (1, 4): 1.5 -> 2 and 2.5 -> 2 (half up: 2 and 3; floors: 1 and 2).
    assert_eq!(tilt(half, 1, 4), (2, 2));
    // (0, 3): 1.5 -> 2 both. (4, 1): along = -1.5 -> -2, the same rule on the magnitude
    // (rounding toward plus infinity gives -1).
    assert_eq!(tilt(half, 0, 3), (2, 2));
    assert_eq!(tilt(half, 4, 1), (-2, 2));
    // sigma = 1.0, kappa = 0: along = t3 and up = t2, which is negative here and taken as 0.
    assert_eq!(tilt((ONE, 0), -5, 7), (7, 0));
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_157_the_normal_is_unstretched_and_normalized_to_nearest() {
    // Straight down onto the ground: forward = +Z, across = +X, normal = +Y.
    let view = View::new([0, -ONE, 0], &ground()).expect("above the horizon");
    assert_eq!((view.cosine(), view.sine), (ONE, 0));
    // Roughness 1.0, hemisphere point (along, t1, up) = (3/4, 0, 1): the vector (0, 1, 3/4),
    // unit (0, 4/5, 3/5), each to nearest.
    assert_eq!(view.unstretch(ONE, [3 << 30, 0, ONE]), [0, FOUR_FIFTHS, THREE_FIFTHS]);
    // Roughness 1/2 halves the horizontal part: (along, t1, up) = (0, 3/2, 1) gives
    // (3/4, 1, 0), unit (3/5, 4/5, 0).
    assert_eq!(view.unstretch(HALF, [0, 3 << 31, ONE]), [THREE_FIFTHS, FOUR_FIFTHS, 0]);
    // The zero vector cannot be normalized: the frame normal.
    assert_eq!(view.unstretch(ONE, [0, 0, 0]), [0, ONE, 0]);
    // The whole construction at the disk's centre is the normal itself.
    assert_eq!(view.visible_normal(HALF, [0, 0]), [0, ONE, 0]);
}
