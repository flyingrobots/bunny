//! `ray_intersects_aabb_prepared` must return exactly what `ray_intersects_aabb` returns.

use bunny_geom::{FixedAabb3, FixedRay3};
use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_query::{ray_intersects_aabb, ray_intersects_aabb_prepared, PreparedRay3};
use wasm_bindgen_test::wasm_bindgen_test;

const PREPARED_SEED: u64 = 0xB002_9AE9_0000_0001;

fn next_seed(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state
}

/// A coordinate in [-range, range) Bunny units, from the generator's high bits; a quarter
/// of them are exactly zero so axis-parallel directions occur.
fn coordinate(state: &mut u64, range: i64) -> FixedQ32_32 {
    let bits = next_seed(state) >> 16;
    if bits.is_multiple_of(4) {
        return FixedQ32_32::ZERO;
    }
    let span = u64::try_from(2 * range * ONE_RAW).expect("positive span");
    FixedQ32_32::from_raw(i64::try_from(bits % span).expect("bounded") - range * ONE_RAW)
}

fn vec3(state: &mut u64, range: i64) -> FixedVec3 {
    FixedVec3::new(coordinate(state, range), coordinate(state, range), coordinate(state, range))
}

fn aabb(state: &mut u64) -> FixedAabb3 {
    let (a, b) = (vec3(state, 8), vec3(state, 8));
    let min = FixedVec3::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z));
    let max = FixedVec3::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z));
    FixedAabb3::try_new(min, max).expect("ordered bounds")
}

#[wasm_bindgen_test(unsupported = test)]
fn prepared_ray_matches_unprepared_on_seeded_rays_and_boxes() {
    let mut state = PREPARED_SEED;
    let mut hits = 0;
    for case in 0..20_000 {
        let Ok(ray) = FixedRay3::try_new(vec3(&mut state, 10), vec3(&mut state, 1)) else {
            continue;
        };
        let prepared = PreparedRay3::new(ray);
        assert_eq!(prepared.ray(), &ray);
        for _ in 0..8 {
            let bounds = aabb(&mut state);
            let expected = ray_intersects_aabb(&ray, &bounds);
            hits += i32::from(expected.is_some());
            assert_eq!(ray_intersects_aabb_prepared(&prepared, &bounds), expected, "case={case}");
        }
    }
    assert!(hits > 1_000, "the inputs exercise hits as well as misses: {hits}");
}

#[wasm_bindgen_test(unsupported = test)]
fn prepared_ray_matches_unprepared_for_axis_rays_inside_and_outside_boxes() {
    let one = FixedQ32_32::ONE;
    let unit_box =
        FixedAabb3::try_new(FixedVec3::new(-one, -one, -one), FixedVec3::new(one, one, one))
            .expect("ordered bounds");
    let zero = FixedQ32_32::ZERO;
    let origins = [
        FixedVec3::new(zero, zero, zero),
        FixedVec3::new(one + one, zero, zero),
        FixedVec3::new(zero, one, -one),
    ];
    let directions = [
        FixedVec3::new(one, zero, zero),
        FixedVec3::new(zero, -one, zero),
        FixedVec3::new(zero, zero, one),
    ];
    for origin in origins {
        for direction in directions {
            let ray = FixedRay3::try_new(origin, direction).expect("unit direction");
            let expected = ray_intersects_aabb(&ray, &unit_box);
            assert_eq!(ray_intersects_aabb_prepared(&PreparedRay3::new(ray), &unit_box), expected);
        }
    }
}

fn q(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

/// A coordinate anywhere in `i64`, near the extremes, or ordinary, so slab distances and
/// coordinate differences overflow Q32.32.
fn wide_coordinate(state: &mut u64) -> i64 {
    let bits = i64::from_le_bytes(next_seed(state).to_le_bytes());
    match (next_seed(state) >> 33) % 5 {
        0 => bits,
        1 => i64::MIN + (bits & 0xFF_FFFF_FFFF),
        2 => i64::MAX - (bits & 0xFF_FFFF_FFFF),
        3 => bits >> 24,
        _ => 0,
    }
}

/// A unit direction with a major axis and minor components that are zero or a few raw units
/// (so the slab distance along the major axis does not fit), or `None` if not a unit vector.
fn tiny_direction(state: &mut u64) -> Option<FixedUnitVec3> {
    let major = usize::try_from((next_seed(state) >> 40) % 3).expect("small");
    let mut components = [0_i64; 3];
    for (axis, component) in components.iter_mut().enumerate() {
        let r = next_seed(state) >> 24;
        let sign = if r & 1 == 0 { 1 } else { -1 };
        *component = if axis == major {
            sign * ONE_RAW
        } else if r.is_multiple_of(4) {
            0
        } else {
            sign * i64::try_from(1 + (r >> 2) % 4_000).expect("small")
        };
    }
    let [x, y, z] = components;
    FixedUnitVec3::try_from_unit(FixedVec3::new(q(x), q(y), q(z)))
}

#[wasm_bindgen_test(unsupported = test)]
fn prepared_ray_matches_unprepared_on_the_overflow_corpus() {
    let mut state = PREPARED_SEED ^ 0x0F10;
    let (mut cases, mut hits, mut saturated, mut misses) = (0, 0, 0, 0);
    for _ in 0..60_000 {
        let Some(unit) = tiny_direction(&mut state) else {
            continue;
        };
        let origin = FixedVec3::new(
            q(wide_coordinate(&mut state)),
            q(wide_coordinate(&mut state)),
            q(wide_coordinate(&mut state)),
        );
        let ray = FixedRay3::new(origin, unit);
        let prepared = PreparedRay3::new(ray);
        let dir = unit.into_inner();
        for _ in 0..4 {
            // Half the boxes are centred on a point of the ray's line, so the ray reaches them;
            // the others are anywhere.
            let t = q(wide_coordinate(&mut state));
            let on_line =
                FixedVec3::new(origin.x + dir.x * t, origin.y + dir.y * t, origin.z + dir.z * t);
            let anywhere = FixedVec3::new(
                q(wide_coordinate(&mut state)),
                q(wide_coordinate(&mut state)),
                q(wide_coordinate(&mut state)),
            );
            let centre = if next_seed(&mut state) & (1 << 40) == 0 { on_line } else { anywhere };
            let half = |state: &mut u64| {
                i64::try_from(next_seed(state) >> (1 + (next_seed(state) >> 33) % 62))
                    .expect("63 bits")
            };
            let (hx, hy, hz) = (half(&mut state), half(&mut state), half(&mut state));
            let min = FixedVec3::new(
                q(centre.x.to_raw().saturating_sub(hx)),
                q(centre.y.to_raw().saturating_sub(hy)),
                q(centre.z.to_raw().saturating_sub(hz)),
            );
            let max = FixedVec3::new(
                q(centre.x.to_raw().saturating_add(hx)),
                q(centre.y.to_raw().saturating_add(hy)),
                q(centre.z.to_raw().saturating_add(hz)),
            );
            let bounds = FixedAabb3::new(min, max);
            let expected = ray_intersects_aabb(&ray, &bounds);
            cases += 1;
            match expected {
                Some((enter, exit)) => {
                    hits += 1;
                    saturated +=
                        usize::from(enter.to_raw() == i64::MIN || exit.to_raw() == i64::MAX);
                }
                None => misses += 1,
            }
            assert_eq!(
                ray_intersects_aabb_prepared(&prepared, &bounds),
                expected,
                "{ray:?} {bounds:?}"
            );
        }
    }
    assert!(cases > 100_000, "cases={cases}");
    assert!(hits > 5_000 && misses > 5_000, "hits={hits} misses={misses}");
    assert!(saturated > 1_000, "saturated bounds are exercised: {saturated}");
}

#[wasm_bindgen_test(unsupported = test)]
fn prepared_ray_matches_unprepared_on_constructed_overflow_cases() {
    let tiny = FixedUnitVec3::try_from_unit(FixedVec3::new(q(ONE_RAW), q(1), q(0))).expect("unit");
    let far = FixedAabb3::new(
        FixedVec3::new(q(i64::MAX - 10), q(-(1 << 40)), q(-(1 << 40))),
        FixedVec3::new(q(i64::MAX), q(1 << 40), q(1 << 40)),
    );
    let ray = FixedRay3::new(FixedVec3::new(q(i64::MIN), q(0), q(0)), tiny);
    let expected = ray_intersects_aabb(&ray, &far);
    assert!(expected.is_some(), "the ray reaches the far box through the wide path");
    assert_eq!(ray_intersects_aabb_prepared(&PreparedRay3::new(ray), &far), expected);
}
