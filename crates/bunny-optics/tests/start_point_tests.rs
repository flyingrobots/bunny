//! The start point of secondary rays in `bunny-diffuse-shading/1` (the self-intersection rule):
//! how many raw-unit steps it takes from `point_at(ray, hit.distance)`, and along which axis,
//! on hits the trace kernel reports.
//!
//! Every hit here is the `TriangleHit` that `hit_triangle` returned. The golden cases' expected
//! values were derived by hand with exact integers (the plane values are in the comments) and
//! checked by a separate exact Python model of the chapter's rule; the generated cases check
//! the rule as `docs/topics/optics/README.md#facing-and-secondary-rays` states it, with the exact
//! `i128` plane test written out here.

use std::hash::{BuildHasher, Hasher};

use bunny_linalg::FixedVec3;
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_optics::{point_at, SurfaceHit, TriangleSurface};
use bunny_query::{TraceRay, TraceTriangle, TriangleHit};
use wasm_bindgen_test::wasm_bindgen_test;

const ONE: i64 = ONE_RAW;
const HALF: i64 = ONE / 2;
const QUARTER: i64 = ONE / 4;
const MAX: FixedQ32_32 = FixedQ32_32::from_raw(i64::MAX);
const START_POINT_TEST_SEED: u64 = 0xB002_0288_5747_0001;

type Raw3 = [i64; 3];
type Wide3 = [i128; 3];

fn fixed(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

fn vec3(raw: Raw3) -> FixedVec3 {
    FixedVec3::new(fixed(raw[0]), fixed(raw[1]), fixed(raw[2]))
}

fn raw3(v: FixedVec3) -> Raw3 {
    [v.x.raw(), v.y.raw(), v.z.raw()]
}

fn triangle(v: [Raw3; 3]) -> Option<TraceTriangle> {
    TraceTriangle::new(vec3(v[0]), vec3(v[1]), vec3(v[2]))
}

/// The hit `ray.hit_triangle` reports on `vertices` and the `SurfaceHit` built from it.
fn traced(vertices: [Raw3; 3], ray: &TraceRay) -> (TriangleHit, SurfaceHit) {
    let triangle = triangle(vertices).expect("in the trace domain");
    let surface = TriangleSurface::new(&triangle).expect("not degenerate");
    let hit = ray.hit_triangle(&triangle, fixed(0), MAX).expect("the kernel reports a hit");
    (hit, SurfaceHit::new(&surface, ray, &hit).expect("a start point"))
}

/// The step from `point` to `start` as `(axis, signed raw units)`, `(0, 0)` for none; fails if
/// the two differ on more than one axis.
fn step(point: Raw3, start: Raw3) -> (usize, i64) {
    let moved: Vec<(usize, i64)> =
        (0..3).map(|i| (i, start[i] - point[i])).filter(|(_, d)| *d != 0).collect();
    assert!(moved.len() <= 1, "{point:?} -> {start:?} moves on more than one axis");
    moved.first().copied().unwrap_or((0, 0))
}

/// One golden hit: the ray, the kernel's distance and face, `point_at` and the start point.
struct Golden {
    name: &'static str,
    vertices: [Raw3; 3],
    origin: Raw3,
    direction: Raw3,
    distance: i64,
    front_face: bool,
    point: Raw3,
    start: Raw3,
}

fn check_golden(case: &Golden) -> (usize, i64) {
    let ray = TraceRay::new(vec3(case.origin), vec3(case.direction)).expect("a trace ray");
    let (hit, shading) = traced(case.vertices, &ray);
    let name = case.name;
    assert_eq!((hit.distance.raw(), hit.front_face), (case.distance, case.front_face), "{name}");
    let point = raw3(point_at(&ray, hit.distance).expect("representable"));
    assert_eq!(point, case.point, "{name}: point_at");
    assert_eq!(raw3(shading.point()), case.start, "{name}: start point");
    step(point, case.start)
}

/// The regular octahedron's face `(-1, 0, 0), (0, -1, 0), (0, 0, -1)`: plane normal
/// `(2^64, 2^64, 2^64)`, three equal components, the worst case of the two-step proof.
const OCTAHEDRON_FACE: [Raw3; 3] = [[-ONE, 0, 0], [0, -ONE, 0], [0, 0, -ONE]];

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_087_kernel_hits_take_zero_one_or_two_steps() {
    let cases = [
        // The octahedron witness: x + y + z of point_at is -(2^32 + 1), so the plane values
        // N . (p + k e) - N . v0 are -2^64, 0 and +2^64: the first step lands exactly on the
        // plane and the second is strictly in front. All three components tie: the lowest
        // axis, x, is stepped (the highest would give z = -949 370 707).
        Golden {
            name: "octahedron, two steps along +x",
            vertices: OCTAHEDRON_FACE,
            origin: [70_665_646, 517_454_629, 466_404_641_127],
            direction: [-22_947_436, -7_784_727, -3_651_203_219],
            distance: 549_755_813_657,
            front_face: true,
            point: [-2_866_606_161, -478_990_427, -949_370_709],
            start: [-2_866_606_159, -478_990_427, -949_370_709],
        },
        // A general triangle hit on its back face: the facing normal (the cross product
        // negated) is (4 009 118 584 266 148, 5 224 304 115 987 750, -5 598 972 802 970 042),
        // so the steps go along -z. Plane values -6 003 629 432 447 778,
        // -404 656 629 477 736 and +5 194 316 173 492 306: two steps, with no tie.
        Golden {
            name: "general triangle, two steps along -z",
            vertices: [
                [-118_361_732_970, 402_739_955_760, 359_001_064_095],
                [-118_285_892_404, 402_731_067_766, 359_047_076_149],
                [-118_377_805_607, 402_815_664_930, 359_060_198_267],
            ],
            origin: [67_557_292_915, 256_412, 7_010],
            direction: [-1_452_474_992, 3_146_459_434, 2_804_760_033],
            distance: 549_755_813_748,
            front_face: false,
            point: [-118_359_506_014, 402_747_063_861, 359_009_291_143],
            start: [-118_359_506_014, 402_747_063_861, 359_009_291_141],
        },
        // The ground y = 0 from 2 units above: point_at is (0, 0, 0), exactly on the plane
        // (value 0); one step up gives 2^72 > 0.
        Golden {
            name: "ground, one step along +y",
            vertices: [[-8 * ONE, 0, 8 * ONE], [8 * ONE, 0, 8 * ONE], [0, 0, -8 * ONE]],
            origin: [0, 2 * ONE, 0],
            direction: [0, -ONE, 0],
            distance: 2 * ONE,
            front_face: true,
            point: [0, 0, 0],
            start: [0, 1, 0],
        },
        // The plane x + y + z = 1 from the origin along (1, 1, 1), on its back face (facing
        // normal -(2^64, 2^64, 2^64)): the exact distance 2^32 / 3 raw is floored to
        // 1 431 655 765, short of the plane, and point_at is exact. Value
        // 2^96 - 3 * 1 431 655 765 * 2^64 = +2^64: strictly in front already, no step.
        Golden {
            name: "diagonal, no step",
            vertices: [[ONE, 0, 0], [0, ONE, 0], [0, 0, ONE]],
            origin: [0; 3],
            direction: [ONE; 3],
            distance: 1_431_655_765,
            front_face: false,
            point: [1_431_655_765; 3],
            start: [1_431_655_765; 3],
        },
    ];
    let steps = cases.iter().map(check_golden).collect::<Vec<_>>();
    assert_eq!(steps, [(0, 2), (2, -2), (1, 1), (0, 0)]);
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_088_a_tie_steps_along_the_lowest_axis() {
    // Each hit lands exactly on a plane whose largest normal components tie: the distance is
    // 5/4 units and the direction component along the ray is exactly 1.0, so point_at is the
    // exact point, its plane value is 0 and one step makes it positive. The axis stepped is
    // the lowest of the tied ones; the highest-axis rule would step the one named after "not".
    let cases = [
        // N = (2^65, 2^65, 0), plane x + y = 1: x and y tie; x, not y.
        Golden {
            name: "x = y",
            vertices: [[ONE, 0, -ONE], [0, ONE, -ONE], [ONE, 0, ONE]],
            origin: [2 * ONE, QUARTER, 0],
            direction: [-ONE, 0, 0],
            distance: 5 * QUARTER,
            front_face: true,
            point: [3 * QUARTER, QUARTER, 0],
            start: [3 * QUARTER + 1, QUARTER, 0],
        },
        // N = (0, 2^65, 2^65), plane y + z = 1: y, not z.
        Golden {
            name: "y = z",
            vertices: [[-ONE, ONE, 0], [-ONE, 0, ONE], [ONE, ONE, 0]],
            origin: [0, 2 * ONE, QUARTER],
            direction: [0, -ONE, 0],
            distance: 5 * QUARTER,
            front_face: true,
            point: [0, 3 * QUARTER, QUARTER],
            start: [0, 3 * QUARTER + 1, QUARTER],
        },
        // N = (2^65, 0, 2^65), plane x + z = 1: x, not z.
        Golden {
            name: "x = z",
            vertices: [[0, -ONE, ONE], [ONE, -ONE, 0], [0, ONE, ONE]],
            origin: [QUARTER, 0, 2 * ONE],
            direction: [0, 0, -ONE],
            distance: 5 * QUARTER,
            front_face: true,
            point: [QUARTER, 0, 3 * QUARTER],
            start: [QUARTER + 1, 0, 3 * QUARTER],
        },
        // N = (-2^65, 2^65, 0), plane -x + y = 1: magnitudes tie with opposite signs; the
        // step is along x with the sign of N_x, -1 (not +y).
        Golden {
            name: "|x| = |y|",
            vertices: [[-ONE, 0, -ONE], [-ONE, 0, ONE], [0, ONE, -ONE]],
            origin: [-2 * ONE, QUARTER, 0],
            direction: [ONE, 0, 0],
            distance: 5 * QUARTER,
            front_face: true,
            point: [-3 * QUARTER, QUARTER, 0],
            start: [-3 * QUARTER - 1, QUARTER, 0],
        },
        // N = (2^64, 2^64, 2^64), plane x + y + z = 1, from below (back face, facing normal
        // negated): the step is -x, not -z.
        Golden {
            name: "x = y = z",
            vertices: [[ONE, 0, 0], [0, ONE, 0], [0, 0, ONE]],
            origin: [HALF, QUARTER, -ONE],
            direction: [0, 0, ONE],
            distance: 5 * QUARTER,
            front_face: false,
            point: [HALF, QUARTER, QUARTER],
            start: [HALF - 1, QUARTER, QUARTER],
        },
    ];
    let steps = cases.iter().map(check_golden).collect::<Vec<_>>();
    assert_eq!(steps, [(0, 1), (1, 1), (0, 1), (0, -1), (0, -1)]);
}

/// `SplitMix64`: a seeded source of full 64-bit draws for the generated hits.
struct Seeds(u64);

impl Seeds {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A value in `low..=high`.
    fn range(&mut self, low: i64, high: i64) -> i64 {
        let span = i128::from(high) - i128::from(low) + 1;
        let offset = i128::from(self.next()) % span;
        i64::try_from(i128::from(low) + offset).expect("inside the range")
    }

    fn triple(&mut self, bound: i64) -> Raw3 {
        [self.range(-bound, bound), self.range(-bound, bound), self.range(-bound, bound)]
    }

    /// A face of a regular octahedron (radius from `2^24` to `2^38` raw, centre within `2^38`
    /// of the origin): its plane normal has three equal component magnitudes.
    fn octahedron_face(&mut self) -> [Raw3; 3] {
        let radius = self.range(1 << 24, 1 << 38);
        let centre = self.triple(1 << 38);
        let mut corner = |axis: usize| {
            let mut v = centre;
            v[axis] += if self.range(0, 1) == 0 { radius } else { -radius };
            v
        };
        [corner(0), corner(1), corner(2)]
    }

    /// A triangle at a scale from `2^8` to `2^36` raw, centre within `2^38` of the origin.
    fn general_triangle(&mut self) -> [Raw3; 3] {
        let bound = 1_i64 << self.range(8, 36);
        let centre = self.triple(1 << 38);
        let mut corner = || {
            let offset = self.triple(bound);
            [0, 1, 2].map(|i| centre[i] + offset[i])
        };
        [corner(), corner(), corner()]
    }

    /// A ray from a seeded origin toward a seeded point of `v` (weights in `2^-16` steps),
    /// with the largest direction component's magnitude from 0.5 to 1.0, as a camera ray's
    /// is; `None` if the two points coincide. (With a component of exactly 1.0 that product is
    /// exact, and two steps would need the other two products to round down by exactly half a
    /// raw unit each.)
    fn ray_at(&mut self, v: [Raw3; 3]) -> Option<TraceRay> {
        let (a, b) = (self.range(0, 1 << 16), self.range(0, 1 << 16));
        let (a, b) = if a + b > 1 << 16 { ((1 << 16) - a, (1 << 16) - b) } else { (a, b) };
        let target = [0, 1, 2].map(|i| {
            let along = |w: i64, corner: Raw3| i128::from(w) * i128::from(corner[i] - v[0][i]);
            i128::from(v[0][i]) + ((along(a, v[1]) + along(b, v[2])) >> 16)
        });
        let reach = 1_i64 << self.range(4, 38);
        let offset = self.triple(reach);
        let limit = (1_i128 << 40) - 1;
        let origin = [0, 1, 2].map(|i| (target[i] + i128::from(offset[i])).clamp(-limit, limit));
        let toward = [0, 1, 2].map(|i| target[i] - origin[i]);
        let largest = toward.iter().map(|c| c.abs()).max().filter(|m| *m > 0)?;
        let length = i128::from(self.range(HALF, ONE));
        let direction = toward.map(|c| c * length / largest);
        let narrow = |w: Wide3| w.map(|c| i64::try_from(c).expect("inside the trace domain"));
        TraceRay::new(vec3(narrow(origin)), vec3(narrow(direction)))
    }
}

fn wide(v: Raw3) -> Wide3 {
    v.map(i128::from)
}

fn dot(a: Wide3, b: Wide3) -> i128 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// The rule as the chapter states it: from `point`, the first of `point + k e` for `k` in
/// `0..=3` strictly on the arrival side of the exact plane, with `e` one raw unit along the
/// facing normal's largest component (the lowest axis on a tie), toward its sign. Returns `k`
/// and that point, or `None` if even three steps do not reach the arrival side.
fn rule(vertices: [Raw3; 3], front_face: bool, point: Raw3) -> Option<(usize, Raw3)> {
    let [v0, v1, v2] = vertices.map(wide);
    let (a, b) = ([0, 1, 2].map(|i| v1[i] - v0[i]), [0, 1, 2].map(|i| v2[i] - v0[i]));
    let cross = [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
    let normal = if front_face { cross } else { cross.map(|c| -c) };
    let offset = dot(normal, v0);
    let magnitudes = normal.map(i128::unsigned_abs);
    let largest = magnitudes.iter().max().copied().expect("three components");
    let axis = magnitudes.iter().position(|m| *m == largest).expect("the largest is present");
    let sign = i64::try_from(normal[axis].signum()).expect("a sign");
    (0..=3).find_map(|k| {
        let mut moved = point;
        moved[axis] += sign * i64::try_from(k).expect("small");
        (dot(normal, wide(moved)) > offset).then_some((k, moved))
    })
}

/// How many kernel-reported hits needed each number of steps, per population.
#[derive(Debug, Default)]
struct Steps {
    misses: usize,
    octahedron: [usize; 3],
    general: [usize; 3],
}

/// Traces `cases` seeded rays at octahedron faces and as many at general triangles, and checks
/// every reported hit's start point against [`rule`].
fn check_start_points(seed: u64, cases: usize) -> Steps {
    let mut seeds = Seeds(seed);
    let mut steps = Steps::default();
    for case in 0..2 * cases {
        let octahedron = case % 2 == 0;
        let vertices = if octahedron { seeds.octahedron_face() } else { seeds.general_triangle() };
        let Some(ray) = seeds.ray_at(vertices) else { continue };
        let Some(triangle) = triangle(vertices) else { continue };
        let Some(surface) = TriangleSurface::new(&triangle) else { continue };
        let Some(hit) = ray.hit_triangle(&triangle, fixed(0), MAX) else {
            steps.misses += 1;
            continue;
        };
        let context = format!("seed {seed:#x}, case {case}");
        let point = point_at(&ray, hit.distance)
            .map(raw3)
            .unwrap_or_else(|| panic!("{context}: point_at is not representable"));
        let shading = SurfaceHit::new(&surface, &ray, &hit);
        let (k, start) = rule(vertices, hit.front_face, point)
            .unwrap_or_else(|| panic!("{context}: three steps do not reach the arrival side"));
        assert!(k <= 2, "{context}: {k} steps needed at {point:?}");
        assert_eq!(shading.map(|s| raw3(s.point())), Some(start), "{context}");
        let counts = if octahedron { &mut steps.octahedron } else { &mut steps.general };
        counts[k] += 1;
    }
    steps
}

/// The witness counts every run must reach, so the generated hits cannot all be easy ones.
fn check_counts(seed: u64, steps: &Steps) {
    let found = format!("seed {seed:#x}: {steps:?}");
    println!("{found}");
    let [zero, one, two] = steps.octahedron;
    assert!(zero >= 10_000 && one >= 10_000 && two >= 1_000, "octahedron faces: {found}");
    let [zero, one, _] = steps.general;
    assert!(zero >= 10_000 && one >= 5_000, "general triangles: {found}");
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_089_generated_hits_follow_the_start_point_rule() {
    let steps = check_start_points(START_POINT_TEST_SEED, 50_000);
    check_counts(START_POINT_TEST_SEED, &steps);
}

/// The same check from a seed drawn per run, printed in every failure message; rerun a failure
/// with it as `START_POINT_TEST_SEED`. (On `wasm32-unknown-unknown` the standard library's hash
/// keys are fixed, so there the seed is the same every run.)
#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_090_generated_hits_follow_the_start_point_rule_from_a_fresh_seed() {
    // dojo: allow unordered-container -- RandomState only draws this per-run seed, printed on failure
    let seed = std::collections::hash_map::RandomState::new().build_hasher().finish();
    let steps = check_start_points(seed, 50_000);
    check_counts(seed, &steps);
}
