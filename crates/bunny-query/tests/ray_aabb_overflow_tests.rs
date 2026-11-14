//! Regression tests for flyingrobots/bunny#211: `ray_intersects_aabb` used to report a miss for
//! rays that pass through the box when a slab distance or a coordinate difference did not fit in
//! Q32.32. Since the fix, an overflowing slab distance saturates by sign, the coordinate
//! difference is taken in `i128`, and the hit is decided exactly on that path.
//!
//! The differential and exact-rational oracle corpora live in `ray_aabb_oracle_tests.rs`.

use bunny_geom::{FixedAabb3, FixedRay3};
use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::FixedQ32_32;
use bunny_query::ray_intersects_aabb;
use wasm_bindgen_test::wasm_bindgen_test;

const ONE_RAW: i64 = bunny_num::fixed_q32_32::ONE_RAW;

fn units(value: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(value * ONE_RAW)
}

fn raw(value: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(value)
}

fn cube(half: i64) -> FixedAabb3 {
    FixedAabb3::new(
        FixedVec3::new(units(-half), units(-half), units(-half)),
        FixedVec3::new(units(half), units(half), units(half)),
    )
}

/// A ray along +x from x = -5 with a raw y component of `y_raw`, validated as a unit vector.
fn near_axis_ray(y_raw: i64) -> FixedRay3 {
    let direction = FixedVec3::new(units(1), raw(y_raw), raw(0));
    let unit = FixedUnitVec3::try_from_unit(direction).expect("(1, tiny, 0) is within tolerance");
    FixedRay3::new(FixedVec3::new(units(-5), raw(0), raw(0)), unit)
}

#[wasm_bindgen_test(unsupported = test)]
fn control_axis_aligned_ray_hits() {
    // y component exactly zero takes the `axis_contains_origin` branch: correct.
    let hit = ray_intersects_aabb(&near_axis_ray(0), &cube(1));
    assert_eq!(hit, Some((units(4), raw(6 * ONE_RAW))));
}

#[wasm_bindgen_test(unsupported = test)]
fn control_three_raw_units_hits() {
    // 1 / 3 raw still fits: (1 << 64) / 3 < 2^63.
    assert!(ray_intersects_aabb(&near_axis_ray(3), &cube(1)).is_some());
}

#[wasm_bindgen_test(unsupported = test)]
fn one_raw_unit_component_hits_the_box_it_passes_through() {
    // The ray enters at t = 4 and leaves at t = 6; at t = 6 the y offset is 6 * 2^-32.
    // The y slab distances (+-1) / 2^-32 = +-2^32 do not fit Q32.32 and saturate, so x clamps them.
    assert_eq!(ray_intersects_aabb(&near_axis_ray(1), &cube(1)), Some((units(4), units(6))));
}

#[wasm_bindgen_test(unsupported = test)]
fn two_raw_unit_component_hits_the_box_it_passes_through() {
    // (1 << 64) / 2 = 2^63 does not fit in i64; the distance saturates.
    assert_eq!(ray_intersects_aabb(&near_axis_ray(2), &cube(1)), Some((units(4), units(6))));
}

#[wasm_bindgen_test(unsupported = test)]
fn negative_tiny_component_hits_too() {
    assert_eq!(ray_intersects_aabb(&near_axis_ray(-1), &cube(1)), Some((units(4), units(6))));
}

#[wasm_bindgen_test(unsupported = test)]
fn hits_across_the_old_overflow_threshold_of_twice_the_slab_half_width() {
    // Box [-k, k]^3, origin (-100, 0, 0). The y slab distance is k * 2^64 / d raw; it overflows
    // i64 exactly when d <= 2k, where the old function reported a miss. Every such ray passes
    // through the box, entering at x = -k: the result must be the x slab on both sides of the
    // old boundary.
    for k in 1..=64_i64 {
        for d in 1..=(2 * k + 3) {
            let ray = FixedRay3::new(
                FixedVec3::new(units(-100), raw(0), raw(0)),
                FixedUnitVec3::try_from_unit(FixedVec3::new(units(1), raw(d), raw(0)))
                    .expect("within tolerance"),
            );
            let result = ray_intersects_aabb(&ray, &cube(k));
            assert_eq!(result, Some((units(100 - k), units(100 + k))), "k={k} d={d}");
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn origin_far_from_box_saturates_the_unrepresentable_exit() {
    // The ray runs +x from x = -1.5e9 into the box x in [5e8, 7e8]. It enters at t = 2e9, which
    // fits Q32.32; it leaves at t = 2.2e9 > 2^31, which does not, so the exit saturates.
    let origin = FixedVec3::new(units(-1_500_000_000), raw(0), raw(0));
    let ray = FixedRay3::new(origin, FixedUnitVec3::UNIT_X);
    let aabb = FixedAabb3::new(
        FixedVec3::new(units(500_000_000), units(-1), units(-1)),
        FixedVec3::new(units(700_000_000), units(1), units(1)),
    );
    assert_eq!(ray_intersects_aabb(&ray, &aabb), Some((units(2_000_000_000), raw(i64::MAX))));
}

#[wasm_bindgen_test(unsupported = test)]
fn coordinate_difference_beyond_i64_is_widened_not_saturated() {
    // Origin x = i64::MIN raw, box max x = 2^30 raw: the difference is 2^63 + 2^30 raw, which does
    // not fit i64. With direction x = 1 + 2^-32 (one raw unit above 1, accepted as unit length)
    // the quotient still fits: round((2^63 + 2^30) * 2^32 / (2^32 + 1)) = 9_223_372_035_781_033_984.
    // Saturating the difference to i64::MAX first would give 9_223_372_034_707_292_160 instead.
    // The entry is round((2^63 - 2^32) * 2^32 / (2^32 + 1)) = 9_223_372_030_412_324_865.
    // Values computed with exact rationals (Python `fractions`), ties to even.
    let direction = FixedUnitVec3::try_from_unit(FixedVec3::new(raw(ONE_RAW + 1), raw(0), raw(0)))
        .expect("one raw unit above 1 is within tolerance");
    let ray = FixedRay3::new(FixedVec3::new(raw(i64::MIN), raw(0), raw(0)), direction);
    let aabb = FixedAabb3::new(
        FixedVec3::new(units(-1), units(-1), units(-1)),
        FixedVec3::new(raw(1 << 30), units(1), units(1)),
    );
    assert_eq!(
        ray_intersects_aabb(&ray, &aabb),
        Some((raw(9_223_372_030_412_324_865), raw(9_223_372_035_781_033_984)))
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn saturated_bounds_never_create_a_false_hit() {
    // A ray that truly misses stays a miss for the same near-axis directions, including the ones
    // whose slab distances saturate.
    for d in 0..=4 {
        let ray = near_axis_ray(d);
        let far_box = FixedAabb3::new(
            FixedVec3::new(units(-1), units(10), units(-1)),
            FixedVec3::new(units(1), units(12), units(1)),
        );
        assert_eq!(ray_intersects_aabb(&ray, &far_box), None, "d={d}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn saturated_entries_that_tie_when_rounded_are_decided_exactly() {
    // Direction (1, 2^-32, 2^-32) from the origin. The y slab [10, 12] starts at t = 10 * 2^32 and
    // the z slab [20, 22] at t = 20 * 2^32: both entries saturate to the maximum. The x slab ends
    // at t = i64::MAX raw (about 2^31), which is also the maximum. Rounded and saturated, entry
    // and exit would tie at (MAX, MAX) and report a hit; exactly, the ray leaves the x slab long
    // before it reaches the y slab, so it misses. The exact decision must win.
    let direction = FixedUnitVec3::try_from_unit(FixedVec3::new(units(1), raw(1), raw(1)))
        .expect("within tolerance");
    let ray = FixedRay3::new(FixedVec3::new(raw(0), raw(0), raw(0)), direction);
    let aabb = FixedAabb3::new(
        FixedVec3::new(raw(i64::MIN), units(10), units(20)),
        FixedVec3::new(raw(i64::MAX), units(12), units(22)),
    );
    assert_eq!(ray_intersects_aabb(&ray, &aabb), None);
}

#[wasm_bindgen_test(unsupported = test)]
fn a_hit_beyond_the_representable_distance_saturates_both_bounds() {
    // From x = i64::MIN raw (about -2^31) along +x, the box x in [1, i64::MAX raw] starts more than
    // 2^31 away and ends about 2^32 away: both distances exceed Q32.32 and saturate. The ray does
    // reach the box, so the result is a hit with both bounds at the maximum.
    let ray = FixedRay3::new(FixedVec3::new(raw(i64::MIN), raw(0), raw(0)), FixedUnitVec3::UNIT_X);
    let aabb = FixedAabb3::new(
        FixedVec3::new(units(1), units(-1), units(-1)),
        FixedVec3::new(raw(i64::MAX), units(1), units(1)),
    );
    assert_eq!(ray_intersects_aabb(&ray, &aabb), Some((raw(i64::MAX), raw(i64::MAX))));
}
