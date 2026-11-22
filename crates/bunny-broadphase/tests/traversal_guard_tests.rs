//! The interior-visit bound of `intersect_aabb`, `intersect_ray`, `closest_hit` and `any_hit`
//! (flyingrobots/bunny#269): cyclic buffers and shared subtrees are typed errors after a
//! bounded number of steps, and trees keep every result and callback of the retained
//! functions in `support/traversal_reference.rs`. Run these under a wall-clock limit: without
//! the bound, the cycle cases do not terminate.

#[path = "support/traversal_reference.rs"]
mod reference;

use bunny_broadphase::utils::aabbs_overlap;
use bunny_broadphase::{
    any_hit, build_bvh, build_bvh_sah, closest_hit, compact_bvh, intersect_aabb, intersect_ray,
    BvhNode, BvhView, ClosestHit, CompactError, CompactNode, TraversalError,
};
use bunny_geom::{FixedAabb3, FixedRay3};
use bunny_linalg::FixedVec3;
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_query::{TraceRay, TraceTriangle};
use wasm_bindgen_test::wasm_bindgen_test;

const GUARD_SEED: u64 = 0xB002_0269_0000_0001;
const FAR: FixedQ32_32 = FixedQ32_32::from_raw(i64::MAX);
const REFUSED: Result<(), TraversalError> = Err(TraversalError::InvalidNodeIndex);

fn vec3(raw: [i64; 3]) -> FixedVec3 {
    let [x, y, z] = raw.map(FixedQ32_32::from_raw);
    FixedVec3::new(x, y, z)
}

fn cube(low: i64, high: i64) -> FixedAabb3 {
    FixedAabb3::new(vec3([low * ONE_RAW; 3]), vec3([high * ONE_RAW; 3]))
}

fn interior(first: u32) -> BvhNode {
    BvhNode { bounds: cube(0, 0), first_child_or_prim_idx: first, prim_count: 0 }
}

/// Interior nodes cover everything; leaves (`Some(primitive)`) cover `leaf_box`.
fn buffer(shape: &[Option<u32>], first: &[u32], leaf_box: FixedAabb3) -> Vec<BvhNode> {
    let mut firsts = first.iter();
    let node = |slot: &Option<u32>| match slot {
        Some(primitive) => {
            BvhNode { bounds: leaf_box, first_child_or_prim_idx: *primitive, prim_count: 1 }
        }
        None => BvhNode {
            bounds: cube(-16, 16),
            ..interior(*firsts.next().expect("a first child per interior node"))
        },
    };
    shape.iter().map(node).collect()
}

/// The cyclic buffers, each with its leaf callbacks for `intersect_ray` and `intersect_aabb`.
fn cyclic_buffers() -> [(&'static str, Vec<BvhNode>, usize); 4] {
    let (away, covering) = (cube(7, 8), cube(-16, 16));
    let leaf = Some(0);
    [
        // Node 0 is interior with first child 0: its children are itself and node 1.
        ("two nodes, leaf missed", buffer(&[None, leaf], &[0], away), 0),
        ("two nodes, leaf covering", buffer(&[None, leaf], &[0], covering), 1),
        // 0 -> 2 -> 4 -> 0, each through the left child, beside leaves 1, 3 and 5.
        ("three-node cycle", buffer(&[None, leaf, None, leaf, None, leaf], &[2, 4, 0], away), 0),
        // Node 2's children are leaf 1 and node 2 itself.
        ("own right child", buffer(&[None, leaf, None], &[1, 1], away), 0),
    ]
}

fn trace_ray() -> TraceRay {
    TraceRay::new(vec3([0, 0, 5 * ONE_RAW]), vec3([0, 0, -ONE_RAW])).expect("in domain")
}

fn calls_of<R>(run: impl FnOnce(&mut Vec<u32>) -> R) -> (R, Vec<u32>) {
    let mut calls = Vec::new();
    let result = run(&mut calls);
    (result, calls)
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_401_closest_hit_refuses_cyclic_buffers() {
    let ray = trace_ray();
    for (name, nodes, _) in cyclic_buffers() {
        let (found, calls) = calls_of(|calls| {
            closest_hit(BvhView::new(&nodes, &[0]), &ray, FAR, |p, _| {
                calls.push(p);
                None
            })
        });
        assert_eq!((found, calls), (Err(TraversalError::InvalidNodeIndex), vec![]), "{name}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_402_any_hit_refuses_cyclic_buffers() {
    let ray = trace_ray();
    for (name, nodes, _) in cyclic_buffers() {
        let (found, calls) = calls_of(|calls| {
            any_hit(BvhView::new(&nodes, &[0]), &ray, FAR, |p| {
                calls.push(p);
                false
            })
        });
        assert_eq!((found, calls), (Err(TraversalError::InvalidNodeIndex), vec![]), "{name}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_403_intersect_ray_refuses_cyclic_buffers() {
    let direction = vec3([0, 0, -ONE_RAW]);
    let ray = FixedRay3::try_new(vec3([0, 0, 5 * ONE_RAW]), direction).expect("unit direction");
    for (name, nodes, callbacks) in cyclic_buffers() {
        let found = calls_of(|calls| intersect_ray(&nodes, &[0], &ray, |p| calls.push(p)));
        assert_eq!(found, (REFUSED, vec![0; callbacks]), "{name}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_404_intersect_aabb_refuses_cyclic_buffers() {
    let query = cube(-1, 1);
    for (name, nodes, callbacks) in cyclic_buffers() {
        let found = calls_of(|calls| intersect_aabb(&nodes, &[0], &query, |p| calls.push(p)));
        assert_eq!(found, (REFUSED, vec![0; callbacks]), "{name}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_405_shared_subtrees_are_searched_within_the_budget_and_refused_beyond_it() {
    let all = cube(-16, 16);
    let (l0, l1) = (Some(0), Some(1));
    // Node 2 is the right child of node 0 and the left child of node 1: two interior visits.
    let shared_leaf = buffer(&[None, None, l0, l1], &[1, 2], all);
    // Nodes 1 and 2 both have children 3 and 4: three interior visits in five nodes.
    let shared_interior = buffer(&[None, None, None, l0, l1], &[1, 3, 3], all);
    // Node i has children i + 1 and i + 2: about 1.6^38 interior visits unfolded.
    let mut ladder_shape = vec![None; 38];
    ladder_shape.extend([l0, l1]);
    let ladder = buffer(&ladder_shape, &(1..=38).collect::<Vec<_>>(), all);
    let aabb =
        |nodes: &[BvhNode]| calls_of(|c| intersect_aabb(nodes, &[0, 1], &all, |p| c.push(p)));
    let retained = |nodes: &[BvhNode]| {
        calls_of(|c| reference::traverse(nodes, &[0, 1], |_| true, |p| c.push(p)))
    };
    assert_eq!(aabb(&shared_leaf), (Ok(()), vec![0, 1, 0]), "searched as the unfolded tree");
    assert_eq!(aabb(&shared_leaf), retained(&shared_leaf));
    assert_eq!(aabb(&shared_interior), (REFUSED, vec![1, 0]));
    assert_eq!(retained(&shared_interior), (Ok(()), vec![1, 0, 1, 0]), "terminates unguarded");
    let (ladder_result, ladder_calls) = aabb(&ladder);
    assert_eq!((ladder_result, ladder_calls.len() < ladder.len()), (REFUSED, true));
    let (found, calls) = calls_of(|c| {
        closest_hit(BvhView::new(&shared_interior, &[0, 1]), &trace_ray(), FAR, |p, _| {
            c.push(p);
            None
        })
    });
    assert_eq!((found, calls), (Err(TraversalError::InvalidNodeIndex), vec![0, 1]));
    // The compact conversion refuses exactly the buffers the searches refuse.
    let mut out = [CompactNode { bounds: [all; 2], children: Default::default() }; 64];
    assert!(matches!(compact_bvh(&shared_leaf, &mut out), Ok(Some((_, 2)))));
    assert_eq!(compact_bvh(&shared_interior, &mut out), Err(CompactError::NotATree));
    assert_eq!(compact_bvh(&ladder, &mut out), Err(CompactError::NotATree));
}

struct Seeds(u64);

impl Seeds {
    fn next(&mut self) -> u64 {
        self.0 =
            self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    fn below(&mut self, bound: usize) -> usize {
        let bound = u64::try_from(bound).expect("fits u64");
        usize::try_from(self.next() % bound).expect("below a usize bound")
    }

    fn point(&mut self, bound: i64) -> FixedVec3 {
        let span = u64::try_from(2 * bound + 1).expect("bound is positive");
        let mut signed = || i64::try_from(self.next() % span).expect("below the span") - bound;
        vec3([signed(), signed(), signed()])
    }
}

struct Case<'a> {
    nodes: &'a [BvhNode],
    prim_indices: &'a [u32],
    triangles: &'a [TraceTriangle],
}

/// What one function returned and every callback it made, in order (`closest_hit` records
/// the limit it passed; the others record zero).
type Calls<R> = (Result<R, TraversalError>, Vec<(u32, i64)>);

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    closest: Calls<Option<ClosestHit>>,
    any: Calls<bool>,
    line: Calls<()>,
    aabb: Calls<()>,
}

struct Query {
    ray: TraceRay,
    /// `FAR` for half the queries, otherwise up to 15 units.
    t_max: FixedQ32_32,
    line: FixedRay3,
    aabb: FixedAabb3,
}

impl Case<'_> {
    fn distance(&self, ray: &TraceRay, primitive: u32, limit: FixedQ32_32) -> Option<FixedQ32_32> {
        let triangle = self.triangles.get(primitive as usize)?;
        ray.hit_triangle(triangle, FixedQ32_32::ZERO, limit).map(|hit| hit.distance)
    }

    /// Every function on one query: the library's (`guarded`) or the retained copies.
    fn run(&self, query: &Query, guarded: bool) -> Outcome {
        let (bvh, ray) = (BvhView::new(self.nodes, self.prim_indices), &query.ray);
        let (nodes, prims) = (self.nodes, self.prim_indices);
        let mut calls = Vec::new();
        let mut closest = |p, limit: FixedQ32_32| {
            calls.push((p, limit.raw()));
            self.distance(ray, p, limit)
        };
        let found = if guarded {
            closest_hit(bvh, ray, query.t_max, &mut closest)
        } else {
            reference::search(bvh, (ray, query.t_max), false, &mut closest)
        };
        let closest = (found, std::mem::take(&mut calls));
        let mut any = |p, _| {
            calls.push((p, 0));
            self.distance(ray, p, query.t_max).map(|_| FixedQ32_32::ZERO)
        };
        let found = if guarded {
            any_hit(bvh, ray, query.t_max, |p| any(p, FAR).is_some())
        } else {
            reference::search(bvh, (ray, query.t_max), true, &mut any).map(|hit| hit.is_some())
        };
        let any = (found, std::mem::take(&mut calls));
        let line = &query.line;
        let mut visit = |p| calls.push((p, 0));
        let found = if guarded {
            intersect_ray(nodes, prims, line, &mut visit)
        } else {
            let crosses = |n: &BvhNode| bunny_query::ray_intersects_aabb(line, &n.bounds).is_some();
            reference::traverse(nodes, prims, crosses, &mut visit)
        };
        let line = (found, std::mem::take(&mut calls));
        let mut visit = |p| calls.push((p, 0));
        let found = if guarded {
            intersect_aabb(nodes, prims, &query.aabb, &mut visit)
        } else {
            let overlaps = |n: &BvhNode| aabbs_overlap(&n.bounds, &query.aabb);
            reference::traverse(nodes, prims, overlaps, &mut visit)
        };
        Outcome { closest, any, line, aabb: (found, calls) }
    }
}

/// A ray from anywhere near the scene toward a point inside it, as both ray types, and a
/// box somewhere in the scene.
fn seeded_query(seeds: &mut Seeds) -> Option<Query> {
    let origin = seeds.point(8 * ONE_RAW);
    let toward = seeds.point(6 * ONE_RAW) - origin;
    // At most 14 units per axis; a sixteenth keeps trace components within 1.0.
    let direction = vec3([toward.x.raw() / 16, toward.y.raw() / 16, toward.z.raw() / 16]);
    let ray = TraceRay::new(origin, direction)?;
    let line = FixedRay3::try_new(origin, direction).ok()?;
    let corner = seeds.point(7 * ONE_RAW);
    let extent = seeds.point(3 * ONE_RAW);
    let size = vec3([extent.x.raw().abs(), extent.y.raw().abs(), extent.z.raw().abs()]);
    let short = FixedQ32_32::from_raw(size.x.raw() * 5);
    let t_max = if seeds.next().is_multiple_of(2) { FAR } else { short };
    Some(Query { ray, t_max, line, aabb: FixedAabb3::new(corner, corner + size) })
}

/// Seeded triangles in a 12-unit cube, built by `builder` and truncated to the used nodes.
fn scene(
    seeds: &mut Seeds,
    count: usize,
    sah: bool,
) -> (Vec<TraceTriangle>, Vec<BvhNode>, Vec<u32>) {
    let triangles: Vec<TraceTriangle> = (0..count)
        .filter_map(|_| {
            let v0 = seeds.point(6 * ONE_RAW);
            TraceTriangle::new(v0, v0 + seeds.point(ONE_RAW), v0 + seeds.point(ONE_RAW))
        })
        .collect();
    let boxes: Vec<FixedAabb3> = triangles.iter().map(TraceTriangle::bounds).collect();
    let mut nodes = vec![interior(0); 2 * boxes.len()];
    let mut prims = vec![0_u32; boxes.len()];
    let builder = if sah { build_bvh_sah } else { build_bvh };
    let used = builder(&mut nodes, &mut prims, &boxes).expect("buffers fit");
    nodes.truncate(used);
    (triangles, nodes, prims)
}

/// The tree unchanged, then with one seeded node's child index or leaf range made out of
/// range: still trees, so the guard must not fire, and the retained functions' errors must
/// be kept.
fn variants(seeds: &mut Seeds, nodes: &[BvhNode], prims: usize) -> Vec<Vec<BvhNode>> {
    let past = |node: &BvhNode| if node.prim_count > 0 { prims } else { nodes.len() };
    let breaks: [fn(&mut BvhNode, usize); 3] = [
        |node, past| node.first_child_or_prim_idx = u32::try_from(past).expect("small"),
        |node, _| node.first_child_or_prim_idx = u32::MAX,
        |node, _| node.prim_count = u32::MAX,
    ];
    let mut out = vec![nodes.to_vec()];
    for broken in breaks {
        let mut tree = nodes.to_vec();
        let node = &mut tree[seeds.below(nodes.len())];
        let past = past(node);
        broken(node, past);
        out.push(tree);
    }
    out
}

/// A comb of `levels` interior nodes, each keeping a leaf beside the next level. Complete,
/// 70 levels make a valid tree deeper than the 64-entry stack, which `closest_hit` searches
/// completely (70 interior visits, the whole budget) and the box queries refuse with
/// `StackOverflow`. Truncated, 64 levels end in a node whose children are out of range,
/// reached when the box queries' stack is full: they must still say `StackOverflow`.
fn comb(levels: u32, complete: bool) -> Vec<BvhNode> {
    let count = 2 * levels + 1;
    let shape: Vec<Option<u32>> =
        (0..count).map(|i| (i % 2 == 1 || i + 1 == count).then_some(0)).collect();
    let mut nodes =
        buffer(&shape, &(0..levels).map(|level| 2 * level + 1).collect::<Vec<_>>(), cube(-16, 16));
    if !complete {
        nodes.truncate(nodes.len() - 2);
    }
    nodes
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_406_valid_trees_keep_every_result_and_callback() {
    let mut seeds = Seeds(GUARD_SEED);
    let mut compared = 0;
    for (count, sah) in [1, 2, 3, 5, 17, 64, 300].into_iter().flat_map(|n| [(n, false), (n, true)])
    {
        let (triangles, nodes, prims) = scene(&mut seeds, count, sah);
        let mut trees = variants(&mut seeds, &nodes, prims.len());
        trees.extend([comb(70, true), comb(64, false)]);
        for (tree, nodes) in trees.iter().enumerate() {
            let case = Case { nodes, prim_indices: &prims, triangles: &triangles };
            for ray_index in 0..60 {
                let Some(query) = seeded_query(&mut seeds) else { continue };
                let label = format!(
                    "seed={GUARD_SEED:#018x} count={count} sah={sah} tree={tree} ray={ray_index}"
                );
                assert_eq!(case.run(&query, true), case.run(&query, false), "{label}");
                compared += 1;
            }
        }
    }
    assert!(compared > 2_500, "corpus too small: {compared}");
}
