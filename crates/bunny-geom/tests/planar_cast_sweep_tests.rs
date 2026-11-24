//! Seeded sweeps of the 2D ray queries and point in polygon against the independent
//! big-integer oracle (`bunny-planar/1`, `docs/topics/planar-geometry/`).
//!
//! Random inputs almost never touch, so the generators build contacts deliberately: a point
//! `o + k d` on the ray, and around it box corners, circles about a point `n` direction
//! lengths off the line with radius about `n 2^32`, and segments across, along or ending on
//! the ray's line, each then moved by one raw unit; and polygons on a small lattice with
//! points on and next to their vertices and edges. Directions include the axes, shallow ones
//! one to eight raw units off an axis, and normalized random vectors. The outcome counts of
//! each sweep are pinned, so a generator that stops producing contacts fails the test.

// The queries name their points `a` to `e`, `o` and `p` as the chapter does.
#![allow(clippy::many_single_char_names)]

mod planar_cast_oracle;
mod planar_oracle;
mod predicates_oracle;

use bunny_geom::{FixedAabb2, FixedCircle2, FixedPolygon2, FixedRay2, FixedSegment2};
use bunny_geom::{PolygonLocation, RayHit2};
use bunny_linalg::{FixedUnitVec2, FixedVec2};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const SEED_RAYS: u64 = 0x0149_0002_0000_0012;
const SEED_POLYGONS: u64 = 0x0149_0002_0000_0013;
const EXTREMES: [i64; 8] = [i64::MIN, i64::MIN + 1, -2, -1, 0, 1, i64::MAX - 1, i64::MAX];
const U: i64 = 1 << 32;

/// The `SplitMix64` generator.
fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A raw value of any magnitude: a random `i64` shifted right (arithmetically) by 0 to 63.
fn spread(state: &mut u64) -> i64 {
    let value = next(state).cast_signed();
    value >> (next(state) % 64)
}

/// A value in `-bound..=bound`.
fn small(state: &mut u64, bound: u64) -> i64 {
    i64::try_from(next(state) % (2 * bound + 1)).expect("small") - bound.cast_signed()
}

fn extreme(state: &mut u64) -> i64 {
    EXTREMES[usize::try_from(next(state) % 8).expect("index")]
}

/// A coordinate that is small, of any magnitude, or from the ends of the range.
fn coordinate(state: &mut u64) -> i64 {
    match next(state) % 3 {
        0 => small(state, 16),
        1 => spread(state),
        _ => extreme(state),
    }
}

fn jitter(state: &mut u64) -> i128 {
    i128::from(small(state, 1))
}

/// The value clamped to the raw range, to keep constructed points representable.
fn sat(value: i128) -> i64 {
    i64::try_from(value).unwrap_or(if value < 0 { i64::MIN } else { i64::MAX })
}

fn raw(v: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(v)
}

fn v(x: i64, y: i64) -> FixedVec2 {
    FixedVec2::new(raw(x), raw(y))
}

fn wide(p: FixedVec2) -> [i128; 2] {
    [i128::from(p.x.raw()), i128::from(p.y.raw())]
}

/// `p + k step + (jx, jy)`, clamped to the raw range.
fn offset(p: FixedVec2, k: i128, step: [i128; 2], j: [i128; 2]) -> FixedVec2 {
    let [x, y] = wide(p);
    v(sat(x + k * step[0] + j[0]), sat(y + k * step[1] + j[1]))
}

/// An axis direction, a shallow one (`2^32` or `2^32 - 1` along one axis and up to eight raw
/// units across, each sign and swap), or a normalized random vector.
fn direction(state: &mut u64) -> FixedUnitVec2 {
    loop {
        let candidate = match next(state) % 3 {
            0 => [FixedUnitVec2::UNIT_X, FixedUnitVec2::NEG_UNIT_X]
                .into_iter()
                .chain([FixedUnitVec2::UNIT_Y, FixedUnitVec2::NEG_UNIT_Y])
                .nth(usize::try_from(next(state) % 4).expect("index")),
            1 => {
                let along = U - i64::from(next(state).is_multiple_of(2));
                let along = if next(state).is_multiple_of(2) { along } else { -along };
                let across = small(state, 8);
                let swap = next(state).is_multiple_of(2);
                let (x, y) = if swap { (across, along) } else { (along, across) };
                FixedUnitVec2::try_from_unit(v(x, y))
            }
            _ => FixedUnitVec2::new(v(spread(state), spread(state))),
        };
        if let Some(d) = candidate {
            return d;
        }
    }
}

/// A ray and a point `o + k d` on it (clamped), `k` a small or large count of directions, up
/// to about `2^32` so that some contacts are beyond the parameter range,
/// sometimes negative so the point is behind the origin.
fn ray_and_target(state: &mut u64) -> (FixedRay2, FixedVec2, [i128; 2]) {
    let d = direction(state);
    let o = v(coordinate(state), coordinate(state));
    let k = if next(state).is_multiple_of(2) {
        i128::from(small(state, 12))
    } else {
        i128::from(spread(state) >> 31)
    };
    let step = wide(d.into_inner());
    (FixedRay2::new(o, d), offset(o, k, step, [0, 0]), step)
}

fn some_box(state: &mut u64, p: FixedVec2) -> FixedAabb2 {
    let corner = offset(p, 0, [0, 0], [jitter(state), jitter(state)]);
    let far = if next(state).is_multiple_of(3) {
        v(coordinate(state), coordinate(state))
    } else {
        offset(corner, 1, [i128::from(small(state, 9)), i128::from(small(state, 9))], [0, 0])
    };
    let (cx, cy, fx, fy) = (corner.x.raw(), corner.y.raw(), far.x.raw(), far.y.raw());
    if next(state).is_multiple_of(4) {
        FixedAabb2::new(corner, far)
    } else {
        FixedAabb2::new(v(cx.min(fx), cy.min(fy)), v(cx.max(fx), cy.max(fy)))
    }
}

/// A circle about a point `n` direction lengths off the ray's line at `p`, with radius
/// `n 2^32` moved by a raw unit, or a random or negative radius.
fn some_circle(state: &mut u64, p: FixedVec2, step: [i128; 2]) -> FixedCircle2 {
    let n = i128::from(small(state, 3));
    let centre = offset(p, n, [-step[1], step[0]], [jitter(state), jitter(state)]);
    let radius = match next(state) % 8 {
        0 => -1 - (spread(state).unsigned_abs() >> 1).cast_signed(),
        1 => spread(state).abs(),
        _ => sat(n.abs() * i128::from(U) + jitter(state)),
    };
    FixedCircle2::new(centre, raw(radius))
}

/// A segment across the ray's line at `p`, along it, ending on it, zero-length, or random.
fn some_segment(state: &mut u64, p: FixedVec2, step: [i128; 2]) -> FixedSegment2 {
    let perp = [-step[1], step[0]];
    let j0 = [jitter(state), jitter(state)];
    let j1 = [jitter(state), jitter(state)];
    match next(state) % 5 {
        0 => FixedSegment2::new(offset(p, -1, perp, j0), offset(p, 1, perp, j1)),
        1 => {
            let k = i128::from(small(state, 6));
            FixedSegment2::new(offset(p, 0, step, [0, 0]), offset(p, k, step, [0, 0]))
        }
        2 => FixedSegment2::new(offset(p, 0, perp, j0), offset(p, 2, perp, [0, 0])),
        3 => FixedSegment2::new(offset(p, 0, perp, j0), offset(p, 0, perp, j0)),
        _ => FixedSegment2::new(
            v(coordinate(state), coordinate(state)),
            v(coordinate(state), coordinate(state)),
        ),
    }
}

/// Whether the segment is parallel to the ray's direction or zero-length.
fn parallel(r: FixedRay2, s: FixedSegment2) -> bool {
    let d = wide(r.direction.into_inner());
    let (a, b) = (wide(s.start), wide(s.end));
    let ds = [b[0] - a[0], b[1] - a[1]];
    // Each product is below `2^33 2^64`, so the difference fits `i128`.
    d[0] * ds[1] - d[1] * ds[0] == 0
}

fn tally(counts: &mut [u32; 3], hit: RayHit2) {
    match hit {
        RayHit2::Miss => {}
        RayHit2::At(t) => {
            counts[0] += 1;
            counts[1] += u32::from(t.raw() == 0);
        }
        RayHit2::Beyond => counts[2] += 1,
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn planar_tp_018_seeded_ray_queries_match_the_oracle() {
    const CASES: u32 = 30_000;
    println!("planar_tp_018 seed={SEED_RAYS:#018x} cases={CASES}");
    let mut state = SEED_RAYS;
    // per shape: hit at a parameter, of which at zero, beyond the range
    let mut counts = [[0_u32; 3]; 3];
    let mut parallel_hits = 0_u32;
    for case in 0..CASES {
        let (r, p, step) = ray_and_target(&mut state);
        let context = format!("seed={SEED_RAYS:#018x} case={case} {r:?}");

        let aabb = some_box(&mut state, p);
        let hit = r.hit_aabb(&aabb);
        assert_eq!(r.intersects_aabb(&aabb), hit.is_hit(), "{context} {aabb:?}");
        let exact = planar_cast_oracle::ray_box(r, aabb);
        assert!(
            planar_cast_oracle::parameter_is(exact.as_ref(), hit),
            "{context} {aabb:?} {hit:?}"
        );
        tally(&mut counts[0], hit);

        let c = some_circle(&mut state, p, step);
        let hit = r.hit_circle(&c);
        assert_eq!(r.intersects_circle(&c), hit.is_hit(), "{context} {c:?}");
        assert!(planar_cast_oracle::circle_hit_is(r, c, hit), "{context} {c:?} {hit:?}");
        tally(&mut counts[1], hit);

        let s = some_segment(&mut state, p, step);
        let hit = r.hit_segment(&s);
        assert_eq!(r.intersects_segment(&s), hit.is_hit(), "{context} {s:?}");
        assert_eq!(r.hit_segment(&FixedSegment2::new(s.end, s.start)), hit, "{context} {s:?}");
        let exact = planar_cast_oracle::ray_segment(r, s.start, s.end);
        assert!(planar_cast_oracle::parameter_is(exact.as_ref(), hit), "{context} {s:?} {hit:?}");
        tally(&mut counts[2], hit);
        parallel_hits += u32::from(hit.is_hit() && parallel(r, s));
    }
    println!("planar_tp_018 [hit,at_zero,beyond] box,circle,segment={counts:?} parallel_hits={parallel_hits}");
    assert_eq!(
        counts,
        [[8_788, 3_958, 4], [8_879, 4_183, 2], [9_330, 2_521, 105]],
        "seed={SEED_RAYS:#018x}"
    );
    assert_eq!(parallel_hits, 4_001, "seed={SEED_RAYS:#018x}");
}

/// A polygon of zero to eight vertices: on a small lattice (many repeated and collinear
/// vertices, and self-intersections), of any magnitude, or from the ends of the range.
fn some_polygon(state: &mut u64) -> (u64, Vec<FixedVec2>) {
    let kind = next(state) % 4;
    let count = usize::try_from(next(state) % 9).expect("count");
    let vertices = (0..count)
        .map(|_| match kind {
            0 | 1 => v(small(state, 4), small(state, 4)),
            2 => v(spread(state), spread(state)),
            _ => v(extreme(state), extreme(state)),
        })
        .collect();
    (kind, vertices)
}

/// A vertex, the midpoint of an edge (rounded down) or a random point, moved by up to one raw
/// unit on each axis.
fn near_polygon(state: &mut u64, vertices: &[FixedVec2]) -> FixedVec2 {
    if vertices.is_empty() || next(state).is_multiple_of(5) {
        return v(coordinate(state), coordinate(state));
    }
    let i = usize::try_from(next(state)).expect("index") % vertices.len();
    let (a, b) = (vertices[i], vertices[(i + 1) % vertices.len()]);
    let base = if next(state).is_multiple_of(2) {
        a
    } else {
        v(a.x.raw().midpoint(b.x.raw()), a.y.raw().midpoint(b.y.raw()))
    };
    offset(base, 0, [0, 0], [jitter(state), jitter(state)])
}

#[wasm_bindgen_test(unsupported = test)]
fn planar_tp_019_seeded_point_in_polygon_matches_the_oracle() {
    const CASES: u32 = 30_000;
    println!("planar_tp_019 seed={SEED_POLYGONS:#018x} cases={CASES}");
    let mut state = SEED_POLYGONS;
    // inside, boundary, outside, inside with winding number other than one
    let mut counts = [0_u32; 4];
    for case in 0..CASES {
        let (kind, vertices) = some_polygon(&mut state);
        let p = near_polygon(&mut state, &vertices);
        let context =
            format!("seed={SEED_POLYGONS:#018x} case={case} kind={kind} {vertices:?} {p:?}");
        let got = FixedPolygon2::new(&vertices).classify_point(p);
        let winding = planar_cast_oracle::winding(&vertices, p);
        let expected = match winding {
            None => PolygonLocation::Boundary,
            Some(0) => PolygonLocation::Outside,
            Some(_) => PolygonLocation::Inside,
        };
        assert_eq!(got, expected, "{context}");
        let mut reversed = vertices.clone();
        reversed.reverse();
        assert_eq!(FixedPolygon2::new(&reversed).classify_point(p), got, "{context}");
        if !vertices.is_empty() {
            let mut rotated = vertices.clone();
            rotated.rotate_left(1);
            assert_eq!(FixedPolygon2::new(&rotated).classify_point(p), got, "{context}");
        }
        counts[match got {
            PolygonLocation::Inside => 0,
            PolygonLocation::Boundary => 1,
            PolygonLocation::Outside => 2,
        }] += 1;
        counts[3] += u32::from(winding.is_some_and(|w| w.abs() > 1));
    }
    println!("planar_tp_019 inside,boundary,outside,winding_above_one={counts:?}");
    assert_eq!(counts, [4_342, 3_841, 21_817, 106], "seed={SEED_POLYGONS:#018x}");
}
