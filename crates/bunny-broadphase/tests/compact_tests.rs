//! The compact layout (`compact_bvh`, `compact_closest_hit`, `compact_any_hit`) against
//! `closest_hit` and `any_hit` on the source tree and against a test of every primitive.

use bunny_broadphase::utils::union_aabb;
use bunny_broadphase::{
    any_hit, build_bvh, closest_hit, compact_any_hit, compact_bvh, compact_closest_hit, BvhNode,
    BvhView, ClosestHit, CompactChild, CompactError, CompactNode, CompactRoot, CompactView,
    TraversalError,
};
use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_query::{TraceRay, TraceTriangle};
use std::cell::Cell;
use wasm_bindgen_test::wasm_bindgen_test;

const CLOSEST_SEED: u64 = 0xB002_C0C7_0000_0001;
const FAR: FixedQ32_32 = FixedQ32_32::from_raw(i64::MAX);

struct Seeds(u64);

impl Seeds {
    fn next(&mut self) -> u64 {
        self.0 =
            self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    fn signed(&mut self, bound: i64) -> i64 {
        let span = u64::try_from(2 * bound + 1).expect("bound is positive");
        i64::try_from(self.next() % span).expect("below the span") - bound
    }

    fn point(&mut self, bound: i64) -> FixedVec3 {
        vec3([self.signed(bound), self.signed(bound), self.signed(bound)])
    }
}

fn vec3(raw: [i64; 3]) -> FixedVec3 {
    FixedVec3::new(
        FixedQ32_32::from_raw(raw[0]),
        FixedQ32_32::from_raw(raw[1]),
        FixedQ32_32::from_raw(raw[2]),
    )
}

struct Scene {
    triangles: Vec<TraceTriangle>,
    nodes: Vec<BvhNode>,
    prim_indices: Vec<u32>,
}

impl Scene {
    fn new(triangles: Vec<TraceTriangle>) -> Self {
        let boxes: Vec<FixedAabb3> = triangles.iter().map(TraceTriangle::bounds).collect();
        let empty = BvhNode {
            bounds: FixedAabb3::new(vec3([0; 3]), vec3([0; 3])),
            first_child_or_prim_idx: 0,
            prim_count: 0,
        };
        let mut nodes = vec![empty; 2 * boxes.len()];
        let mut prim_indices = vec![0_u32; boxes.len()];
        let used = build_bvh(&mut nodes, &mut prim_indices, &boxes).expect("buffers fit");
        nodes.truncate(used);
        Self { triangles, nodes, prim_indices }
    }

    /// The same triangles under a tree built from them in the order `order` gives, with leaf
    /// entries mapped back, so primitive indices are unchanged but the tree's shape differs.
    fn reshaped(&self, order: &[usize]) -> Self {
        let permuted = order.iter().map(|&index| self.triangles[index]).collect();
        let Self { nodes, prim_indices, .. } = Self::new(permuted);
        let prim_indices = prim_indices
            .iter()
            .map(|&slot| u32::try_from(order[slot as usize]).expect("small scene"))
            .collect();
        Self { triangles: self.triangles.clone(), nodes, prim_indices }
    }

    /// The same triangles under a single leaf that holds them all, in reverse index order.
    fn one_leaf(&self) -> Self {
        let count = u32::try_from(self.triangles.len()).expect("small scene");
        let root =
            BvhNode { bounds: self.nodes[0].bounds, first_child_or_prim_idx: 0, prim_count: count };
        let prim_indices = (0..count).rev().collect();
        Self { triangles: self.triangles.clone(), nodes: vec![root], prim_indices }
    }

    fn bvh(&self) -> BvhView<'_> {
        BvhView::new(&self.nodes, &self.prim_indices)
    }

    /// Small triangles scattered through a cube, plus a few that span it.
    fn seeded(seeds: &mut Seeds, count: usize) -> Self {
        let triangles = (0..count)
            .filter_map(|index| {
                let reach = if index % 40 == 0 { 6 * ONE_RAW } else { ONE_RAW };
                let v0 = seeds.point(6 * ONE_RAW);
                TraceTriangle::new(v0, v0 + seeds.point(reach), v0 + seeds.point(reach))
            })
            .collect();
        Self::new(triangles)
    }

    fn distance(&self, ray: &TraceRay, primitive: u32, limit: FixedQ32_32) -> Option<FixedQ32_32> {
        let triangle = self.triangles.get(usize::try_from(primitive).ok()?)?;
        ray.hit_triangle(triangle, FixedQ32_32::ZERO, limit).map(|hit| hit.distance)
    }

    /// The oracle: test every triangle; nearest wins, then the lowest index.
    fn brute_force(&self, ray: &TraceRay, t_max: FixedQ32_32) -> Option<ClosestHit> {
        (0..self.triangles.len())
            .filter_map(|index| {
                let primitive = u32::try_from(index).ok()?;
                let distance = self.distance(ray, primitive, t_max)?;
                Some(ClosestHit { primitive, distance })
            })
            .min_by_key(|hit| (hit.distance, hit.primitive))
    }
}

/// A ray from anywhere near the scene toward a point inside it. One component in five is
/// zero or a few raw units, so axis-parallel and nearly axis-parallel rays are covered.
fn seeded_ray(seeds: &mut Seeds) -> Option<TraceRay> {
    let origin = seeds.point(8 * ONE_RAW);
    let toward = seeds.point(6 * ONE_RAW) - origin;
    // The two points are at most 14 units apart per axis; a sixteenth keeps components in 1.0.
    let mut direction = [toward.x.raw() / 16, toward.y.raw() / 16, toward.z.raw() / 16];
    for component in &mut direction {
        match seeds.next() % 10 {
            0 => *component = 0,
            1 => *component = seeds.signed(2),
            _ => {}
        }
    }
    TraceRay::new(origin, vec3(direction))
}

/// Flat triangles at the given heights, each under the point (0, 0), and a ray falling on
/// them from z = 5 with direction exactly -1.0, so box entry bounds and distances are exact.
fn stack_of(heights: &[i64]) -> (Vec<TraceTriangle>, TraceRay) {
    let triangles = heights
        .iter()
        .map(|&z| {
            let at = |x: i64, y: i64| vec3([x * ONE_RAW, y * ONE_RAW, z * ONE_RAW]);
            TraceTriangle::new(at(-1, -1), at(1, -1), at(0, 1)).expect("in domain")
        })
        .collect();
    let ray = TraceRay::new(vec3([0, 0, 5 * ONE_RAW]), vec3([0, 0, -ONE_RAW])).expect("in domain");
    (triangles, ray)
}

/// A root over two leaves; `leaves` lists each leaf's primitive indices.
fn two_leaves(triangles: &[TraceTriangle], leaves: [&[u32]; 2]) -> Scene {
    let everything = FixedAabb3::new(vec3([-8 * ONE_RAW; 3]), vec3([8 * ONE_RAW; 3]));
    let leaf = |first: usize, primitives: &[u32]| {
        let bounds = primitives
            .iter()
            .map(|&p| triangles[p as usize].bounds())
            .reduce(union_aabb)
            .expect("leaves are not empty");
        let count = u32::try_from(primitives.len()).expect("small");
        BvhNode {
            bounds,
            first_child_or_prim_idx: u32::try_from(first).expect("small"),
            prim_count: count,
        }
    };
    let root = BvhNode { bounds: everything, first_child_or_prim_idx: 1, prim_count: 0 };
    let nodes = vec![root, leaf(0, leaves[0]), leaf(leaves[0].len(), leaves[1])];
    let prim_indices = leaves.concat();
    Scene { triangles: triangles.to_vec(), nodes, prim_indices }
}

/// A compact copy of a scene's tree.
struct Compact {
    root: Option<CompactRoot>,
    nodes: Vec<CompactNode>,
}

const EMPTY_NODE: CompactNode = CompactNode {
    bounds: [FixedAabb3::new(FixedVec3::new(Q0, Q0, Q0), FixedVec3::new(Q0, Q0, Q0)); 2],
    children: [CompactChild { first: 0, prim_count: 0 }; 2],
};
const Q0: FixedQ32_32 = FixedQ32_32::ZERO;

impl Compact {
    fn of(nodes: &[BvhNode]) -> Result<Self, CompactError> {
        let mut out = vec![EMPTY_NODE; nodes.len() / 2];
        let converted = compact_bvh(nodes, &mut out)?;
        let (root, written) = converted.map_or((None, 0), |(root, written)| (Some(root), written));
        out.truncate(written);
        Ok(Self { root, nodes: out })
    }

    fn view<'a>(&'a self, scene: &'a Scene) -> CompactView<'a> {
        CompactView { root: self.root, nodes: &self.nodes, prim_indices: &scene.prim_indices }
    }
}

type Traced = (Result<Option<ClosestHit>, TraversalError>, Vec<(u32, FixedQ32_32)>);
type TracedAny = (Result<bool, TraversalError>, Vec<u32>);

/// `closest_hit` on the source tree and `compact_closest_hit` on `compact`, with the calls
/// each made to the primitive test.
fn closest_both(scene: &Scene, compact: &Compact, ray: &TraceRay, t: FixedQ32_32) -> [Traced; 2] {
    let mut calls = [Vec::new(), Vec::new()];
    let [source_calls, compact_calls] = &mut calls;
    let source = closest_hit(scene.bvh(), ray, t, |primitive, limit| {
        source_calls.push((primitive, limit));
        scene.distance(ray, primitive, limit)
    });
    let found = compact_closest_hit(compact.view(scene), ray, t, |primitive, limit| {
        compact_calls.push((primitive, limit));
        scene.distance(ray, primitive, limit)
    });
    let [source_calls, compact_calls] = calls;
    [(source, source_calls), (found, compact_calls)]
}

fn any_both(scene: &Scene, compact: &Compact, ray: &TraceRay, t: FixedQ32_32) -> [TracedAny; 2] {
    let mut calls = [Vec::new(), Vec::new()];
    let [source_calls, compact_calls] = &mut calls;
    let source = any_hit(scene.bvh(), ray, t, |primitive| {
        source_calls.push(primitive);
        scene.distance(ray, primitive, t).is_some()
    });
    let found = compact_any_hit(compact.view(scene), ray, t, |primitive| {
        compact_calls.push(primitive);
        scene.distance(ray, primitive, t).is_some()
    });
    let [source_calls, compact_calls] = calls;
    [(source, source_calls), (found, compact_calls)]
}

/// Asserts that the compact searches make the source searches' calls and give their answers,
/// and that those equal the test of every primitive. Returns that answer.
fn agree(scene: &Scene, compact: &Compact, ray: &TraceRay, t: FixedQ32_32) -> Option<ClosestHit> {
    let expected = scene.brute_force(ray, t);
    let [source, found] = closest_both(scene, compact, ray, t);
    assert_eq!(source.0, Ok(expected), "closest_hit against every primitive");
    assert_eq!(found, source, "compact_closest_hit against closest_hit");
    let [source, found] = any_both(scene, compact, ray, t);
    assert_eq!(source.0, Ok(expected.is_some()), "any_hit against every primitive");
    assert_eq!(found, source, "compact_any_hit against any_hit");
    expected
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_301_compact_searches_equal_the_source_tree_and_every_primitive() {
    let mut seeds = Seeds(CLOSEST_SEED);
    for (count, rays) in [(2, 200), (3, 200), (5, 300), (17, 300), (64, 500), (600, 2000)] {
        let scene = Scene::seeded(&mut seeds, count);
        let compact = Compact::of(&scene.nodes).expect("a built tree converts");
        let (mut hits, mut traced) = (0, 0);
        for _ in 0..rays {
            let Some(ray) = seeded_ray(&mut seeds) else { continue };
            let found = agree(&scene, &compact, &ray, FAR);
            hits += usize::from(found.is_some());
            traced += 1;
        }
        assert!(hits > 0 && hits < traced, "count={count}: {hits} hits of {traced} rays");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_302_compact_searches_agree_on_ties_limits_and_other_tree_shapes() {
    let mut seeds = Seeds(CLOSEST_SEED ^ 2);
    let mut triangles = Scene::seeded(&mut seeds, 150).triangles;
    // Duplicates make ties; a third of the triangles appear twice.
    let copies: Vec<TraceTriangle> = triangles.iter().step_by(3).copied().collect();
    triangles.extend(copies);
    let scene = Scene::new(triangles);
    let count = scene.triangles.len();
    let reversed: Vec<usize> = (0..count).rev().collect();
    let interleaved: Vec<usize> = (0..count).step_by(2).chain((1..count).step_by(2)).collect();
    let shapes = [scene.reshaped(&reversed), scene.reshaped(&interleaved), scene.one_leaf()];
    let mut limited = 0;
    for _ in 0..800 {
        let Some(ray) = seeded_ray(&mut seeds) else { continue };
        for shape in shapes.iter().chain([&scene]) {
            let compact = Compact::of(&shape.nodes).expect("converts");
            let Some(nearest) = agree(shape, &compact, &ray, FAR) else { continue };
            agree(shape, &compact, &ray, nearest.distance);
            agree(shape, &compact, &ray, FixedQ32_32::from_raw(nearest.distance.raw() - 1));
            limited += 1;
        }
    }
    assert!(limited > 200, "corpus needs hits: {limited}");
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_303_empty_single_primitive_and_single_leaf_trees() {
    let ray = TraceRay::new(vec3([0, 0, 5 * ONE_RAW]), vec3([0, 0, -ONE_RAW])).expect("in domain");
    let empty = Compact::of(&[]).expect("an empty tree converts");
    assert!(empty.root.is_none() && empty.nodes.is_empty());
    let view = CompactView { root: None, nodes: &[], prim_indices: &[] };
    assert_eq!(compact_closest_hit(view, &ray, FAR, |_, _| Some(Q0)), Ok(None));
    assert_eq!(compact_any_hit(view, &ray, FAR, |_| true), Ok(false));

    let (triangles, ray) = stack_of(&[0, -1, -2]);
    let single = Scene::new(triangles[..1].to_vec());
    let compact = Compact::of(&single.nodes).expect("converts");
    let leaf = CompactChild { first: 0, prim_count: 1 };
    assert_eq!(compact.root, Some(CompactRoot { bounds: single.nodes[0].bounds, child: leaf }));
    assert!(compact.nodes.is_empty(), "a one-leaf tree has no compact node");
    assert!(agree(&single, &compact, &ray, FAR).is_some());

    let one_leaf = Scene::new(triangles).one_leaf();
    let compact = Compact::of(&one_leaf.nodes).expect("converts");
    assert_eq!(compact.root.map(|root| root.child), Some(CompactChild { first: 0, prim_count: 3 }));
    assert_eq!(agree(&one_leaf, &compact, &ray, FAR).map(|hit| hit.primitive), Some(0));
}

fn everything() -> FixedAabb3 {
    FixedAabb3::new(vec3([-8 * ONE_RAW; 3]), vec3([8 * ONE_RAW; 3]))
}

const fn interior(first: u32) -> BvhNode {
    let bounds = FixedAabb3::new(FixedVec3::new(Q0, Q0, Q0), FixedVec3::new(Q0, Q0, Q0));
    BvhNode { bounds, first_child_or_prim_idx: first, prim_count: 0 }
}

const fn leaf(first: u32) -> BvhNode {
    let bounds = FixedAabb3::new(FixedVec3::new(Q0, Q0, Q0), FixedVec3::new(Q0, Q0, Q0));
    BvhNode { bounds, first_child_or_prim_idx: first, prim_count: 1 }
}

/// A tree in which each of `levels` nodes on the left spine has an interior right sibling,
/// so a depth-first walk holds one pending right subtree per level.
fn comb(levels: u32) -> Vec<BvhNode> {
    let mut nodes = vec![interior(1)];
    for level in 0..levels {
        let at = u32::try_from(nodes.len()).expect("small");
        // Spine node `at`, its sibling `at + 1`, the sibling's two leaves at the end.
        let spine = if level + 1 == levels { leaf(0) } else { interior(at + 4) };
        nodes.extend([spine, interior(at + 2), leaf(0), leaf(0)]);
    }
    nodes
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_304_malformed_input_is_a_typed_error() {
    let dangling = [interior(7)];
    assert_eq!(Compact::of(&dangling).err(), Some(CompactError::InvalidNodeIndex));
    let past_end = [interior(1), leaf(0), interior(u32::MAX)];
    assert_eq!(Compact::of(&past_end).err(), Some(CompactError::InvalidNodeIndex));
    let mut out = [EMPTY_NODE; 1];
    let two_interior = [interior(1), interior(3), leaf(0), leaf(0), leaf(0)];
    assert_eq!(compact_bvh(&two_interior, &mut out), Err(CompactError::OutputTooSmall));
    assert_eq!(
        compact_bvh(&two_interior, &mut [EMPTY_NODE; 2]).map(|r| r.map(|r| r.1)),
        Ok(Some(2))
    );
    let cycle = [interior(1), interior(0), leaf(0)];
    assert_eq!(Compact::of(&cycle).err(), Some(CompactError::NotATree));
    assert!(Compact::of(&comb(60)).is_ok());
    assert_eq!(Compact::of(&comb(70)).err(), Some(CompactError::TooDeep));
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_305_malformed_compact_trees_are_traversal_errors() {
    let ray = TraceRay::new(vec3([0, 0, 5 * ONE_RAW]), vec3([0, 0, -ONE_RAW])).expect("in domain");
    let miss = |_: u32, _: FixedQ32_32| None;
    let root = |child| Some(CompactRoot { bounds: everything(), child });
    let to_node = CompactView {
        root: root(CompactChild { first: 0, prim_count: 0 }),
        nodes: &[],
        prim_indices: &[],
    };
    assert_eq!(
        compact_closest_hit(to_node, &ray, FAR, miss),
        Err(TraversalError::InvalidNodeIndex)
    );
    let short = CompactView {
        root: root(CompactChild { first: 0, prim_count: 3 }),
        nodes: &[],
        prim_indices: &[0],
    };
    assert_eq!(
        compact_closest_hit(short, &ray, FAR, miss),
        Err(TraversalError::InvalidPrimitiveRange)
    );
    // Each node keeps a leaf underneath and descends into the next: the stack grows per level.
    let chain: Vec<CompactNode> = (1..=70)
        .map(|next| CompactNode {
            bounds: [everything(); 2],
            children: [
                CompactChild { first: next, prim_count: 0 },
                CompactChild { first: 0, prim_count: 1 },
            ],
        })
        .collect();
    let deep =
        CompactView { root: root(CompactChild::default()), nodes: &chain, prim_indices: &[0] };
    assert_eq!(compact_closest_hit(deep, &ray, FAR, miss), Err(TraversalError::StackOverflow));
    // A node whose left child is itself and whose right box the ray misses: each visit
    // pushes only itself, so without a visit bound the search would never end.
    let away = FixedAabb3::new(vec3([7 * ONE_RAW; 3]), vec3([8 * ONE_RAW; 3]));
    let cycle = [CompactNode {
        bounds: [everything(), away],
        children: [CompactChild::default(), CompactChild { first: 0, prim_count: 1 }],
    }];
    let looping =
        CompactView { root: root(CompactChild::default()), nodes: &cycle, prim_indices: &[0] };
    assert_eq!(
        compact_closest_hit(looping, &ray, FAR, miss),
        Err(TraversalError::InvalidNodeIndex)
    );
    assert_eq!(
        compact_any_hit(looping, &ray, FAR, |_| false),
        Err(TraversalError::InvalidNodeIndex)
    );
}

/// `compact_closest_hit` on the scene's compact tree, with the primitives it tested.
fn traced(scene: &Scene, ray: &TraceRay) -> (Option<ClosestHit>, Vec<u32>) {
    let compact = Compact::of(&scene.nodes).expect("converts");
    let mut tested = Vec::new();
    let found = compact_closest_hit(compact.view(scene), ray, FAR, |primitive, limit| {
        tested.push(primitive);
        scene.distance(ray, primitive, limit)
    });
    (found.expect("well formed"), tested)
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_306_the_compact_search_keeps_the_order_pruning_and_early_stop() {
    let five = FixedQ32_32::from_raw(5 * ONE_RAW);
    // Primitive 0 is 8.0 away in the left leaf, primitive 1 is 5.0 away in the right one.
    let (triangles, ray) = stack_of(&[-3, 0]);
    let near_right = two_leaves(&triangles, [&[0], &[1]]);
    let only_near = (Some(ClosestHit { primitive: 1, distance: five }), vec![1]);
    assert_eq!(traced(&near_right, &ray), only_near, "nearer first, the farther one pruned");
    // The ray enters the nearer leaf's box (5.0 away) but misses its triangle; the hit is in
    // the farther leaf (8.0 away), so the farther child must be kept, not dropped.
    let at = |x: i64, y: i64, z: i64| vec3([x * ONE_RAW, y * ONE_RAW, z * ONE_RAW]);
    let near_miss =
        TraceTriangle::new(at(-1, -1, 0), at(1, -1, 0), at(0, 1, 0)).expect("in domain");
    let far_hit =
        TraceTriangle::new(at(-4, -4, -3), at(4, -4, -3), at(0, 4, -3)).expect("in domain");
    let corner =
        TraceRay::new(vec3([9 * ONE_RAW / 10, 9 * ONE_RAW / 10, 5 * ONE_RAW]), at(0, 0, -1));
    let corner = corner.expect("in domain");
    let behind = two_leaves(&[near_miss, far_hit], [&[0], &[1]]);
    let eight = FixedQ32_32::from_raw(8 * ONE_RAW);
    assert_eq!(
        traced(&behind, &corner),
        (Some(ClosestHit { primitive: 1, distance: eight }), vec![0, 1]),
        "the farther child is searched when the nearer one has no hit"
    );
    // Coincident triangles: both leaves start exactly at the hit distance 5.0; the left
    // one, searched first on the tie, holds the higher index.
    let (triangles, ray) = stack_of(&[0, 0]);
    let tie = two_leaves(&triangles, [&[1], &[0]]);
    let lowest = (Some(ClosestHit { primitive: 0, distance: five }), vec![1, 0]);
    assert_eq!(traced(&tie, &ray), lowest, "a subtree starting at the best distance is searched");
    // Every primitive is hit, so one test settles any-hit; below every box, none runs.
    let (triangles, ray) = stack_of(&[0, -1, -2, -3]);
    let scene = two_leaves(&triangles, [&[0, 1], &[2, 3]]);
    let compact = Compact::of(&scene.nodes).expect("converts");
    for (limit, expected) in
        [(FAR, (Ok(true), 1)), (FixedQ32_32::from_raw(4 * ONE_RAW), (Ok(false), 0))]
    {
        let calls = Cell::new(0);
        let found = compact_any_hit(compact.view(&scene), &ray, limit, |primitive| {
            calls.set(calls.get() + 1);
            scene.distance(&ray, primitive, limit).is_some()
        });
        assert_eq!((found, calls.get()), expected, "limit={}", limit.raw());
    }
}

fn boxed(n: i64) -> FixedAabb3 {
    FixedAabb3::new(vec3([n * ONE_RAW; 3]), vec3([(n + 1) * ONE_RAW; 3]))
}

fn fnv(hash: u64, value: u64) -> u64 {
    value.to_le_bytes().iter().fold(hash, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3))
}

fn checksum(compact: &Compact) -> u64 {
    let raws = |b: &FixedAabb3| {
        [b.min.x, b.min.y, b.min.z, b.max.x, b.max.y, b.max.z].map(FixedQ32_32::raw)
    };
    let mut hash = 0xcbf2_9ce4_8422_2325;
    for node in &compact.nodes {
        for (bounds, child) in node.bounds.iter().zip(node.children) {
            for raw in raws(bounds) {
                hash = fnv(hash, raw.cast_unsigned());
            }
            hash = fnv(fnv(hash, u64::from(child.first)), u64::from(child.prim_count));
        }
    }
    hash
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_307_compact_conversion_golden_vectors() {
    // Root 0 over 1 and 2; node 1 over 3 (interior, over leaves 5 and 6) and leaf 4; node 2
    // over leaves 7 and 8. Depth first, left subtree first: 0, 1, 3, 2.
    let with = |mut node: BvhNode, n: i64| {
        node.bounds = boxed(n);
        node
    };
    let source = [
        with(interior(1), 0),
        with(interior(3), 1),
        with(interior(7), 2),
        with(interior(5), 3),
        with(BvhNode { prim_count: 1, ..leaf(2) }, 4),
        with(leaf(0), 5),
        with(leaf(1), 6),
        with(BvhNode { prim_count: 2, ..leaf(3) }, 7),
        with(leaf(5), 8),
    ];
    let compact = Compact::of(&source).expect("converts");
    let node = |bounds: [i64; 2], children: [(u32, u32); 2]| CompactNode {
        bounds: bounds.map(boxed),
        children: children.map(|(first, prim_count)| CompactChild { first, prim_count }),
    };
    assert_eq!(
        compact.root,
        Some(CompactRoot { bounds: boxed(0), child: CompactChild::default() })
    );
    assert_eq!(
        compact.nodes,
        [
            node([1, 2], [(1, 0), (3, 0)]),
            node([3, 4], [(2, 0), (2, 1)]),
            node([5, 6], [(0, 1), (1, 1)]),
            node([7, 8], [(3, 2), (5, 1)]),
        ]
    );
    // The bytes of a seeded scene's conversion, locked on every target.
    let scene = Scene::seeded(&mut Seeds(CLOSEST_SEED ^ 7), 300);
    let seeded = Compact::of(&scene.nodes).expect("converts");
    assert_eq!(seeded.nodes.len(), scene.nodes.len() / 2);
    assert_eq!(checksum(&seeded), checksum(&Compact::of(&scene.nodes).expect("converts")));
    assert_eq!(checksum(&seeded), 0x09a7_ad04_d4b0_833e, "golden checksum");
}
