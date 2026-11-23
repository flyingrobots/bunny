//! Geometry-level triangles (`docs/topics/shape-library/test-plan.md`, SHAPE-TP-003, -004).
//!
//! The oracle is `shape_oracle::triangle_is_degenerate`: the full cross product formed in
//! 256-bit two's complement, sharing no code with the kernel's sign-and-magnitude comparison.

mod shape_oracle;

use bunny_geom::{FixedTriangle3, ShapeError};
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use shape_oracle::{triangle_is_degenerate, Rng};
use wasm_bindgen_test::wasm_bindgen_test;

const MIN: i64 = i64::MIN;
const MAX: i64 = i64::MAX;
const TRIANGLE_SEED: u64 = 0x5EED_0115_0000_0003;
const TRIANGLE_CASES: usize = 30_000;

fn pt(v: [i64; 3]) -> FixedVec3 {
    FixedVec3::new(
        FixedQ32_32::from_raw(v[0]),
        FixedQ32_32::from_raw(v[1]),
        FixedQ32_32::from_raw(v[2]),
    )
}

fn triangle(a: [i64; 3], b: [i64; 3], c: [i64; 3]) -> FixedTriangle3 {
    FixedTriangle3::new(pt(a), pt(b), pt(c))
}

/// The kernel agrees with the oracle on all six vertex orders, and `try_new` follows it.
fn check(a: [i64; 3], b: [i64; 3], c: [i64; 3], context: &str) -> bool {
    let expected = triangle_is_degenerate(a, b, c);
    for (p, q, r) in [(a, b, c), (b, c, a), (c, a, b), (b, a, c), (a, c, b), (c, b, a)] {
        assert_eq!(triangle(p, q, r).is_degenerate(), expected, "{context}: {p:?} {q:?} {r:?}");
    }
    let built = FixedTriangle3::try_new(pt(a), pt(b), pt(c));
    let wanted = if expected { Err(ShapeError::DegenerateTriangle) } else { Ok(triangle(a, b, c)) };
    assert_eq!(built, wanted, "{context}");
    expected
}

#[wasm_bindgen_test(unsupported = test)]
fn shape_tp_003_hand_built_degenerate_and_near_degenerate_triangles() {
    let one = 1_i64 << 32;
    // A proper triangle, and the vertex order kept as the winding.
    let (a, b, c) = ([0, 0, 0], [one, 0, 0], [0, one, 0]);
    assert!(!check(a, b, c, "unit right triangle"));
    let t = FixedTriangle3::try_new(pt(a), pt(b), pt(c)).expect("proper");
    assert_eq!(t.vertices(), [pt(a), pt(b), pt(c)]);
    assert_eq!((t.v0, t.v1, t.v2), (pt(a), pt(b), pt(c)));
    assert_ne!(t, triangle(b, a, c), "swapping two vertices reverses the winding");
    // new accepts a degenerate triangle; try_new refuses it.
    assert!(triangle(a, a, a).is_degenerate());
    assert_eq!(triangle(a, a, a), FixedTriangle3::new(pt(a), pt(a), pt(a)));
    // Coincident vertices, collinear vertices, opposite sides of v0, and zero products of
    // either sign (the zero-is-not-negative rule).
    assert!(check(a, a, a, "three coincident"));
    assert!(check(a, a, b, "two coincident"));
    assert!(check([0, 0, 0], [1, 2, 3], [2, 4, 6], "collinear"));
    assert!(check([0, 0, 0], [1, 0, 0], [-1, 0, 0], "collinear across v0"));
    assert!(check([0, 0, 0], [0, -5, 0], [0, 3, 0], "zero products of mixed sign"));
    // One raw unit off collinear is not degenerate, although the rounded Q32.32 cross product
    // of its edges is zero: the test is exact, not rounded.
    assert!(!check([0, 0, 0], [1, 2, 3], [2, 4, 7], "one raw unit off collinear"));
    assert!(!check([0, 0, 0], [1, 0, 0], [0, 1, 0], "one raw unit triangle"));
    assert_eq!(pt([1, 0, 0]).cross(pt([0, 1, 0])), pt([0, 0, 0]), "the rounded cross is zero");
    // The extremes of the raw range: edges of 2^64 - 1, products near 2^128.
    assert!(check([MIN; 3], [MAX; 3], [-1; 3], "diagonal of the whole range"));
    assert!(!check([MIN; 3], [MAX; 3], [-1, -1, 0], "one raw unit off the diagonal"));
    assert!(!check([MIN; 3], [MAX; 3], [-1, -1, -2], "one raw unit off the diagonal"));
    // Edges (2a, 2b) and (a, b) with a = 2^63 - 1, b = 2^63 - 3: equal products near 2^127.
    let (v0, v1) = ([MIN, MIN, 0], [MAX - 1, MAX - 5, 0]);
    assert!(check(v0, v1, [-1, -3, 0], "wide collinear"));
    for off in [[-1, -2, 0], [0, -3, 0], [-1, -4, 0], [-2, -3, 0], [-1, -3, 1], [-1, -3, -1]] {
        assert!(!check(v0, v1, off, "wide, one raw unit off"));
    }
    // Edges (2^64 - 1, 2^64 - 2) and (2^64 - 2, 2^64 - 3): the cross product is -1 at 2^128.
    assert!(!check([MIN, MIN, 0], [MAX, MAX - 1, 0], [MAX - 1, MAX - 2, 0], "cross of -1"));
    assert!(check([MIN, MIN, 0], [MAX, MAX, 0], [MAX - 1, MAX - 1, 0], "widest collinear"));
}

#[wasm_bindgen_test(unsupported = test)]
fn shape_tp_004_seeded_triangles_match_the_exact_oracle() {
    let mut rng = Rng(TRIANGLE_SEED);
    let (mut degenerate, mut near, mut proper) = (0, 0, 0);
    for case in 0..TRIANGLE_CASES {
        let context = format!("seed={TRIANGLE_SEED:#x} case={case}");
        match case % 3 {
            0 => {
                let [a, b, c] = [(); 3].map(|()| [(); 3].map(|()| rng.next_i64()));
                if !check(a, b, c, &context) {
                    proper += 1;
                }
            }
            1 => {
                let (a, b, c) = collinear(&mut rng);
                assert!(check(a, b, c, &context), "{context}: constructed collinear");
                degenerate += 1;
            }
            _ => {
                let (a, b, mut c) = collinear(&mut rng);
                let i = usize::try_from(rng.next_u64() % 3).expect("below 3");
                c[i] = if c[i] == MAX { c[i] - 1 } else { c[i] + 1 };
                if !check(a, b, c, &context) {
                    near += 1;
                }
            }
        }
    }
    let counts = format!("seed={TRIANGLE_SEED:#x}: {proper} {degenerate} {near}");
    assert!(proper >= 9_900 && degenerate == 10_000 && near >= 9_000, "{counts}");
}

/// `a`, `a + s d` and `a + t d`, with every coordinate below `2^63` in magnitude: `a` below
/// `2^62`, and `|s d|`, `|t d|` below `2^62`, at a random split of bits between `d` and `s`.
fn collinear(rng: &mut Rng) -> ([i64; 3], [i64; 3], [i64; 3]) {
    let bits = u32::try_from(rng.next_u64() % 61).expect("below 61");
    let d = [(); 3].map(|()| rng.within((1_i64 << bits) - 1));
    let scale = (1_i64 << (61 - bits)) - 1;
    let (s, t) = (rng.within(scale), rng.within(scale));
    let a = [(); 3].map(|()| rng.within((1_i64 << 62) - 1));
    let along = |k: i64| core::array::from_fn(|i| a[i] + k * d[i]);
    (a, along(s), along(t))
}
