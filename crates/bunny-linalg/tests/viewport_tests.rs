//! Viewport mapping contract tests: golden vectors on power-of-two sizes, the
//! exact raw rounding rule at the edges, typed errors, and a seeded property
//! test of the round-trip bound against exact integer arithmetic.

use bunny_linalg::{FixedVec3, Viewport, ViewportError};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const ONE_RAW: i64 = bunny_num::fixed_q32_32::ONE_RAW;
const HALF_RAW: i64 = ONE_RAW / 2;
const SEED: u64 = 0xB002_0202_0000_0001;
const CASES: usize = 256;

fn v(x: i64, y: i64, z: i64) -> FixedVec3 {
    let raw = FixedQ32_32::from_raw;
    FixedVec3::new(raw(x), raw(y), raw(z))
}

fn raws(p: FixedVec3) -> [i64; 3] {
    [p.x.raw(), p.y.raw(), p.z.raw()]
}

fn viewport(width: u32, height: u32) -> Viewport {
    Viewport::try_new(width, height).expect("non-empty viewport")
}

fn to_pixels(vp: Viewport, x: i64, y: i64, z: i64) -> [i64; 3] {
    raws(vp.try_ndc_to_viewport(v(x, y, z)).expect("in range"))
}

fn to_ndc(vp: Viewport, x: i64, y: i64, z: i64) -> [i64; 3] {
    raws(vp.try_viewport_to_ndc(v(x, y, z)).expect("in range"))
}

#[wasm_bindgen_test(unsupported = test)]
fn cl_tp_006_four_by_two_viewport_has_exact_golden_vectors() {
    let vp = viewport(4, 2);
    // Origin is the top-left corner and pixel y runs down: NDC (-1, +1) is (0, 0).
    assert_eq!(to_pixels(vp, -ONE_RAW, ONE_RAW, 0), [0, 0, 0]);
    assert_eq!(to_pixels(vp, ONE_RAW, ONE_RAW, 0), [4 * ONE_RAW, 0, 0]);
    assert_eq!(to_pixels(vp, -ONE_RAW, -ONE_RAW, 0), [0, 2 * ONE_RAW, 0]);
    assert_eq!(to_pixels(vp, ONE_RAW, -ONE_RAW, ONE_RAW), [4 * ONE_RAW, 2 * ONE_RAW, ONE_RAW]);
    // The NDC origin is the exact centre of the pixel grid.
    assert_eq!(to_pixels(vp, 0, 0, HALF_RAW), [2 * ONE_RAW, ONE_RAW, HALF_RAW]);
    // Quarter points: NDC x = -1/2 is pixel x = 1; NDC y = 1/2 is pixel y = 1/2.
    assert_eq!(to_pixels(vp, -HALF_RAW, HALF_RAW, 0), [ONE_RAW, HALF_RAW, 0]);
}

#[wasm_bindgen_test(unsupported = test)]
fn cl_tp_007_one_by_one_and_hd_viewports_have_exact_golden_vectors() {
    let unit = viewport(1, 1);
    assert_eq!(to_pixels(unit, -ONE_RAW, ONE_RAW, 0), [0, 0, 0]);
    assert_eq!(to_pixels(unit, ONE_RAW, -ONE_RAW, ONE_RAW), [ONE_RAW, ONE_RAW, ONE_RAW]);
    assert_eq!(to_pixels(unit, 0, 0, 0), [HALF_RAW, HALF_RAW, 0]);
    let hd = viewport(1920, 1080);
    assert_eq!(to_pixels(hd, 0, 0, 0), [960 * ONE_RAW, 540 * ONE_RAW, 0]);
    assert_eq!(to_pixels(hd, -ONE_RAW, ONE_RAW, 0), [0, 0, 0]);
    assert_eq!(
        to_pixels(hd, ONE_RAW, -ONE_RAW, ONE_RAW),
        [1920 * ONE_RAW, 1080 * ONE_RAW, ONE_RAW]
    );
    // Odd sizes: a 3 x 5 viewport puts the NDC origin at (3/2, 5/2).
    let odd = viewport(3, 5);
    assert_eq!(to_pixels(odd, 0, 0, 0), [3 * HALF_RAW, 5 * HALF_RAW, 0]);
}

#[wasm_bindgen_test(unsupported = test)]
fn cl_tp_008_rounding_is_floor_of_exact_raw_value_at_the_edges() {
    // In a 1 x 1 viewport the exact raw result is (ndc_raw +- ONE_RAW) / 2, so
    // an odd numerator is a half raw unit and floor decides the result.
    let unit = viewport(1, 1);
    // x = -1 + 1 raw: exact 1/2 raw unit rounds down to 0.
    assert_eq!(to_pixels(unit, -ONE_RAW + 1, 0, 0)[0], 0);
    // x = -1 + 3 raw: exact 3/2 raw units rounds down to 1.
    assert_eq!(to_pixels(unit, -ONE_RAW + 3, 0, 0)[0], 1);
    // x = -1 - 1 raw is outside the cube: exact -1/2 raw unit floors to -1,
    // not to zero, so the rule is floor and not truncation.
    assert_eq!(to_pixels(unit, -ONE_RAW - 1, 0, 0)[0], -1);
    // y = +1 - 1 raw: exact 1/2 raw unit rounds down to 0; y = +1 + 1 raw gives -1/2 -> -1.
    assert_eq!(to_pixels(unit, 0, ONE_RAW - 1, 0)[1], 0);
    assert_eq!(to_pixels(unit, 0, ONE_RAW + 1, 0)[1], -1);
    // y = -1 + 1 raw: exact 2^32 - 1/2 raw units rounds down to 2^32 - 1.
    assert_eq!(to_pixels(unit, 0, -ONE_RAW + 1, 0)[1], ONE_RAW - 1);
}

#[wasm_bindgen_test(unsupported = test)]
fn cl_tp_009_depth_passes_through_and_points_outside_the_cube_map_linearly() {
    let vp = viewport(4, 2);
    for z in [0, 1, HALF_RAW, ONE_RAW - 1, ONE_RAW, -ONE_RAW, 3 * ONE_RAW] {
        assert_eq!(to_pixels(vp, 0, 0, z)[2], z);
        assert_eq!(to_ndc(vp, 0, 0, z)[2], z);
    }
    // NDC x = 2 is one full viewport width past the right edge; y = -3 is two
    // heights below the bottom edge; no clipping happens.
    assert_eq!(to_pixels(vp, 2 * ONE_RAW, -3 * ONE_RAW, 0), [6 * ONE_RAW, 4 * ONE_RAW, 0]);
    assert_eq!(to_pixels(vp, -3 * ONE_RAW, 3 * ONE_RAW, 0), [-4 * ONE_RAW, -2 * ONE_RAW, 0]);
}

#[wasm_bindgen_test(unsupported = test)]
fn cl_tp_010_empty_extent_and_overflow_are_typed_errors() {
    assert_eq!(Viewport::try_new(0, 1), Err(ViewportError::EmptyExtent));
    assert_eq!(Viewport::try_new(1, 0), Err(ViewportError::EmptyExtent));
    assert_eq!(Viewport::try_new(0, 0), Err(ViewportError::EmptyExtent));
    let vp = viewport(4, 2);
    assert_eq!(vp.try_ndc_to_viewport(v(i64::MAX, 0, 0)), Err(ViewportError::Overflow));
    assert_eq!(vp.try_ndc_to_viewport(v(i64::MIN, 0, 0)), Err(ViewportError::Overflow));
    assert_eq!(vp.try_ndc_to_viewport(v(0, i64::MIN, 0)), Err(ViewportError::Overflow));
    // (1 - y) * height / 2 is negative here, so it needs height 4 to leave the range.
    let tall = viewport(4, 4);
    assert_eq!(tall.try_ndc_to_viewport(v(0, i64::MAX, 0)), Err(ViewportError::Overflow));
    let one = viewport(1, 1);
    assert_eq!(one.try_viewport_to_ndc(v(i64::MAX, 0, 0)), Err(ViewportError::Overflow));
    assert_eq!(one.try_viewport_to_ndc(v(0, i64::MIN, 0)), Err(ViewportError::Overflow));
    assert_eq!(
        ViewportError::EmptyExtent.to_string(),
        "viewport width and height must be positive"
    );
    assert_eq!(
        ViewportError::Overflow.to_string(),
        "viewport arithmetic is outside the Q32.32 range"
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn cl_tp_011_inverse_mapping_has_exact_golden_vectors() {
    let vp = viewport(4, 2);
    assert_eq!(to_ndc(vp, 0, 0, 0), [-ONE_RAW, ONE_RAW, 0]);
    assert_eq!(to_ndc(vp, 4 * ONE_RAW, 2 * ONE_RAW, ONE_RAW), [ONE_RAW, -ONE_RAW, ONE_RAW]);
    assert_eq!(to_ndc(vp, 2 * ONE_RAW, ONE_RAW, 0), [0, 0, 0]);
    // The inverse floors too: in a 1 x 1 viewport pixel raw 1 is NDC x = -1 + 2 raw
    // exactly, and pixel raw -1 floors the exact -2 raw to -2 raw below -1.
    let unit = viewport(1, 1);
    assert_eq!(to_ndc(unit, 1, 0, 0)[0], -ONE_RAW + 2);
    assert_eq!(to_ndc(unit, -1, 0, 0)[0], -ONE_RAW - 2);
    // A width of 3: pixel x = 1 is NDC -1 + floor(2 * 2^32 / 3) raw = -1/3 - 1 raw
    // (2^33 / 3 = 2863311530.67, floored to 2863311530).
    let three = viewport(3, 3);
    assert_eq!(to_ndc(three, ONE_RAW, 0, 0)[0], -ONE_RAW + 2_863_311_530);
}

fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state >> 33
}

fn bounded(state: &mut u64, lo: u64, hi: u64) -> i64 {
    i64::try_from(lo + next(state) % (hi - lo + 1)).expect("bounded")
}

#[wasm_bindgen_test(unsupported = test)]
fn cl_tp_012_seeded_round_trip_matches_exact_integer_arithmetic() {
    let mut state = SEED;
    for case in 0..CASES {
        let (w, h) = (bounded(&mut state, 1, 5000), bounded(&mut state, 1, 5000));
        let vp = viewport(u32::try_from(w).expect("w"), u32::try_from(h).expect("h"));
        let x = bounded(&mut state, 0, 4 * ONE_RAW as u64) - 2 * ONE_RAW;
        let y = bounded(&mut state, 0, 4 * ONE_RAW as u64) - 2 * ONE_RAW;
        let px = vp.try_ndc_to_viewport(v(x, y, 0)).expect("forward");
        // Exact oracle: 2 * pixel <= (ndc + 1) * width < 2 * pixel + 2.
        let (ax, ay) = (i128::from(x + ONE_RAW), i128::from(ONE_RAW - y));
        for (a, size, p) in [(ax, w, px.x.raw()), (ay, h, px.y.raw())] {
            let exact_twice = a * i128::from(size);
            assert!(2 * i128::from(p) <= exact_twice, "floor; case={case}");
            assert!(exact_twice < 2 * i128::from(p) + 2, "within one raw unit; case={case}");
        }
        // Round trip: exact unless width (or height) and the numerator are both
        // odd, in which case the result is exactly one raw unit lower in the
        // numerator (x: ndc one raw unit lower; y: one raw unit higher).
        let back = vp.try_viewport_to_ndc(px).expect("inverse");
        let lost = |a: i128, size: i64| i64::from(a % 2 != 0 && size % 2 != 0);
        assert_eq!(back.x.raw(), x - lost(ax, w), "x round trip; case={case}");
        assert_eq!(back.y.raw(), y + lost(ay, h), "y round trip; case={case}");
    }
}
