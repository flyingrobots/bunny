//! Dedicated tests for the existing trace-domain bounds, `TraceSphere::bounds` and
//! `TraceTriangle::bounds`, pinned as they behave today (flyingrobots/bunny#151), and their
//! agreement with the geometry-level `FixedAabb3::from_sphere` and `from_triangle`.

use bunny_geom::{FixedAabb3, FixedSphere3};
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use bunny_query::{TraceSphere, TraceTriangle, TRACE_COORD_LIMIT_RAW};
use wasm_bindgen_test::wasm_bindgen_test;

const SEED: u64 = 0xB015_1000_0000_0003;
const CASES: usize = 5_000;
/// The largest magnitude inside the trace domain.
const DOMAIN: i64 = TRACE_COORD_LIMIT_RAW - 1;

type Raw3 = [i64; 3];

fn v(raw: Raw3) -> FixedVec3 {
    FixedVec3::new(
        FixedQ32_32::from_raw(raw[0]),
        FixedQ32_32::from_raw(raw[1]),
        FixedQ32_32::from_raw(raw[2]),
    )
}

fn raw(p: FixedVec3) -> Raw3 {
    [p.x.raw(), p.y.raw(), p.z.raw()]
}

fn corners(b: &FixedAabb3) -> (Raw3, Raw3) {
    (raw(b.min), raw(b.max))
}

fn trace_sphere(center: Raw3, radius: i64) -> TraceSphere {
    TraceSphere::new(v(center), FixedQ32_32::from_raw(radius)).expect("in the trace domain")
}

fn trace_triangle(a: Raw3, b: Raw3, c: Raw3) -> TraceTriangle {
    TraceTriangle::new(v(a), v(b), v(c)).expect("in the trace domain")
}

#[wasm_bindgen_test(unsupported = test)]
fn bounds_tp_014_trace_bounds_hand_derived() {
    let sphere = trace_sphere([1, -2, 3], 4);
    assert_eq!(corners(&sphere.bounds()), ([-3, -6, -1], [5, 2, 7]));
    let smallest = trace_sphere([0; 3], 1);
    assert_eq!(corners(&smallest.bounds()), ([-1; 3], [1; 3]));
    let widest = trace_sphere([DOMAIN, -DOMAIN, 0], DOMAIN);
    assert_eq!(corners(&widest.bounds()), ([0, -2 * DOMAIN, -DOMAIN], [2 * DOMAIN, 0, DOMAIN]));

    let tri = trace_triangle([0, 0, 0], [4, -1, 2], [-2, 3, 1]);
    assert_eq!(corners(&tri.bounds()), ([-2, -1, 0], [4, 3, 2]));
    let point = trace_triangle([5, 5, 5], [5, 5, 5], [5, 5, 5]);
    assert_eq!(corners(&point.bounds()), ([5; 3], [5; 3]), "a degenerate triangle");
    let collinear = trace_triangle([0, 0, 0], [2, 2, 2], [1, 1, 1]);
    assert_eq!(corners(&collinear.bounds()), ([0; 3], [2; 3]));
    let wide = trace_triangle([-DOMAIN; 3], [DOMAIN; 3], [DOMAIN, -DOMAIN, 0]);
    assert_eq!(
        corners(&wide.bounds()),
        ([-DOMAIN; 3], [DOMAIN; 3]),
        "the vertices survive the edges"
    );
}

fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(0x5851_F42D_4C95_7F2D).wrapping_add(0x1405_7B7E_F767_814F);
    *state >> 11 ^ *state << 53 ^ *state
}

/// A raw value in the trace domain, every magnitude.
fn in_domain(state: &mut u64) -> i64 {
    let shift = 24 + next(state) % 40;
    i64::from_ne_bytes(next(state).to_ne_bytes()) >> shift
}

#[wasm_bindgen_test(unsupported = test)]
fn bounds_tp_015_seeded_trace_bounds_match_the_oracle_and_the_geometry_helpers() {
    let mut state = SEED;
    for case in 0..CASES {
        let ctx = format!("seed={SEED:#x} case={case}");
        let centre = [0; 3].map(|_| in_domain(&mut state));
        let radius = in_domain(&mut state).unsigned_abs().max(1);
        let radius = i64::try_from(radius).expect("in the domain");
        let got = corners(&trace_sphere(centre, radius).bounds());
        let want = (centre.map(|c| c - radius), centre.map(|c| c + radius));
        assert_eq!(got, want, "sphere {centre:?} r={radius}; {ctx}");
        let geom =
            FixedAabb3::from_sphere(&FixedSphere3::new(v(centre), FixedQ32_32::from_raw(radius)));
        assert_eq!(geom.map(|b| corners(&b)), Ok(got), "from_sphere agrees; {ctx}");

        let [a, b, c] = [0; 3].map(|_| [0; 3].map(|_| in_domain(&mut state)));
        let got = corners(&trace_triangle(a, b, c).bounds());
        let lo = [0, 1, 2].map(|i| *[a[i], b[i], c[i]].iter().min().expect("three"));
        let hi = [0, 1, 2].map(|i| *[a[i], b[i], c[i]].iter().max().expect("three"));
        assert_eq!(got, (lo, hi), "triangle {a:?} {b:?} {c:?}; {ctx}");
        let geom = corners(&FixedAabb3::from_triangle([v(a), v(b), v(c)]));
        assert_eq!(geom, got, "from_triangle agrees; {ctx}");
    }
    println!("seed={SEED:#x}: {CASES} spheres and {CASES} triangles");
}
