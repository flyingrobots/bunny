//! Seeded sweeps of the separating-axis kernels against the independent oracles
//! (`bunny-sat/1`, `docs/topics/convex-collision/`).
//!
//! Random shapes almost never touch, so the generators build contact deliberately: shapes
//! placed face to face, edge to edge and corner to corner on exact signed-permutation axes,
//! triangles laid on a box face or through a box edge, each also moved by one raw unit, next
//! to rotated shapes, degenerate shapes and shapes at every magnitude up to the ends of the
//! raw range. The outcome counts are pinned, so a generator that stops producing contacts
//! fails the test.

mod predicates_oracle;
mod sat_oracle;

use bunny_geom::{
    sat_obb_obb, sat_triangle_aabb, FixedAabb3, FixedObb3, FixedTriangle3, ObbAxis, SatClass,
    TriangleAabbAxis,
};
use bunny_linalg::{FixedMat3, FixedUnitVec3, FixedVec3};
use bunny_num::{FixedAngle, FixedQ32_32};
use sat_oracle::{
    aabb_corners, expected, hull, kind, obb_axes, obb_corners, triangle_aabb_axes, triangle_points,
};
use wasm_bindgen_test::wasm_bindgen_test;

const SEED_OBB: u64 = 0x0147_0001_0000_0001;
const SEED_TRIANGLE: u64 = 0x0147_0001_0000_0002;
const SMALL_OBB_CASES: usize = 400;
const LARGE_OBB_CASES: usize = 400;
const LARGE_OBB_HULL_EVERY: usize = 100;
const TRIANGLE_CASES: usize = 2_000;
const ONE: i64 = 1 << 32;
const EXTREMES: [i64; 8] = [i64::MIN, i64::MIN + 1, -2, -1, 0, 1, i64::MAX - 1, i64::MAX];

/// The `SplitMix64` generator.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(n).expect("n")).expect("index")
    }

    /// A value in `-bound..=bound`.
    fn within(&mut self, bound: i64) -> i64 {
        let span = u64::try_from(2 * bound + 1).expect("bound");
        i64::try_from(self.next() % span).expect("small") - bound
    }

    /// A raw value of any magnitude: a random `i64` shifted right by 0 to 63.
    fn spread(&mut self) -> i64 {
        self.next().cast_signed() >> (self.next() % 64)
    }

    fn extreme(&mut self) -> i64 {
        EXTREMES[self.below(8)]
    }
}

fn pt(v: [i64; 3]) -> FixedVec3 {
    FixedVec3::new(
        FixedQ32_32::from_raw(v[0]),
        FixedQ32_32::from_raw(v[1]),
        FixedQ32_32::from_raw(v[2]),
    )
}

/// A signed permutation matrix, exactly orthonormal, as columns `sign_i e_perm(i)`.
fn permutation(rng: &mut Rng) -> ([usize; 3], [i64; 3], FixedMat3) {
    let perms = [[0, 1, 2], [0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0]];
    let perm = perms[rng.below(6)];
    let signs = [(); 3].map(|()| if rng.next() & 1 == 0 { 1 } else { -1 });
    let mut rows = [[0_i64; 3]; 3];
    for (column, (&row, &sign)) in perm.iter().zip(&signs).enumerate() {
        rows[row][column] = sign * ONE;
    }
    (perm, signs, FixedMat3::from_rows(pt(rows[0]), pt(rows[1]), pt(rows[2])))
}

fn rotation(rng: &mut Rng) -> FixedMat3 {
    let axis = FixedUnitVec3::new(pt([(); 3].map(|()| rng.within(ONE))));
    FixedMat3::rotation_axis_angle(
        axis.unwrap_or(FixedUnitVec3::UNIT_Z),
        FixedAngle::from_raw(rng.next()),
    )
}

#[derive(Default, Debug)]
struct Counts {
    separated: usize,
    touching: usize,
    overlapping: usize,
    /// Results whose reported axis is a cross product of two edges.
    edge: usize,
}

impl Counts {
    fn add<A: Copy>(&mut self, class: &SatClass<A>, is_edge: impl Fn(A) -> bool) {
        self.edge += usize::from(class.axis().is_some_and(is_edge));
        match class {
            SatClass::Separated { .. } => self.separated += 1,
            SatClass::Touching { .. } => self.touching += 1,
            SatClass::Overlapping => self.overlapping += 1,
        }
    }
}

/// Checks one box pair against the corner projections, optionally the hull, and the swap.
fn check_boxes(a: &FixedObb3, b: &FixedObb3, with_hull: bool, context: &str) -> SatClass<ObbAxis> {
    let got = sat_obb_obb(a, b);
    let (ca, cb) = (obb_corners(a), obb_corners(b));
    assert_eq!(got, expected(&ca, &cb, &obb_axes(a, b)), "{context}: corner projections");
    if with_hull {
        assert_eq!(kind(&got), hull(&ca, &cb), "{context}: Minkowski hull");
    }
    let swapped = sat_obb_obb(b, a);
    assert_eq!(kind(&swapped), kind(&got), "{context}: swapped class");
    assert_eq!(swapped, expected(&cb, &ca, &obb_axes(b, a)), "{context}: swapped axis");
    got
}

const fn obb_edge(axis: ObbAxis) -> bool {
    matches!(axis, ObbAxis::EdgePair { .. })
}

#[wasm_bindgen_test(unsupported = test)]
fn sat_tp_011_seeded_box_pairs_match_the_oracles() {
    let mut rng = Rng(SEED_OBB);
    let mut small = Counts::default();
    for case in 0..SMALL_OBB_CASES {
        let context = format!("seed={SEED_OBB:#x} small case={case}");
        // Small boxes: centres within 8 raw units and half extents up to 8, so every
        // Minkowski difference point is below 2^39 and the hull runs in checked i128.
        let (a, b) = if case.is_multiple_of(2) {
            // Exact axes placed in contact: on each world axis the gap between the boxes is
            // zero (contact), or one raw unit either way, or the boxes simply overlap.
            let (pa, _, ma) = permutation(&mut rng);
            let (pb, _, mb) = permutation(&mut rng);
            let ha = [(); 3].map(|()| rng.within(8).abs());
            let hb = [(); 3].map(|()| rng.within(8).abs());
            let world = |perm: [usize; 3], h: [i64; 3]| {
                let mut e = [0; 3];
                for (column, &row) in perm.iter().enumerate() {
                    e[row] = h[column];
                }
                e
            };
            let (ea, eb) = (world(pa, ha), world(pb, hb));
            let ca = [(); 3].map(|()| rng.within(8));
            let cb = [0, 1, 2].map(|k| {
                let reach = ea[k] + eb[k];
                let offset = match rng.below(5) {
                    0 => reach,
                    1 => -reach,
                    2 => reach + rng.within(1),
                    _ => rng.within(reach),
                };
                ca[k] + offset
            });
            let a = FixedObb3::try_new(pt(ca), ma, pt(ha)).expect("valid");
            let b = FixedObb3::try_new(pt(cb), mb, pt(hb)).expect("valid");
            (a, b)
        } else {
            let ma = rotation(&mut rng);
            let mb = if case % 6 == 1 { ma } else { rotation(&mut rng) };
            let mk = |m: FixedMat3, rng: &mut Rng| {
                let c = [(); 3].map(|()| rng.within(8));
                let h = [(); 3].map(|()| rng.within(8).abs());
                FixedObb3::try_new(pt(c), m, pt(h)).expect("valid")
            };
            (mk(ma, &mut rng), mk(mb, &mut rng))
        };
        small.add(&check_boxes(&a, &b, true, &context), obb_edge);
    }
    let mut large = Counts::default();
    for case in 0..LARGE_OBB_CASES {
        let context = format!("seed={SEED_OBB:#x} large case={case}");
        let axes = |rng: &mut Rng| match rng.below(3) {
            0 => permutation(rng).2,
            _ => rotation(rng),
        };
        let (ma, mb) = (axes(&mut rng), axes(&mut rng));
        let mb = if case.is_multiple_of(4) { ma } else { mb };
        let (a, b) = if case.is_multiple_of(3) {
            // Face to face on exact columns: the second box is the first moved by m columns
            // a_0, with g_0 = m 2^32 - h_0 + delta, delta in -1..=1.
            let m = 1 + rng.within(3).abs();
            let a0 = ma.column0();
            let shift = [a0.x.raw(), a0.y.raw(), a0.z.raw()];
            let ca = [(); 3].map(|()| rng.within(1 << 40));
            let cb = [0, 1, 2].map(|i| ca[i] + m * shift[i]);
            let h0 = 1 + rng.within(m * ONE - 1).abs();
            let ha = [h0, rng.within(4 * ONE).abs(), rng.within(4 * ONE).abs()];
            let hb = [
                m * ONE - h0 + rng.within(1),
                rng.within(4 * ONE).abs(),
                rng.within(4 * ONE).abs(),
            ];
            let a = FixedObb3::try_new(pt(ca), ma, pt(ha)).expect("valid");
            let b = FixedObb3::try_new(pt(cb), ma, pt(hb.map(i64::abs))).expect("valid");
            (a, b)
        } else {
            let value = |rng: &mut Rng| match rng.below(3) {
                0 => rng.extreme(),
                1 => rng.spread(),
                _ => rng.within(4 * ONE),
            };
            let c = |rng: &mut Rng| [(); 3].map(|()| value(rng));
            let h = |rng: &mut Rng| [(); 3].map(|()| value(rng).checked_abs().unwrap_or(i64::MAX));
            let a = FixedObb3::try_new(pt(c(&mut rng)), ma, pt(h(&mut rng))).expect("valid");
            let b = FixedObb3::try_new(pt(c(&mut rng)), mb, pt(h(&mut rng))).expect("valid");
            (a, b)
        };
        large.add(
            &check_boxes(&a, &b, case.is_multiple_of(LARGE_OBB_HULL_EVERY), &context),
            obb_edge,
        );
    }
    println!("seed={SEED_OBB:#x} small={small:?} large={large:?}");
    assert!(
        small.separated >= 120
            && small.touching >= 120
            && small.overlapping >= 80
            && small.edge >= 3,
        "seed={SEED_OBB:#x}: {small:?}"
    );
    assert!(
        large.separated >= 200
            && large.touching >= 30
            && large.overlapping >= 60
            && large.edge >= 8,
        "seed={SEED_OBB:#x}: {large:?}"
    );
}

/// Checks one triangle and box against the corner projections and the hull, and checks that
/// rotating or reversing the vertices and inverting the box keep the class.
fn check_triangle(t: &FixedTriangle3, b: &FixedAabb3, context: &str) -> SatClass<TriangleAabbAxis> {
    let got = sat_triangle_aabb(t, b);
    let (ct, cb) = (triangle_points(t), aabb_corners(b));
    assert_eq!(got, expected(&ct, &cb, &triangle_aabb_axes(t)), "{context}: projections");
    let class = kind(&got);
    assert_eq!(class, hull(&ct, &cb), "{context}: Minkowski hull");
    let [v0, v1, v2] = t.vertices();
    for other in [FixedTriangle3::new(v1, v2, v0), FixedTriangle3::new(v0, v2, v1)] {
        let again = sat_triangle_aabb(&other, b);
        assert_eq!(kind(&again), class, "{context}: vertex order");
        let co = triangle_points(&other);
        assert_eq!(again, expected(&co, &cb, &triangle_aabb_axes(&other)), "{context}: order");
    }
    assert_eq!(sat_triangle_aabb(t, &FixedAabb3::new(b.max, b.min)), got, "{context}: inverted");
    got
}

#[wasm_bindgen_test(unsupported = test)]
fn sat_tp_012_seeded_triangles_and_boxes_match_the_oracles() {
    let mut rng = Rng(SEED_TRIANGLE);
    let mut counts = Counts::default();
    for case in 0..TRIANGLE_CASES {
        let context = format!("seed={SEED_TRIANGLE:#x} case={case}");
        let scale = if case % 10 == 9 { 0 } else { 1 << 20 };
        let value = |rng: &mut Rng| if scale == 0 { rng.extreme() } else { rng.within(scale) };
        let mut lo = [(); 3].map(|()| value(&mut rng));
        let mut hi = [(); 3].map(|()| value(&mut rng));
        if case % 7 == 3 {
            // A flat box.
            let k = rng.below(3);
            hi[k] = lo[k];
        }
        let mut v = [(); 3].map(|()| [(); 3].map(|()| value(&mut rng)));
        match case % 5 {
            0 => {
                // On a box face plane, or one raw unit off it.
                let k = rng.below(3);
                let plane = if rng.next() & 1 == 0 { lo[k].max(hi[k]) } else { lo[k].min(hi[k]) };
                let off = rng.within(1);
                for p in &mut v {
                    p[k] = plane.saturating_add(off);
                }
            }
            1 => {
                // Edge 0 through a box corner (exactly collinear in 3D), or one raw unit off.
                let corner = [0, 1, 2].map(|k| if rng.next() & 1 == 0 { lo[k] } else { hi[k] });
                let d = [(); 3].map(|()| rng.within(64));
                let (s, t) = (1 + rng.within(64).abs(), 1 + rng.within(64).abs());
                let off = [(); 3].map(|()| rng.within(1));
                v[0] = [0, 1, 2].map(|k| corner[k].saturating_add(s * d[k]).saturating_add(off[k]));
                v[1] = [0, 1, 2].map(|k| corner[k].saturating_sub(t * d[k]));
            }
            2 => {
                // A degenerate triangle: a repeated vertex or three collinear points.
                if rng.next() & 1 == 0 {
                    v[2] = v[rng.below(2)];
                } else {
                    v[2] = [0, 1, 2].map(|k| v[0][k] / 2 + v[1][k] / 2);
                    v[1] = [0, 1, 2].map(|k| v[2][k].saturating_mul(2).saturating_sub(v[0][k]));
                }
            }
            3 => {
                // Swap lo and hi on some axes: the inverted box is the same box.
                for k in 0..3 {
                    if rng.next() & 1 == 0 {
                        core::mem::swap(&mut lo[k], &mut hi[k]);
                    }
                }
            }
            _ => {}
        }
        let t = FixedTriangle3::new(pt(v[0]), pt(v[1]), pt(v[2]));
        let b = FixedAabb3::new(pt(lo), pt(hi));
        let class = check_triangle(&t, &b, &context);
        counts.add(&class, |axis| !matches!(axis, TriangleAabbAxis::BoxFace(_)));
    }
    println!("seed={SEED_TRIANGLE:#x} {counts:?}");
    assert!(
        counts.separated >= 1_000
            && counts.touching >= 90
            && counts.overlapping >= 600
            && counts.edge >= 100,
        "seed={SEED_TRIANGLE:#x}: {counts:?}"
    );
}
