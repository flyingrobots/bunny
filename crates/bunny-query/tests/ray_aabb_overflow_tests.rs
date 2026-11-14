//! Reproduction for flyingrobots/bunny#211: `ray_intersects_aabb` reports a miss for rays that
//! pass through the box when a slab distance does not fit in Q32.32.
//!
//! These tests DOCUMENT A KNOWN DEFECT. Each one states the geometrically correct answer in a
//! comment and asserts the current (wrong) output, so the suite stays green while the defect is
//! undecided. When the maintainer decides on a fix, the `None` assertions flip to `Some`.
//! Nothing here changes an output bit of the function.

use bunny_geom::{FixedAabb3, FixedRay3};
use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::FixedQ32_32;
use bunny_query::ray_intersects_aabb;

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

#[test]
fn control_axis_aligned_ray_hits() {
    // y component exactly zero takes the `axis_contains_origin` branch: correct.
    let hit = ray_intersects_aabb(&near_axis_ray(0), &cube(1));
    assert_eq!(hit, Some((units(4), raw(6 * ONE_RAW))));
}

#[test]
fn control_three_raw_units_hits() {
    // 1 / 3 raw still fits: (1 << 64) / 3 < 2^63.
    assert!(ray_intersects_aabb(&near_axis_ray(3), &cube(1)).is_some());
}

#[test]
fn known_defect_one_raw_unit_component_misses_a_box_it_passes_through() {
    // Correct answer: HIT with t in [4, 6]. At t = 6 the y offset is 6 * 2^-32, far inside [-1, 1].
    // Current output: None, because (1 - 0) / 2^-32 = 2^32 overflows Q32.32 (max < 2^31).
    assert_eq!(ray_intersects_aabb(&near_axis_ray(1), &cube(1)), None);
}

#[test]
fn known_defect_two_raw_unit_component_misses_a_box_it_passes_through() {
    // (1 << 64) / 2 = 2^63 does not fit in i64.
    assert_eq!(ray_intersects_aabb(&near_axis_ray(2), &cube(1)), None);
}

#[test]
fn known_defect_negative_tiny_component_misses_too() {
    assert_eq!(ray_intersects_aabb(&near_axis_ray(-1), &cube(1)), None);
}

#[test]
fn known_defect_threshold_is_raw_component_at_most_twice_the_slab_half_width() {
    // Box [-k, k]^3, origin y = 0. Slab distance is k * 2^64 / d raw; it overflows i64 exactly
    // when d <= 2k. Every such ray passes through the box (it enters at x = -k, y ~ 0).
    for k in 1..=64_i64 {
        for d in 1..=(2 * k + 3) {
            let ray = FixedRay3::new(
                FixedVec3::new(units(-100), raw(0), raw(0)),
                FixedUnitVec3::try_from_unit(FixedVec3::new(units(1), raw(d), raw(0)))
                    .expect("within tolerance"),
            );
            let result = ray_intersects_aabb(&ray, &cube(k));
            let overflows = d <= 2 * k;
            assert_eq!(result.is_none(), overflows, "k={k} d={d}");
        }
    }
}

#[test]
fn known_defect_origin_far_from_box_overflows_the_subtraction() {
    // Correct answer: HIT. The ray runs +x from x = -1.5e9 into the box x in [5e8, 7e8].
    // Current output: None, because the slab subtraction does not fit in Q32.32.
    let origin = FixedVec3::new(units(-1_500_000_000), raw(0), raw(0));
    let ray = FixedRay3::new(origin, FixedUnitVec3::UNIT_X);
    let aabb = FixedAabb3::new(
        FixedVec3::new(units(500_000_000), units(-1), units(-1)),
        FixedVec3::new(units(700_000_000), units(1), units(1)),
    );
    // 7e8 - (-1.5e9) = 2.2e9 > 2^31 - 1 = 2_147_483_647: checked_sub overflows.
    assert_eq!(ray_intersects_aabb(&ray, &aabb), None);
}

#[test]
fn defect_never_reports_a_false_hit() {
    // The overflow path returns None only, so it cannot create a hit. A ray that truly misses
    // stays a miss for the same near-axis directions.
    for d in 0..=4 {
        let ray = near_axis_ray(d);
        let far_box = FixedAabb3::new(
            FixedVec3::new(units(-1), units(10), units(-1)),
            FixedVec3::new(units(1), units(12), units(1)),
        );
        assert_eq!(ray_intersects_aabb(&ray, &far_box), None, "d={d}");
    }
}
