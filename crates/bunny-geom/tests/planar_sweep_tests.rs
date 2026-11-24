//! Seeded sweeps of the 2D primitives and queries against the independent big-integer oracle
//! (`bunny-planar/1`, `docs/topics/planar-geometry/`).
//!
//! Random inputs almost never touch, so the generators build touching cases deliberately:
//! points on a box's boundary, on a circle (`(3k, 4k)` from the centre with radius `5k`),
//! tangent circles, segments on one lattice line, each then moved by one raw unit, at small
//! magnitudes and at every magnitude up to the ends of the raw range. The outcome counts of
//! each sweep are pinned, so a generator that stops producing boundary cases fails the test.

// The queries name their points `a` to `d` and `p` as the chapter does.
#![allow(clippy::many_single_char_names)]

mod planar_oracle;
mod predicates_oracle;

use bunny_geom::{FixedAabb2, FixedCircle2, FixedSegment2};
use bunny_linalg::FixedVec2;
use bunny_num::FixedQ32_32;
use predicates_oracle::{rounds_to, Big};
use wasm_bindgen_test::wasm_bindgen_test;

const SEED_BOXES: u64 = 0x0149_0001_0000_0009;
const SEED_SEGMENTS: u64 = 0x0149_0001_0000_000A;
const SEED_CLOSEST: u64 = 0x0149_0001_0000_000B;
const EXTREMES: [i64; 8] = [i64::MIN, i64::MIN + 1, -2, -1, 0, 1, i64::MAX - 1, i64::MAX];

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

/// `-1`, `0` or `1`.
fn jitter(state: &mut u64) -> i64 {
    small(state, 1)
}

/// The value clamped to the raw range; the generators use it to keep constructed points
/// representable.
fn sat(value: i128) -> i64 {
    i64::try_from(value).unwrap_or(if value < 0 { i64::MIN } else { i64::MAX })
}

fn raw(v: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(v)
}

fn v(x: i64, y: i64) -> FixedVec2 {
    FixedVec2::new(raw(x), raw(y))
}

fn moved(p: FixedVec2, dx: i128, dy: i128) -> FixedVec2 {
    v(sat(i128::from(p.x.raw()) + dx), sat(i128::from(p.y.raw()) + dy))
}

/// A box from two random corners, sorted three times in four so that a quarter are inverted
/// on some axis whenever the corners differ.
fn random_box(state: &mut u64) -> FixedAabb2 {
    let (x0, x1, y0, y1) =
        (coordinate(state), coordinate(state), coordinate(state), coordinate(state));
    if next(state) % 4 == 0 {
        FixedAabb2::new(v(x0, y0), v(x1, y1))
    } else {
        FixedAabb2::new(v(x0.min(x1), y0.min(y1)), v(x0.max(x1), y0.max(y1)))
    }
}

/// A point on the boundary of `aabb` (or of where it would be, for an inverted box) moved by
/// up to one raw unit on each axis, or an unrelated point.
fn near_box(state: &mut u64, aabb: FixedAabb2) -> FixedVec2 {
    let pick = |state: &mut u64, lo: i64, hi: i64| match next(state) % 3 {
        0 => lo,
        1 => hi,
        _ => sat((i128::from(lo) + i128::from(hi)) / 2),
    };
    if next(state) % 5 == 0 {
        return v(coordinate(state), coordinate(state));
    }
    let x = pick(state, aabb.min.x.raw(), aabb.max.x.raw());
    let y = pick(state, aabb.min.y.raw(), aabb.max.y.raw());
    moved(v(x, y), i128::from(jitter(state)), i128::from(jitter(state)))
}

/// A Pythagorean step `(3k, 4k)` in one of the eight sign and swap orientations, and its
/// length `5k`, with `k` small or large.
fn pythagorean(state: &mut u64) -> (i128, i128, i128) {
    let k = if next(state).is_multiple_of(2) {
        i128::from(small(state, 8).abs())
    } else {
        i128::from(spread(state).unsigned_abs() >> 3)
    };
    let (mut dx, mut dy) = (3 * k, 4 * k);
    if next(state).is_multiple_of(2) {
        (dx, dy) = (dy, dx);
    }
    if next(state).is_multiple_of(2) {
        dx = -dx;
    }
    if next(state).is_multiple_of(2) {
        dy = -dy;
    }
    (dx, dy, 5 * k)
}

/// A radius from the generator's construction moved by up to one raw unit, or a random one,
/// negative one time in sixteen.
fn radius(state: &mut u64, exact: i128) -> FixedQ32_32 {
    if next(state) % 16 == 0 {
        return raw(-1 - (spread(state).unsigned_abs() >> 1).cast_signed());
    }
    raw(sat(exact + i128::from(jitter(state))))
}

#[wasm_bindgen_test(unsupported = test)]
fn planar_tp_009_seeded_box_and_circle_queries_match_the_oracle() {
    const CASES: u32 = 40_000;
    println!("planar_tp_009 seed={SEED_BOXES:#018x} cases={CASES}");
    let mut state = SEED_BOXES;
    // box contains, boxes overlap, inverted boxes, circle contains, circles meet,
    // circle meets box
    let mut counts = [0_u32; 6];
    for case in 0..CASES {
        let context = format!("seed={SEED_BOXES:#018x} case={case}");

        let aabb = random_box(&mut state);
        let p = near_box(&mut state, aabb);
        let contains = aabb.contains_point(p);
        assert_eq!(contains, planar_oracle::box_contains(aabb, p), "{context} {aabb:?} {p:?}");
        let closest = aabb.closest_point(p);
        let oracle = planar_oracle::box_closest(aabb, p).map(|[x, y]| v(x, y));
        assert_eq!(closest, oracle, "{context} {aabb:?} {p:?}");
        assert_eq!(closest.is_some(), aabb.is_ordered(), "{context}");
        if let Some(q) = closest {
            assert!(aabb.contains_point(q), "{context} {q:?}");
            assert_eq!(q == p, contains, "{context}");
        }

        // A second box touching the first at `p`'s neighbourhood.
        let other = {
            let corner = near_box(&mut state, aabb);
            let far = v(coordinate(&mut state), coordinate(&mut state));
            FixedAabb2::new(
                v(corner.x.raw().min(far.x.raw()), corner.y.raw().min(far.y.raw())),
                v(corner.x.raw().max(far.x.raw()), corner.y.raw().max(far.y.raw())),
            )
        };
        let overlaps = aabb.overlaps(&other);
        assert_eq!(overlaps, other.overlaps(&aabb), "{context}");
        assert_eq!(
            overlaps,
            planar_oracle::boxes_meet(aabb, other),
            "{context} {aabb:?} {other:?}"
        );

        // A point `(3k, 4k)` from a centre with radius `5k`, moved by up to one raw unit.
        let centre = v(coordinate(&mut state), coordinate(&mut state));
        let (dx, dy, length) = pythagorean(&mut state);
        let c = FixedCircle2::new(centre, radius(&mut state, length));
        let q = moved(centre, dx, dy);
        let in_circle = c.contains_point(q);
        assert_eq!(in_circle, planar_oracle::circle_contains(c, q), "{context} {c:?} {q:?}");

        // Tangent circles: centres `5k` apart and radii summing to about `5k`.
        let (dx, dy, length) = pythagorean(&mut state);
        let split = i128::from(small(&mut state, 4).abs()).min(length);
        let a = FixedCircle2::new(centre, radius(&mut state, split));
        let b = FixedCircle2::new(moved(centre, dx, dy), radius(&mut state, length - split));
        let meet = a.overlaps_circle(&b);
        assert_eq!(meet, b.overlaps_circle(&a), "{context}");
        assert_eq!(meet, planar_oracle::circles_meet(a, b), "{context} {a:?} {b:?}");

        // A circle about a point near the box with a radius near its distance to the box.
        let near = near_box(&mut state, aabb);
        let (dx, dy, length) = pythagorean(&mut state);
        let around = if next(&mut state).is_multiple_of(2) {
            FixedCircle2::new(moved(near, dx, dy), radius(&mut state, length))
        } else {
            FixedCircle2::new(near, radius(&mut state, i128::from(small(&mut state, 2).abs())))
        };
        let touches = around.overlaps_aabb(&aabb);
        let expected = planar_oracle::circle_meets_box(around, aabb);
        assert_eq!(touches, expected, "{context} {around:?} {aabb:?}");

        counts[0] += u32::from(contains);
        counts[1] += u32::from(overlaps);
        counts[2] += u32::from(!aabb.is_ordered());
        counts[3] += u32::from(in_circle);
        counts[4] += u32::from(meet);
        counts[5] += u32::from(touches);
    }
    println!("planar_tp_009 box_contains,boxes_meet,inverted,circle_contains,circles_meet,circle_box={counts:?}");
    assert_eq!(counts, [0, 0, 0, 0, 0, 0], "seed={SEED_BOXES:#018x}");
}

/// Four points of one of six kinds: on one lattice line at small or any magnitude, the same
/// with one raw unit moved, small independent values (many shared and repeated points), and
/// values from the ends of the raw range.
fn four_points(state: &mut u64) -> (u64, [FixedVec2; 4]) {
    let kind = next(state) % 6;
    let mut out = match kind {
        0 | 2 => line_points(
            state,
            [small(state, 1 << 10), small(state, 1 << 10)],
            [small(state, 9), small(state, 9)],
            6,
        ),
        1 | 3 => {
            line_points(state, [spread(state), spread(state)], [spread(state), spread(state)], 3)
        }
        4 => [0; 4].map(|_| v(small(state, 3), small(state, 3))),
        _ => [0; 4].map(|_| v(extreme(state), extreme(state))),
    };
    if kind == 2 || kind == 3 {
        let point = usize::try_from(next(state) % 4).expect("index");
        let (dx, dy) = if next(state).is_multiple_of(2) { (1, 0) } else { (0, 1) };
        let sign = if next(state).is_multiple_of(2) { 1 } else { -1 };
        out[point] = moved(out[point], sign * dx, sign * dy);
    }
    (kind, out)
}

/// Points `base + t step` for `t` in `-bound..=bound`, with the step halved until every
/// point fits.
fn line_points(state: &mut u64, base: [i64; 2], step: [i64; 2], bound: u64) -> [FixedVec2; 4] {
    let ts = [0; 4].map(|_| i128::from(small(state, bound)));
    let mut step = step.map(i128::from);
    loop {
        let fits = ts.iter().all(|t| {
            let x = i128::from(base[0]) + t * step[0];
            let y = i128::from(base[1]) + t * step[1];
            i64::try_from(x).is_ok() && i64::try_from(y).is_ok()
        });
        if fits {
            return ts.map(|t| {
                v(sat(i128::from(base[0]) + t * step[0]), sat(i128::from(base[1]) + t * step[1]))
            });
        }
        step = step.map(|s| s >> 1);
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn planar_tp_010_seeded_segment_predicates_match_the_oracle() {
    const CASES: u32 = 60_000;
    println!("planar_tp_010 seed={SEED_SEGMENTS:#018x} cases={CASES}");
    let mut state = SEED_SEGMENTS;
    // meet, endpoint on the other segment, meet among the collinear-line kinds, meet among
    // the nudged kinds, zero-length segments
    let mut counts = [0_u32; 5];
    for case in 0..CASES {
        let (kind, [a, b, c, d]) = four_points(&mut state);
        let context =
            format!("seed={SEED_SEGMENTS:#018x} case={case} kind={kind} {a:?} {b:?} {c:?} {d:?}");
        let (s, t) = (FixedSegment2::new(a, b), FixedSegment2::new(c, d));
        let meet = s.intersects(&t);
        assert_eq!(meet, planar_oracle::segments_meet(a, b, c, d), "{context}");
        for (x, y) in [
            (FixedSegment2::new(b, a), t),
            (s, FixedSegment2::new(d, c)),
            (FixedSegment2::new(b, a), FixedSegment2::new(d, c)),
        ] {
            assert_eq!(x.intersects(&y), meet, "{context}");
            assert_eq!(y.intersects(&x), meet, "{context}");
        }
        let mut on = false;
        for (segment, point) in [(s, c), (s, d), (t, a), (t, b)] {
            let inside = segment.contains_point(point);
            let expected = planar_oracle::on_segment(segment.start, segment.end, point);
            assert_eq!(inside, expected, "{context} {segment:?} {point:?}");
            let back = FixedSegment2::new(segment.end, segment.start);
            assert_eq!(back.contains_point(point), inside, "{context}");
            // An endpoint on the other segment is a shared point.
            assert!(!inside || meet, "{context}");
            on |= inside;
        }
        counts[0] += u32::from(meet);
        counts[1] += u32::from(on);
        counts[2] += u32::from(meet && kind <= 1);
        counts[3] += u32::from(meet && (kind == 2 || kind == 3));
        counts[4] += u32::from(a == b || c == d);
    }
    println!("planar_tp_010 meet,endpoint_on,line_meet,nudged_meet,zero_length={counts:?}");
    assert_eq!(counts, [0, 0, 0, 0, 0], "seed={SEED_SEGMENTS:#018x}");
}

#[wasm_bindgen_test(unsupported = test)]
fn planar_tp_011_seeded_segment_closest_points_match_the_oracle() {
    const CASES: u32 = 40_000;
    println!("planar_tp_011 seed={SEED_CLOSEST:#018x} cases={CASES}");
    let mut state = SEED_CLOSEST;
    // at start, at end, strictly between, a tie on some axis, zero-length
    let mut counts = [0_u32; 5];
    for case in 0..CASES {
        let (kind, [a, b, p, _]) = four_points(&mut state);
        let context =
            format!("seed={SEED_CLOSEST:#018x} case={case} kind={kind} {a:?} {b:?} {p:?}");
        let s = FixedSegment2::new(a, b);
        let got = s.closest_point(p).expect("never refused");
        assert_eq!(FixedSegment2::new(b, a).closest_point(p), Some(got), "{context}");
        let ([nx, ny], den) = planar_oracle::segment_closest_exact(s, p);
        assert!(rounds_to(&nx, &den, Some(got.x.raw())), "{context} x {got:?}");
        assert!(rounds_to(&ny, &den, Some(got.y.raw())), "{context} y {got:?}");
        // The result is in the endpoints' box.
        let box_ = FixedAabb2::new(
            v(a.x.raw().min(b.x.raw()), a.y.raw().min(b.y.raw())),
            v(a.x.raw().max(b.x.raw()), a.y.raw().max(b.y.raw())),
        );
        assert!(box_.contains_point(got), "{context} {got:?}");
        let tie = |n: &Big, r: i64| {
            let error = n.sub(&Big::from_i64(r).mul(&den)).abs();
            error.add(&error).cmp(&den).is_eq()
        };
        counts[0] += u32::from(a != b && got == a);
        counts[1] += u32::from(a != b && got == b);
        counts[2] += u32::from(got != a && got != b);
        counts[3] += u32::from(tie(&nx, got.x.raw()) || tie(&ny, got.y.raw()));
        counts[4] += u32::from(a == b);
    }
    println!("planar_tp_011 start,end,between,tie,zero_length={counts:?}");
    assert_eq!(counts, [0, 0, 0, 0, 0], "seed={SEED_CLOSEST:#018x}");
}
