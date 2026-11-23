//! Typed refusals of the trace constructors: `TraceRay::try_new` and `TraceTriangle::try_new`
//! (flyingrobots/bunny#445).
//!
//! The oracle is the refusal rule written out here from the raw coordinates, in the order the
//! constructors' documentation gives; it shares no helper with the kernels. Every boundary is
//! tried on, and one raw unit either side of, its threshold. The seeded cases check that each
//! `Option` form returns `Some(x)` exactly when its `try_` form returns `Ok(x)`, with the error
//! the oracle names otherwise.

use core::error::Error;

use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use bunny_query::{
    TraceRay, TraceRayError, TraceTriangle, TraceTriangleError, TRACE_COORD_LIMIT_RAW,
    TRACE_DIRECTION_LIMIT_RAW,
};
use wasm_bindgen_test::wasm_bindgen_test;

/// The seed of the generated cases; printed in every failure message.
const REFUSAL_SEED: u64 = 0xB002_0445_0000_0001;
const RAY_CASES: usize = 200_000;
const TRIANGLE_CASES: usize = 200_000;
/// The coordinate limit of the trace domain (exclusive) and the direction limit (inclusive),
/// written as literals so a changed constant is caught: `2^40` and `2^32` raw.
const COORD_LIMIT: i64 = 1 << 40;
const DIRECTION_LIMIT: i64 = 1 << 32;

fn vec3([x, y, z]: [i64; 3]) -> FixedVec3 {
    FixedVec3::new(FixedQ32_32::from_raw(x), FixedQ32_32::from_raw(y), FixedQ32_32::from_raw(z))
}

/// `[0, 0, 0]` with `value` on `axis`.
fn on_axis(axis: usize, value: i64) -> [i64; 3] {
    let mut raw = [0; 3];
    raw[axis] = value;
    raw
}

/// What the oracle expects of `TraceRay::try_new`.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum RayRule {
    Accepted,
    OriginOutside,
    DirectionTooLarge,
    ZeroDirection,
}

/// The ray refusal rule from the raw values, first failing condition first.
fn ray_rule(origin: [i64; 3], direction: [i64; 3]) -> RayRule {
    if origin.iter().any(|&c| i128::from(c).abs() >= i128::from(COORD_LIMIT)) {
        RayRule::OriginOutside
    } else if direction.iter().any(|&c| i128::from(c).abs() > i128::from(DIRECTION_LIMIT)) {
        RayRule::DirectionTooLarge
    } else if direction == [0, 0, 0] {
        RayRule::ZeroDirection
    } else {
        RayRule::Accepted
    }
}

/// The index of the first vertex with a coordinate outside the trace domain, if any.
fn triangle_rule(vertices: [[i64; 3]; 3]) -> Option<usize> {
    vertices.iter().position(|v| v.iter().any(|&c| i128::from(c).abs() >= i128::from(COORD_LIMIT)))
}

/// Checks one ray against the oracle and the `Option` form; returns the rule it fell under.
fn check_ray(origin: [i64; 3], direction: [i64; 3], context: &str) -> RayRule {
    let (o, d) = (vec3(origin), vec3(direction));
    let result = TraceRay::try_new(o, d);
    let rule = ray_rule(origin, direction);
    let expected = match rule {
        RayRule::Accepted => None,
        RayRule::OriginOutside => Some(TraceRayError::OriginOutsideTraceDomain { origin: o }),
        RayRule::DirectionTooLarge => {
            Some(TraceRayError::DirectionComponentTooLarge { direction: d })
        }
        RayRule::ZeroDirection => Some(TraceRayError::ZeroDirection),
    };
    assert_eq!(result.err(), expected, "{context}: origin {origin:?}, direction {direction:?}");
    if let Ok(ray) = result {
        assert_eq!((ray.origin(), ray.direction()), (o, d), "{context}: accepted ray round-trips");
    }
    assert_eq!(TraceRay::new(o, d), result.ok(), "{context}: Option form differs");
    rule
}

/// Checks one triangle against the oracle and the `Option` form.
fn check_triangle(vertices: [[i64; 3]; 3], context: &str) -> Option<usize> {
    let [a, b, c] = vertices.map(vec3);
    let result = TraceTriangle::try_new(a, b, c);
    let rule = triangle_rule(vertices);
    let expected = rule.map(|index| TraceTriangleError::VertexOutsideTraceDomain {
        index,
        vertex: [a, b, c][index],
    });
    assert_eq!(result.err(), expected, "{context}: vertices {vertices:?}");
    if let Ok(triangle) = result {
        assert_eq!(triangle.vertices(), [a, b, c], "{context}: accepted triangle round-trips");
    }
    assert_eq!(TraceTriangle::new(a, b, c), result.ok(), "{context}: Option form differs");
    rule
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_720_an_origin_outside_the_domain_is_named() {
    assert_eq!(TRACE_COORD_LIMIT_RAW, COORD_LIMIT);
    let down = [0, -DIRECTION_LIMIT, 0];
    for axis in 0..3 {
        for sign in [1, -1] {
            let inside = on_axis(axis, sign * (COORD_LIMIT - 1));
            let edge = on_axis(axis, sign * COORD_LIMIT);
            let past = on_axis(axis, sign * (COORD_LIMIT + 1));
            assert_eq!(check_ray(inside, down, "one raw unit inside"), RayRule::Accepted);
            assert_eq!(check_ray(edge, down, "on the limit"), RayRule::OriginOutside);
            assert_eq!(check_ray(past, down, "one raw unit past"), RayRule::OriginOutside);
        }
    }
    let extreme = [i64::MIN, i64::MAX, 0];
    assert_eq!(check_ray(extreme, down, "i64 extremes"), RayRule::OriginOutside);
    let named = TraceRay::try_new(vec3(on_axis(1, COORD_LIMIT)), vec3(down));
    let origin = vec3(on_axis(1, COORD_LIMIT));
    assert_eq!(named, Err(TraceRayError::OriginOutsideTraceDomain { origin }));
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_721_a_direction_component_above_one_is_named() {
    assert_eq!(TRACE_DIRECTION_LIMIT_RAW, DIRECTION_LIMIT);
    let origin = [0, 0, 0];
    for axis in 0..3 {
        for sign in [1, -1] {
            let below = on_axis(axis, sign * (DIRECTION_LIMIT - 1));
            let on = on_axis(axis, sign * DIRECTION_LIMIT);
            let past = on_axis(axis, sign * (DIRECTION_LIMIT + 1));
            assert_eq!(check_ray(origin, below, "one raw unit below"), RayRule::Accepted);
            assert_eq!(check_ray(origin, on, "on the limit"), RayRule::Accepted);
            assert_eq!(check_ray(origin, past, "one raw unit past"), RayRule::DirectionTooLarge);
        }
    }
    let largest = [DIRECTION_LIMIT, -DIRECTION_LIMIT, DIRECTION_LIMIT];
    assert_eq!(check_ray(origin, largest, "every component at the limit"), RayRule::Accepted);
    let extreme = [i64::MIN, 0, 0];
    assert_eq!(check_ray(origin, extreme, "i64::MIN"), RayRule::DirectionTooLarge);
    let direction = vec3([0, 0, DIRECTION_LIMIT + 1]);
    let named = TraceRay::try_new(vec3(origin), direction);
    assert_eq!(named, Err(TraceRayError::DirectionComponentTooLarge { direction }));
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_722_a_zero_direction_is_named() {
    let origin = [3, -5, 7];
    assert_eq!(check_ray(origin, [0, 0, 0], "zero"), RayRule::ZeroDirection);
    for axis in 0..3 {
        for raw in [1, -1] {
            let smallest = on_axis(axis, raw);
            assert_eq!(check_ray(origin, smallest, "one raw unit"), RayRule::Accepted);
        }
    }
    let named = TraceRay::try_new(vec3(origin), vec3([0, 0, 0]));
    assert_eq!(named, Err(TraceRayError::ZeroDirection));
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_723_the_first_failing_ray_condition_is_reported() {
    let outside = on_axis(0, COORD_LIMIT);
    let too_large = on_axis(2, DIRECTION_LIMIT + 1);
    // Origin before direction, whatever the direction.
    assert_eq!(check_ray(outside, too_large, "both bad"), RayRule::OriginOutside);
    assert_eq!(check_ray(outside, [0, 0, 0], "outside and zero"), RayRule::OriginOutside);
    // A component above the limit is never zero, so the zero check cannot come first.
    let mixed = [DIRECTION_LIMIT + 1, 0, 0];
    assert_eq!(check_ray([0, 0, 0], mixed, "large"), RayRule::DirectionTooLarge);
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_724_a_triangle_vertex_outside_the_domain_is_named_by_index() {
    let base = [[0, 0, 0], [COORD_LIMIT - 1, 0, 0], [0, COORD_LIMIT - 1, 0]];
    assert_eq!(check_triangle(base, "inside"), None);
    for index in 0..3 {
        for axis in 0..3 {
            for sign in [1, -1] {
                let mut vertices = base;
                vertices[index][axis] = sign * (COORD_LIMIT - 1);
                assert_eq!(check_triangle(vertices, "one raw unit inside"), None);
                vertices[index][axis] = sign * COORD_LIMIT;
                assert_eq!(check_triangle(vertices, "on the limit"), Some(index));
                vertices[index][axis] = sign * (COORD_LIMIT + 1);
                assert_eq!(check_triangle(vertices, "one raw unit past"), Some(index));
            }
        }
    }
    // The first vertex outside is the one named.
    let all_out = [[COORD_LIMIT; 3], [-COORD_LIMIT; 3], [i64::MIN; 3]];
    assert_eq!(check_triangle(all_out, "all outside"), Some(0));
    let last_two = [[0; 3], [i64::MAX, 0, 0], [COORD_LIMIT, 0, 0]];
    assert_eq!(check_triangle(last_two, "second and third outside"), Some(1));
    let vertex = vec3([0, 0, -COORD_LIMIT]);
    let named = TraceTriangle::try_new(vec3([0; 3]), vec3([1, 0, 0]), vertex);
    assert_eq!(named, Err(TraceTriangleError::VertexOutsideTraceDomain { index: 2, vertex }));
}

struct Seeds(u64);

impl Seeds {
    fn next(&mut self) -> u64 {
        self.0 =
            self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    fn below(&mut self, bound: u64) -> u64 {
        ((self.next() << 21) ^ self.next()) % bound
    }

    /// A raw value in `-bound..=bound`.
    fn signed(&mut self, bound: i64) -> i64 {
        let span = 2 * bound.unsigned_abs() + 1;
        let offset = i64::try_from(self.below(span)).expect("below 2^42");
        offset - bound
    }

    /// A coordinate: mostly inside the domain, often on or one raw unit either side of its
    /// limit, sometimes at the ends of `i64`.
    fn coordinate(&mut self, limit: i64) -> i64 {
        let sign = if self.next().is_multiple_of(2) { 1 } else { -1 };
        match self.next() % 8 {
            0 => sign * (limit - 1),
            1 => sign * limit,
            2 => sign * (limit + 1),
            3 => [i64::MIN, i64::MAX, 0][usize::try_from(self.next() % 3).expect("small")],
            _ => self.signed(limit),
        }
    }

    /// A coordinate that is usually inside: one in `rare` draws comes from [`Self::coordinate`].
    fn mostly_inside(&mut self, limit: i64, rare: u64) -> i64 {
        if self.next().is_multiple_of(rare) {
            self.coordinate(limit)
        } else {
            self.signed(limit - 1)
        }
    }
}

fn point(seeds: &mut Seeds, limit: i64, rare: u64) -> [i64; 3] {
    [0; 3].map(|_| seeds.mostly_inside(limit, rare))
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_725_ray_option_and_result_forms_agree_on_generated_inputs() {
    let mut seeds = Seeds(REFUSAL_SEED);
    let mut counts = [0_usize; 4];
    for case in 0..RAY_CASES {
        let origin = point(&mut seeds, COORD_LIMIT, 6);
        let direction = match seeds.next() % 16 {
            0 => [0, 0, 0],
            1 => on_axis(usize::try_from(seeds.next() % 3).expect("axis"), 1),
            _ => point(&mut seeds, DIRECTION_LIMIT + 1, 6),
        };
        let context = format!("seed {REFUSAL_SEED:#x}, case {case}");
        let rule = check_ray(origin, direction, &context);
        counts[rule as usize] += 1;
    }
    println!(
        "seed {REFUSAL_SEED:#x}: {RAY_CASES} rays, [accepted, origin, direction, zero] {counts:?}"
    );
    assert!(counts.iter().all(|&n| n >= 1_000), "every outcome is reached often: {counts:?}");
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_726_triangle_option_and_result_forms_agree_on_generated_inputs() {
    let mut seeds = Seeds(REFUSAL_SEED ^ 0x7A1);
    let mut counts = [0_usize; 4];
    for case in 0..TRIANGLE_CASES {
        let vertices = [0; 3].map(|_| point(&mut seeds, COORD_LIMIT, 12));
        let context = format!("seed {:#x}, case {case}", REFUSAL_SEED ^ 0x7A1);
        let slot = check_triangle(vertices, &context).map_or(0, |index| index + 1);
        counts[slot] += 1;
    }
    println!(
        "seed {:#x}: {TRIANGLE_CASES} triangles, [accepted, v0, v1, v2] {counts:?}",
        REFUSAL_SEED ^ 0x7A1
    );
    assert!(counts.iter().all(|&n| n >= 1_000), "every outcome is reached often: {counts:?}");
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_727_trace_refusals_display_their_raw_values_and_are_errors() {
    let origin = vec3([COORD_LIMIT, -2, 3]);
    let ray = TraceRayError::OriginOutsideTraceDomain { origin };
    assert_eq!(
        ray.to_string(),
        "ray origin raw (1099511627776, -2, 3) is outside the trace domain"
    );
    let direction = vec3([0, 4_294_967_297, 0]);
    let large = TraceRayError::DirectionComponentTooLarge { direction };
    assert_eq!(
        large.to_string(),
        "ray direction raw (0, 4294967297, 0) has a component above 1.0 in magnitude"
    );
    assert_eq!(TraceRayError::ZeroDirection.to_string(), "ray direction is zero");
    let vertex = vec3([0, 0, -COORD_LIMIT]);
    let triangle = TraceTriangleError::VertexOutsideTraceDomain { index: 2, vertex };
    assert_eq!(
        triangle.to_string(),
        "triangle vertex 2 at raw (0, 0, -1099511627776) is outside the trace domain"
    );
    let errors: [&dyn Error; 2] = [&ray, &triangle];
    assert!(errors.iter().all(|e| e.source().is_none()), "no underlying cause");
}
