//! `TraceRay::hit_triangle` against an independent signed-volume oracle
//! (`support/volume_oracle.rs`), on cases concentrated on and next to edges and vertices
//! (TRACE-TP-315).
//!
//! Before this test, relaxing all three edge comparisons by one numerator unit failed only
//! the frozen copy of the kernel (flyingrobots/bunny#294). The cases here are built so that
//! the numerators the kernel compares are exactly zero, or one, two or three units either
//! side of zero, on each edge in turn. Each family:
//!
//! - small: raw-unit triangles and directions, origins solved so one edge numerator takes a
//!   chosen value in `-3..=3`, rays through a vertex, a raw unit from a vertex, or anywhere;
//!   half of them moved next to the corner of the trace domain (coordinates near `2^40`),
//!   which leaves every numerator unchanged;
//! - scaled: the same, scaled by about `2^35` with the direction scaled by `2^28`, so the
//!   numerators reach about `2^107`;
//! - long: direction components near `2^32` (grazing when they nearly cancel against the
//!   normal), rays through a vertex, an edge midpoint, or two raw units from either, half of
//!   them near the corner of the trace domain;
//! - degenerate: one in twenty cases in every family has collinear or repeated vertices.
//!
//! The seed is fixed; every failure message names it and the case index.

#[path = "support/volume_oracle.rs"]
mod volume_oracle;

use bunny_query::{TRACE_COORD_LIMIT_RAW, TRACE_DIRECTION_LIMIT_RAW};
use volume_oracle::{assert_kernel_agrees, Verdict};
use wasm_bindgen_test::wasm_bindgen_test;

const EDGE_ORACLE_SEED: u64 = 0xB002_7ACE_0294_0001;
const EDGE_ORACLE_CASES: usize = 40_000;

type Raw3 = [i64; 3];

struct Seeds(u64);

impl Seeds {
    fn next(&mut self) -> u64 {
        self.0 =
            self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    fn below(&mut self, bound: u64) -> usize {
        usize::try_from(self.next() % bound).expect("small bound")
    }

    /// A value in `-bound..=bound`.
    fn signed(&mut self, bound: i64) -> i64 {
        let span = u64::try_from(2 * bound + 1).expect("bound is positive");
        i64::try_from(self.next() % span).expect("below the span") - bound
    }

    fn triple(&mut self, bound: i64) -> Raw3 {
        [self.signed(bound), self.signed(bound), self.signed(bound)]
    }

    fn sign(&mut self) -> i64 {
        if self.next() & 1 == 1 {
            1
        } else {
            -1
        }
    }
}

fn add(a: Raw3, b: Raw3) -> Raw3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: Raw3, b: Raw3) -> Raw3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(a: Raw3, k: i64) -> Raw3 {
    a.map(|c| c * k)
}

fn midpoint(a: Raw3, b: Raw3) -> Raw3 {
    [0, 1, 2].map(|i| (a[i] + b[i]).div_euclid(2))
}

fn wide_cross(a: Raw3, b: Raw3) -> [i128; 3] {
    let (a, b) = (a.map(i128::from), b.map(i128::from));
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

/// The signed volume `(a - origin) x (b - origin) . direction` of edge `(a, b)`, small
/// enough for `i128` with raw-unit geometry.
fn small_volume(origin: Raw3, direction: Raw3, (a, b): (Raw3, Raw3)) -> i128 {
    let normal = wide_cross(sub(a, origin), sub(b, origin));
    (0..3).map(|i| normal[i] * i128::from(direction[i])).sum()
}

/// The edges in the order of the kernel's numerators: `u` (`v2 v0`), `v` (`v0 v1`) and
/// `det - u - v` (`v1 v2`).
const fn edges(v: [Raw3; 3]) -> [(Raw3, Raw3); 3] {
    [(v[2], v[0]), (v[0], v[1]), (v[1], v[2])]
}

/// Moves one coordinate of `origin` so that the oriented numerator of edge `target.0` is
/// exactly `target.1`, when an integer move of at most 12 units does it. The volume of an
/// edge is linear in the origin with gradient `(b - a) x d`.
fn solve_origin(origin: Raw3, d: Raw3, v: [Raw3; 3], target: (usize, i64)) -> Option<Raw3> {
    let (edge, value) = target;
    let (tail, head) = edges(v)[edge];
    let gradient = wide_cross(sub(head, tail), d);
    let volumes = edges(v).map(|pair| small_volume(origin, d, pair));
    let sigma = volumes.iter().sum::<i128>().signum();
    // The first axis with the smallest nonzero gradient.
    let axis = (0..3).filter(|&j| gradient[j] != 0).reduce(|best, j| {
        if gradient[j].abs() < gradient[best].abs() {
            j
        } else {
            best
        }
    })?;
    let delta = i128::from(value) * sigma - volumes[edge];
    if sigma == 0 || delta % gradient[axis] != 0 {
        return None;
    }
    let mut moved = origin;
    moved[axis] += i64::try_from(delta / gradient[axis]).ok()?;
    (moved[axis].abs() <= 12).then_some(moved)
}

/// One generated case: origin, direction, vertices, and whether it is in an extreme family.
struct Case {
    origin: Raw3,
    direction: Raw3,
    vertices: [Raw3; 3],
    extreme: bool,
    degenerate: bool,
}

fn small_origin(seeds: &mut Seeds, d: Raw3, v: [Raw3; 3]) -> Raw3 {
    let route = seeds.below(8);
    if route < 5 {
        let (edge, target) = (seeds.below(3), seeds.signed(3));
        let (tail, head) = edges(v)[edge];
        let mut start = [0; 3];
        for _ in 0..8 {
            start =
                sub(add(midpoint(tail, head), seeds.triple(1)), scale(d, 1 + seeds.sign().max(0)));
            if let Some(origin) = solve_origin(start, d, v, (edge, target)) {
                return origin;
            }
        }
        return start;
    }
    if route < 7 {
        let through =
            sub(v[seeds.below(3)], scale(d, i64::try_from(seeds.below(3)).expect("small")));
        return if route == 6 { add(through, seeds.triple(1)) } else { through };
    }
    seeds.triple(8)
}

fn collapse(seeds: &mut Seeds, v: &mut [Raw3; 3]) {
    let m = seeds.signed(2);
    v[2] = add(v[0], scale(sub(v[1], v[0]), m));
}

fn generate(seeds: &mut Seeds, index: usize) -> Case {
    let family = index % 10;
    let degenerate = seeds.below(20) == 0;
    let corner = |seeds: &mut Seeds, margin: u32| {
        [0; 3].map(|_| seeds.sign() * (TRACE_COORD_LIMIT_RAW - (1 << margin)))
    };
    if family < 6 {
        let mut v = [seeds.triple(4), seeds.triple(4), seeds.triple(4)];
        let mut d = seeds.triple(4);
        if d == [0; 3] {
            d = [0, 0, 1];
        }
        if degenerate {
            collapse(seeds, &mut v);
        }
        let mut origin = small_origin(seeds, d, v);
        let mut shift = [0; 3];
        if family >= 4 {
            shift = corner(seeds, 10);
        }
        if family == 5 {
            let k = (1 << 35) + seeds.signed(1 << 20);
            v = v.map(|p| scale(p, k));
            origin = scale(origin, k);
            d = scale(d, 1 << 28);
            shift = [0; 3];
        }
        let vertices = v.map(|p| add(p, shift));
        return Case {
            origin: add(origin, shift),
            direction: d,
            vertices,
            extreme: family >= 4,
            degenerate,
        };
    }
    let v0 = seeds.triple(8);
    let mut v = [v0, add(v0, scale(seeds.triple(3), 2)), add(v0, scale(seeds.triple(3), 2))];
    if degenerate {
        collapse(seeds, &mut v);
    }
    let unit = TRACE_DIRECTION_LIMIT_RAW;
    let mut d =
        [0; 3].map(|_| seeds.sign() * (unit - i64::try_from(seeds.below(9)).expect("small")));
    if seeds.below(4) == 0 {
        let axis = seeds.below(3);
        d[axis] = seeds.signed(4);
    }
    let (route, steps, corner_index) = (seeds.below(4), seeds.below(4), seeds.below(3));
    let edge_mid = midpoint(v[corner_index], v[(corner_index + 1) % 3]);
    let through = match route {
        0 => v[corner_index],
        1 => edge_mid,
        2 => add(v[corner_index], seeds.triple(2)),
        _ => add(edge_mid, seeds.triple(2)),
    };
    let origin = sub(through, scale(d, i64::try_from(steps).expect("small")));
    let shift = if family >= 8 { corner(seeds, 35) } else { [0; 3] };
    Case {
        origin: add(origin, shift),
        direction: d,
        vertices: v.map(|p| add(p, shift)),
        extreme: true,
        degenerate,
    }
}

/// How many cases of each kind the corpus reached.
#[derive(Debug, Default)]
struct Reached {
    hits: usize,
    /// Hits with exactly one of `u`, `v`, `det - u - v` zero, per numerator.
    edge_hits: [usize; 3],
    vertex_hits: usize,
    /// Misses ahead of the origin with one numerator `-1` or `-2` and the others `>= 0`.
    near_misses: [usize; 3],
    extreme_hits: usize,
    extreme_edge_or_vertex_hits: usize,
    grazing_hits: usize,
    front_hits: usize,
    back_hits: usize,
    degenerate: usize,
    parallel: usize,
}

impl Reached {
    fn count(&mut self, case: &Case, verdict: &Verdict) {
        self.degenerate += usize::from(case.degenerate);
        let Some(weights) = verdict.weights else {
            self.parallel += usize::from(!case.degenerate);
            return;
        };
        let zeros = weights.map(|w| w == 0);
        let Some(hit) = verdict.hit else {
            let outside = weights.iter().filter(|&&w| w < 0).count();
            if verdict.ahead && outside == 1 {
                let edge = (0..3).find(|&e| weights[e] < 0).expect("one is negative");
                self.near_misses[edge] += usize::from(weights[edge] >= -2);
            }
            return;
        };
        self.hits += 1;
        let on_edges = zeros.iter().filter(|&&z| z).count();
        if on_edges == 1 {
            self.edge_hits[(0..3).find(|&e| zeros[e]).expect("one is zero")] += 1;
        }
        self.vertex_hits += usize::from(on_edges >= 2);
        self.extreme_hits += usize::from(case.extreme);
        self.extreme_edge_or_vertex_hits += usize::from(case.extreme && on_edges >= 1);
        let [v0, v1, v2] = case.vertices;
        let normal = wide_cross(sub(v1, v0), sub(v2, v0)).map(i128::abs).into_iter().max();
        let reach = case.direction.map(|c| i128::from(c.abs())).into_iter().max();
        let grazing = hit.det * 64 <= reach.unwrap_or(0) * normal.unwrap_or(0);
        self.grazing_hits += usize::from(grazing);
        self.front_hits += usize::from(hit.front_face);
        self.back_hits += usize::from(!hit.front_face);
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_315_hits_near_edges_and_vertices_match_a_signed_volume_oracle() {
    let mut seeds = Seeds(EDGE_ORACLE_SEED);
    let mut reached = Reached::default();
    for index in 0..EDGE_ORACLE_CASES {
        let case = generate(&mut seeds, index);
        let context = format!("seed={EDGE_ORACLE_SEED:#018x} case={index}");
        let verdict = assert_kernel_agrees(case.origin, case.direction, case.vertices, &context);
        reached.count(&case, &verdict);
    }
    // The corpus must keep reaching the edges: each floor is about 75 % of what the fixed
    // seed reaches (the counts are in the test plan).
    let r = &reached;
    let floors = [
        (r.hits, 11_500, "hits"),
        (r.edge_hits[0], 1_150, "hits on the v2-v0 edge (u = 0)"),
        (r.edge_hits[1], 1_150, "hits on the v0-v1 edge (v = 0)"),
        (r.edge_hits[2], 1_150, "hits on the v1-v2 edge (u + v = det)"),
        (r.vertex_hits, 5_200, "hits on a vertex"),
        (r.near_misses[0], 150, "misses one or two units past u = 0"),
        (r.near_misses[1], 150, "misses one or two units past v = 0"),
        (r.near_misses[2], 150, "misses one or two units past u + v = det"),
        (r.extreme_hits, 8_200, "hits in the scaled, cornered and long families"),
        (r.extreme_edge_or_vertex_hits, 6_700, "edge or vertex hits in those families"),
        (r.grazing_hits, 480, "grazing hits (det * 64 <= |d| |n|)"),
        (r.front_hits, 5_800, "front-face hits"),
        (r.back_hits, 5_800, "back-face hits"),
        (r.degenerate, 1_500, "degenerate triangles"),
        (r.parallel, 680, "parallel rays"),
    ];
    for (count, floor, what) in floors {
        assert!(count >= floor, "{what}: {count} < {floor}; seed={EDGE_ORACLE_SEED:#018x} {r:?}");
    }
}
