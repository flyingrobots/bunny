//! Seeded sweeps of the exact predicates against the independent big-integer oracle
//! (`bunny-predicates/1`, `docs/topics/predicates/`).
//!
//! Random points are almost never collinear or coplanar, so the generators build those cases
//! deliberately: points on an integer lattice line or plane, then the same moved by one raw
//! unit on one coordinate, at small magnitudes and at every magnitude up to the ends of the
//! raw range. The outcome counts of each sweep are pinned, so a generator that stops
//! producing degenerate cases fails the test.

mod predicates_oracle;

use core::cmp::Ordering;

use bunny_geom::{
    barycentric2, doubled_signed_area2, orientation2, orientation3, sextupled_signed_volume3,
    signed_area2_rounded, signed_volume3_rounded, ExactDeterminant, Orientation2, Orientation3,
    TriangleLocation,
};
use bunny_linalg::{FixedVec2, FixedVec3};
use bunny_num::FixedQ32_32;
use predicates_oracle::{det2, det3, rounds_to, same, Big};
use wasm_bindgen_test::wasm_bindgen_test;

const SEED_2D: u64 = 0x0145_0001_0000_0002;
const SEED_3D: u64 = 0x0145_0001_0000_0003;
const SEED_BARY: u64 = 0x0145_0001_0000_0005;
const EXTREMES: [i64; 8] = [i64::MIN, i64::MIN + 1, -2, -1, 0, 1, i64::MAX - 1, i64::MAX];

/// SplitMix64.
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

fn raw(v: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(v)
}

/// `base + k * step` on every axis, or `None` when a coordinate leaves `i64`.
fn along<const N: usize>(base: [i64; N], terms: &[(i64, [i64; N])]) -> Option<[i64; N]> {
    let mut out = [0; N];
    for (axis, slot) in out.iter_mut().enumerate() {
        let mut sum = i128::from(base[axis]);
        for (k, step) in terms {
            sum += i128::from(*k) * i128::from(step[axis]);
        }
        *slot = i64::try_from(sum).ok()?;
    }
    Some(out)
}

/// Moves one coordinate of one point by one raw unit, toward zero at the ends of the range.
fn nudge<const N: usize>(state: &mut u64, points: &mut [[i64; N]]) {
    let point = usize::try_from(next(state) % points.len() as u64).expect("index");
    let axis = usize::try_from(next(state) % N as u64).expect("index");
    let value = &mut points[point][axis];
    let moved = if next(state) % 2 == 0 {
        value.checked_add(1).or_else(|| value.checked_sub(1))
    } else {
        value.checked_sub(1).or_else(|| value.checked_add(1))
    };
    if let Some(moved) = moved {
        *value = moved;
    }
}

/// Lattice points `base + s u + t v` with the steps halved until every point fits.
fn lattice<const N: usize>(
    state: &mut u64,
    base: [i64; N],
    mut u: [i64; N],
    mut v: [i64; N],
    count: usize,
    bound: u64,
) -> Vec<[i64; N]> {
    let coefficients: Vec<(i64, i64)> =
        (0..count).map(|_| (small(state, bound), small(state, bound))).collect();
    loop {
        let points: Option<Vec<[i64; N]>> =
            coefficients.iter().map(|&(s, t)| along(base, &[(s, u), (t, v)])).collect();
        if let Some(points) = points {
            return points;
        }
        u = u.map(|x| x >> 1);
        v = v.map(|x| x >> 1);
    }
}

/// Points of one of six kinds: on a lattice line or plane at small magnitude or any
/// magnitude, the same with one raw unit moved, independent values of any magnitude, and
/// values from the ends of the raw range.
fn points<const N: usize>(state: &mut u64, count: usize, flat: usize) -> (u64, Vec<[i64; N]>) {
    let kind = next(state) % 6;
    let mut out = match kind {
        0 | 2 => {
            let base = [0; N].map(|_| small(state, 1 << 10));
            let u = [0; N].map(|_| small(state, 64));
            let v = if flat == 1 { u } else { [0; N].map(|_| small(state, 64)) };
            lattice(state, base, u, v, count, 8)
        }
        1 | 3 => {
            let base = [0; N].map(|_| spread(state));
            let u = [0; N].map(|_| spread(state));
            let v = if flat == 1 { u } else { [0; N].map(|_| spread(state)) };
            lattice(state, base, u, v, count, 3)
        }
        4 => (0..count).map(|_| [0; N].map(|_| spread(state))).collect(),
        _ => (0..count).map(|_| [0; N].map(|_| extreme(state))).collect(),
    };
    if kind == 2 || kind == 3 {
        nudge(state, &mut out);
    }
    (kind, out)
}

fn v2(p: [i64; 2]) -> FixedVec2 {
    FixedVec2::new(raw(p[0]), raw(p[1]))
}

fn v3(p: [i64; 3]) -> FixedVec3 {
    FixedVec3::new(raw(p[0]), raw(p[1]), raw(p[2]))
}

fn sign_of(o: Orientation2) -> i32 {
    match o {
        Orientation2::CounterClockwise => 1,
        Orientation2::Clockwise => -1,
        Orientation2::Collinear => 0,
    }
}

fn sign3_of(o: Orientation3) -> i32 {
    match o {
        Orientation3::Front => 1,
        Orientation3::Behind => -1,
        Orientation3::Coplanar => 0,
    }
}

/// Whether `x == -y` exactly.
fn negated(x: ExactDeterminant, y: ExactDeterminant) -> bool {
    x.magnitude() == y.magnitude() && x.sign() == y.sign().reverse()
}

#[wasm_bindgen_test(unsupported = test)]
fn pred_tp_008_seeded_2d_orientations_match_the_oracle() {
    const CASES: u32 = 60_000;
    println!("pred_tp_008 seed={SEED_2D:#018x} cases={CASES}");
    let mut state = SEED_2D;
    let mut counts = [0_u32; 6]; // collinear, ccw, cw, rounded None, rounded 0 but not zero, nudged nonzero
    for case in 0..CASES {
        let (kind, p) = points::<2>(&mut state, 3, 1);
        let (a, b, c) = (v2(p[0]), v2(p[1]), v2(p[2]));
        let context = format!("seed={SEED_2D:#018x} case={case} {p:?}");
        let oracle = det2(a, b, c);
        assert!(same(doubled_signed_area2(a, b, c), &oracle), "{context}");
        let sign = sign_of(orientation2(a, b, c));
        assert_eq!(sign, oracle.signum(), "{context}");
        let rounded = signed_area2_rounded(a, b, c).map(FixedQ32_32::raw);
        assert!(rounds_to(&oracle, &Big::from_i128(1 << 33), rounded), "{context}");
        let small = oracle.abs().cmp(&Big::from_i128(1 << 32)).is_le();
        if let Some(r) = rounded {
            assert_eq!(r == 0, small, "{context}");
            assert!(r == 0 || r.signum() == i64::from(sign), "{context}");
        }
        counts[match sign {
            0 => 0,
            1 => 1,
            _ => 2,
        }] += 1;
        counts[3] += u32::from(rounded.is_none());
        counts[4] += u32::from(rounded == Some(0) && sign != 0);
        counts[5] += u32::from((kind == 2 || kind == 3) && sign != 0);
    }
    println!("pred_tp_008 collinear,ccw,cw,refused,rounded_zero,nudged={counts:?}");
    assert_eq!(counts, [0; 6], "seed={SEED_2D:#018x}");
}

#[wasm_bindgen_test(unsupported = test)]
fn pred_tp_009_seeded_3d_orientations_match_the_oracle() {
    const CASES: u32 = 40_000;
    println!("pred_tp_009 seed={SEED_3D:#018x} cases={CASES}");
    let mut state = SEED_3D;
    let six = Big::from_i128(6 << 64);
    let three = Big::from_i128(3 << 64);
    let mut counts = [0_u32; 6]; // coplanar, front, behind, rounded None, rounded 0 but not zero, nudged nonzero
    for case in 0..CASES {
        let (kind, p) = points::<3>(&mut state, 4, 2);
        let (a, b, c, d) = (v3(p[0]), v3(p[1]), v3(p[2]), v3(p[3]));
        let context = format!("seed={SEED_3D:#018x} case={case} {p:?}");
        let oracle = det3(a, b, c, d);
        assert!(same(sextupled_signed_volume3(a, b, c, d), &oracle), "{context}");
        let sign = sign3_of(orientation3(a, b, c, d));
        assert_eq!(sign, oracle.signum(), "{context}");
        let rounded = signed_volume3_rounded(a, b, c, d).map(FixedQ32_32::raw);
        assert!(rounds_to(&oracle, &six, rounded), "{context}");
        if let Some(r) = rounded {
            assert_eq!(r == 0, oracle.abs().cmp(&three).is_le(), "{context}");
            assert!(r == 0 || r.signum() == i64::from(sign), "{context}");
        }
        counts[match sign {
            0 => 0,
            1 => 1,
            _ => 2,
        }] += 1;
        counts[3] += u32::from(rounded.is_none());
        counts[4] += u32::from(rounded == Some(0) && sign != 0);
        counts[5] += u32::from((kind == 2 || kind == 3) && sign != 0);
    }
    println!("pred_tp_009 coplanar,front,behind,refused,rounded_zero,nudged={counts:?}");
    assert_eq!(counts, [0; 6], "seed={SEED_3D:#018x}");
}

/// A triangle with one lattice edge and a point on a vertex, on that edge, one raw unit off
/// it, or anywhere; a tenth of the triangles are collinear.
fn triangle_and_point(state: &mut u64) -> [[i64; 2]; 4] {
    let wide = next(state) % 2 == 0;
    let pick = |state: &mut u64| if wide { spread(state) } else { small(state, 1 << 12) };
    let a = [pick(state), pick(state)];
    let (e, f) = ([pick(state), pick(state)], [pick(state), pick(state)]);
    let m = small(state, 4).abs() + 1;
    let collinear = next(state) % 10 == 0;
    let third = if collinear { e } else { f };
    let mut tri = lattice_triangle(a, e, third, m);
    let j = i64::try_from(next(state) % u64::try_from(m + 1).expect("m")).expect("j");
    let mut p = match next(state) % 4 {
        0 => tri[usize::try_from(next(state) % 3).expect("index")],
        1 | 2 => along(tri[0], &[(j, tri[3])]).unwrap_or(tri[0]),
        _ => [pick(state), pick(state)],
    };
    if next(state) % 3 == 0 {
        let mut one = [p];
        nudge(state, &mut one);
        p = one[0];
    }
    tri[3] = p;
    // Relabel the vertices by one of the six permutations.
    let order = [[0, 1, 2], [1, 2, 0], [2, 0, 1], [1, 0, 2], [0, 2, 1], [2, 1, 0]];
    let o = order[usize::try_from(next(state) % 6).expect("index")];
    [tri[o[0]], tri[o[1]], tri[o[2]], tri[3]]
}

/// `[a, b, c, step]` with `b = a + m e` and `c = b + f`, steps halved until they fit; the
/// edge `a`-`b` holds the lattice points `a + j step`.
fn lattice_triangle(a: [i64; 2], mut e: [i64; 2], mut f: [i64; 2], m: i64) -> [[i64; 2]; 4] {
    loop {
        if let Some(b) = along(a, &[(m, e)]) {
            if let Some(c) = along(b, &[(1, f)]) {
                return [a, b, c, e];
            }
        }
        e = e.map(|x| x >> 1);
        f = f.map(|x| x >> 1);
    }
}

fn location_rule(n: &[Big; 3]) -> TriangleLocation {
    use bunny_geom::{TriangleEdge as E, TriangleVertex as V};
    let zero = |k: usize| n[k].is_zero();
    if n.iter().any(|x| x.signum() < 0) {
        return TriangleLocation::Outside;
    }
    match (zero(0), zero(1), zero(2)) {
        (false, true, true) => TriangleLocation::OnVertex(V::A),
        (true, false, true) => TriangleLocation::OnVertex(V::B),
        (true, true, false) => TriangleLocation::OnVertex(V::C),
        (true, false, false) => TriangleLocation::OnEdge(E::BC),
        (false, true, false) => TriangleLocation::OnEdge(E::CA),
        (false, false, true) => TriangleLocation::OnEdge(E::AB),
        _ => TriangleLocation::Inside,
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn pred_tp_010_seeded_barycentrics_match_the_oracle() {
    const CASES: u32 = 40_000;
    println!("pred_tp_010 seed={SEED_BARY:#018x} cases={CASES}");
    let mut state = SEED_BARY;
    // degenerate, inside, on edge, on vertex, outside, weights refused
    let mut counts = [0_u32; 6];
    for case in 0..CASES {
        let [a, b, c, p] = triangle_and_point(&mut state).map(v2);
        let context = format!("seed={SEED_BARY:#018x} case={case} {p:?} in {a:?} {b:?} {c:?}");
        let den = det2(a, b, c);
        let Some(bary) = barycentric2(p, a, b, c) else {
            assert!(den.is_zero(), "{context}");
            counts[0] += 1;
            continue;
        };
        let flip = |x: Big| if den.signum() < 0 { x.neg() } else { x };
        let n = [flip(det2(p, b, c)), flip(det2(a, p, c)), flip(det2(a, b, p))];
        for (kernel, oracle) in bary.numerators().iter().zip(&n) {
            assert!(same(*kernel, oracle), "{context}");
        }
        assert!(same(bary.denominator(), &den.abs()), "{context}");
        let location = bary.location();
        assert_eq!(location, location_rule(&n), "{context}");
        assert_eq!(bary.contains_closed(), location != TriangleLocation::Outside, "{context}");
        assert_eq!(bary.contains_open(), location == TriangleLocation::Inside, "{context}");
        counts[match location {
            TriangleLocation::Inside => 1,
            TriangleLocation::OnEdge(_) => 2,
            TriangleLocation::OnVertex(_) => 3,
            TriangleLocation::Outside => 4,
        }] += 1;
        let scaled = n.map(|x| x.shl(32));
        match bary.weights_rounded() {
            Some(w) => {
                let w = w.map(FixedQ32_32::raw);
                for (num, w) in scaled.iter().zip(w) {
                    assert!(rounds_to(num, &den.abs(), Some(w)), "{context}");
                }
                let sum: i128 = w.iter().map(|&x| i128::from(x)).sum();
                assert!((sum - (1 << 32)).abs() <= 1, "{context}: {w:?}");
            }
            None => {
                assert!(scaled.iter().any(|x| rounds_to(x, &den.abs(), None)), "{context}");
                counts[5] += 1;
            }
        }
    }
    println!("pred_tp_010 degenerate,inside,edge,vertex,outside,refused={counts:?}");
    assert_eq!(counts, [0; 6], "seed={SEED_BARY:#018x}");
}

#[wasm_bindgen_test(unsupported = test)]
fn pred_tp_011_orientations_are_consistent_under_permutation() {
    let mut state = SEED_2D;
    for case in 0..10_000 {
        let (_, p) = points::<2>(&mut state, 3, 1);
        let [a, b, c] = [v2(p[0]), v2(p[1]), v2(p[2])];
        let context = format!("seed={SEED_2D:#018x} case={case} {p:?}");
        let d = doubled_signed_area2(a, b, c);
        assert_eq!(doubled_signed_area2(b, c, a), d, "{context}");
        assert_eq!(doubled_signed_area2(c, a, b), d, "{context}");
        for swapped in [(b, a, c), (a, c, b), (c, b, a)] {
            let s = doubled_signed_area2(swapped.0, swapped.1, swapped.2);
            assert!(negated(s, d), "{context}");
            assert_eq!(
                sign_of(orientation2(swapped.0, swapped.1, swapped.2)),
                -sign_of(orientation2(a, b, c))
            );
        }
    }
    let mut state = SEED_3D;
    let perms = permutations4();
    for case in 0..5_000 {
        let (_, p) = points::<3>(&mut state, 4, 2);
        let q = [v3(p[0]), v3(p[1]), v3(p[2]), v3(p[3])];
        let context = format!("seed={SEED_3D:#018x} case={case} {p:?}");
        let d = sextupled_signed_volume3(q[0], q[1], q[2], q[3]);
        for (perm, even) in &perms {
            let s = sextupled_signed_volume3(q[perm[0]], q[perm[1]], q[perm[2]], q[perm[3]]);
            assert!(if *even { s == d } else { negated(s, d) }, "{context} {perm:?}");
        }
    }
    let mut state = SEED_BARY;
    for case in 0..10_000 {
        let [a, b, c, p] = triangle_and_point(&mut state).map(v2);
        let Some(bary) = barycentric2(p, a, b, c) else { continue };
        let context = format!("seed={SEED_BARY:#018x} case={case}");
        let t = sign_of(orientation2(a, b, c));
        let subs = [orientation2(p, b, c), orientation2(a, p, c), orientation2(a, b, p)];
        for (n, o) in bary.numerators().iter().zip(subs) {
            let s = match n.sign() {
                Ordering::Less => -1,
                Ordering::Equal => 0,
                Ordering::Greater => 1,
            };
            assert_eq!(s, sign_of(o) * t, "{context}");
        }
    }
}

/// The 24 permutations of four indices, each with whether it is even.
fn permutations4() -> Vec<([usize; 4], bool)> {
    let mut out = Vec::new();
    for i in 0..4 {
        for j in 0..4 {
            for k in 0..4 {
                for l in 0..4 {
                    let p = [i, j, k, l];
                    if i != j && i != k && i != l && j != k && j != l && k != l {
                        let inversions = (0..4)
                            .flat_map(|x| (x + 1..4).map(move |y| (x, y)))
                            .filter(|&(x, y)| p[x] > p[y])
                            .count();
                        out.push((p, inversions % 2 == 0));
                    }
                }
            }
        }
    }
    out
}
