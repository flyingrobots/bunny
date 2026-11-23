//! `closest_point` against a test of every primitive, on trees of many shapes.

use std::cmp::Ordering;

use bunny_broadphase::utils::union_aabb;
use bunny_broadphase::{
    build_bvh, build_bvh_sah, closest_point, BvhNode, BvhView, ClosestPoint, TraversalError,
};
use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_query::{PointQuery, TraceTriangle};
use wasm_bindgen_test::wasm_bindgen_test;

const POINT_TREE_SEED: u64 = 0xB002_0246_0000_0002;

struct Seeds(u64);

impl Seeds {
    fn next(&mut self) -> u64 {
        self.0 =
            self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(n).expect("small")).expect("small")
    }

    fn signed(&mut self, bound: i64) -> i64 {
        let span = u64::try_from(2 * bound + 1).expect("bound is positive");
        i64::try_from(self.next() % span).expect("below the span") - bound
    }

    fn raw3(&mut self, bound: i64) -> [i64; 3] {
        [self.signed(bound), self.signed(bound), self.signed(bound)]
    }

    fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            items.swap(i, self.below(i + 1));
        }
    }
}

fn vec3(raw: [i64; 3]) -> FixedVec3 {
    FixedVec3::new(
        FixedQ32_32::from_raw(raw[0]),
        FixedQ32_32::from_raw(raw[1]),
        FixedQ32_32::from_raw(raw[2]),
    )
}

fn triangle(t: [[i64; 3]; 3]) -> TraceTriangle {
    TraceTriangle::new(vec3(t[0]), vec3(t[1]), vec3(t[2])).expect("in the trace domain")
}

fn query(p: [i64; 3]) -> PointQuery {
    PointQuery::new(vec3(p)).expect("in the trace domain")
}

fn empty_node() -> BvhNode {
    BvhNode {
        bounds: FixedAabb3::new(vec3([0; 3]), vec3([0; 3])),
        first_child_or_prim_idx: 0,
        prim_count: 0,
    }
}

struct Tree {
    nodes: Vec<BvhNode>,
    prim_indices: Vec<u32>,
}

impl Tree {
    fn view(&self) -> BvhView<'_> {
        BvhView::new(&self.nodes, &self.prim_indices)
    }

    /// A builder's tree over the triangles taken in `order`, with leaf entries mapped back to
    /// the original indices, so the shape changes and the primitive indices do not.
    fn built(triangles: &[TraceTriangle], order: &[usize], sah: bool) -> Self {
        let boxes: Vec<FixedAabb3> = order.iter().map(|&i| triangles[i].bounds()).collect();
        let mut nodes = vec![empty_node(); 2 * boxes.len()];
        let mut slots = vec![0_u32; boxes.len()];
        let builder = if sah { build_bvh_sah } else { build_bvh };
        let used = builder(&mut nodes, &mut slots, &boxes).expect("buffers fit");
        nodes.truncate(used);
        let prim_indices =
            slots.iter().map(|&slot| u32::try_from(order[slot as usize]).expect("small")).collect();
        Self { nodes, prim_indices }
    }

    /// A tree split at random points, with leaves of one to `max_leaf` primitives in the
    /// order `order` gives, and exact unions as node bounds.
    fn random(
        triangles: &[TraceTriangle],
        order: &[usize],
        seeds: &mut Seeds,
        max_leaf: usize,
    ) -> Self {
        let mut tree = Self { nodes: vec![empty_node()], prim_indices: Vec::new() };
        tree.fill(0, order, &mut Split { triangles, seeds, max_leaf });
        tree
    }

    fn fill(&mut self, at: usize, order: &[usize], split_with: &mut Split<'_>) {
        let Split { triangles, ref mut seeds, max_leaf } = *split_with;
        let bounds =
            order.iter().map(|&i| triangles[i].bounds()).reduce(union_aabb).expect("not empty");
        if order.len() <= 1 + seeds.below(max_leaf) {
            let first = u32::try_from(self.prim_indices.len()).expect("small");
            self.prim_indices.extend(order.iter().map(|&i| u32::try_from(i).expect("small")));
            let count = u32::try_from(order.len()).expect("small");
            self.nodes[at] = BvhNode { bounds, first_child_or_prim_idx: first, prim_count: count };
            return;
        }
        let left = self.nodes.len();
        self.nodes.extend([empty_node(), empty_node()]);
        let first = u32::try_from(left).expect("small");
        self.nodes[at] = BvhNode { bounds, first_child_or_prim_idx: first, prim_count: 0 };
        let split = 1 + seeds.below(order.len() - 1);
        self.fill(left, &order[..split], split_with);
        self.fill(left + 1, &order[split..], split_with);
    }
}

/// What a random tree is split with.
struct Split<'a> {
    triangles: &'a [TraceTriangle],
    seeds: &'a mut Seeds,
    max_leaf: usize,
}

fn search(tree: &Tree, triangles: &[TraceTriangle], p: &PointQuery) -> Option<ClosestPoint> {
    closest_point(tree.view(), p, |primitive| {
        triangles.get(primitive as usize).map(|t| p.closest_on_triangle(t))
    })
    .expect("the tree is well formed")
}

/// The oracle of the search: every triangle tested, the exactly nearest kept, then the lowest
/// index. It shares the per-triangle kernel and none of the tree logic.
fn brute_force(triangles: &[TraceTriangle], p: &PointQuery) -> Option<ClosestPoint> {
    let mut best: Option<(u32, bunny_query::TrianglePoint)> = None;
    for (primitive, t) in (0_u32..).zip(triangles) {
        let found = p.closest_on_triangle(t);
        if best.is_none_or(|(_, b)| found.cmp_distance(&b) == Ordering::Less) {
            best = Some((primitive, found));
        }
    }
    best.map(|(primitive, found)| ClosestPoint {
        primitive,
        point: found.point(),
        distance_squared: found.distance_squared(),
    })
}

/// A height field of `n` by `n` quads, two triangles each, with integer heights in raw units
/// times `scale`: every interior edge and vertex is shared, so exact ties are common.
fn grid(seeds: &mut Seeds, n: i64, scale: i64) -> Vec<TraceTriangle> {
    let heights: Vec<i64> = (0..(n + 1) * (n + 1)).map(|_| seeds.signed(2) * scale).collect();
    let at = |x: i64, y: i64| {
        [x * scale, y * scale, heights[usize::try_from(y * (n + 1) + x).expect("small")]]
    };
    let mut triangles = Vec::new();
    for y in 0..n {
        for x in 0..n {
            triangles.push(triangle([at(x, y), at(x + 1, y), at(x + 1, y + 1)]));
            triangles.push(triangle([at(x, y), at(x + 1, y + 1), at(x, y + 1)]));
        }
    }
    triangles
}

/// Small triangles through a cube, some degenerate, some spanning it.
fn soup(seeds: &mut Seeds, count: usize, reach: i64) -> Vec<TraceTriangle> {
    (0..count)
        .map(|index| {
            let a = seeds.raw3(reach);
            let span = if index % 25 == 0 { reach } else { reach / 6 };
            let b = if index % 17 == 0 { a } else { add(a, seeds.raw3(span)) };
            triangle([a, b, add(a, seeds.raw3(span))])
        })
        .collect()
}

fn add(a: [i64; 3], b: [i64; 3]) -> [i64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

/// Every tree shape over `triangles` against the brute force, for `queries` query points.
fn assert_shape_independent(
    seeds: &mut Seeds,
    triangles: &[TraceTriangle],
    queries: &[[i64; 3]],
    context: &str,
) -> usize {
    let mut order: Vec<usize> = (0..triangles.len()).collect();
    let mut trees =
        vec![Tree::built(triangles, &order, false), Tree::built(triangles, &order, true)];
    for max_leaf in [1, 2, 4, 8] {
        seeds.shuffle(&mut order);
        trees.push(Tree::built(triangles, &order, max_leaf % 2 == 0));
        trees.push(Tree::random(triangles, &order, seeds, max_leaf));
    }
    let mut ties = 0;
    for (case, &p) in queries.iter().enumerate() {
        let p = query(p);
        let expected = brute_force(triangles, &p);
        let winner = expected.expect("a non-empty mesh has a nearest triangle");
        // Triangles other than the winner at exactly its distance.
        let nearest = p.closest_on_triangle(&triangles[winner.primitive as usize]);
        ties += triangles
            .iter()
            .filter(|t| p.closest_on_triangle(t).cmp_distance(&nearest) == Ordering::Equal)
            .count()
            - 1;
        for (shape, tree) in trees.iter().enumerate() {
            assert_eq!(
                search(tree, triangles, &p),
                expected,
                "{context}: query {case}, tree {shape}"
            );
        }
    }
    ties
}

#[wasm_bindgen_test(unsupported = test)]
fn mq_tp_010_every_tree_shape_returns_what_a_test_of_every_triangle_returns() {
    let mut seeds = Seeds(POINT_TREE_SEED);
    let mut meshes = 0;
    for round in 0..6 {
        let triangles = soup(&mut seeds, 40 + 30 * round, 8 * ONE_RAW);
        let queries: Vec<[i64; 3]> = (0..60).map(|_| seeds.raw3(10 * ONE_RAW)).collect();
        assert_shape_independent(
            &mut seeds,
            &triangles,
            &queries,
            &format!("seed {POINT_TREE_SEED:#X}, soup {round}"),
        );
        meshes += 1;
    }
    // At the edges of the trace domain.
    let edge = (1 << 40) - 1;
    let triangles = soup(&mut seeds, 80, edge / 2);
    let queries: Vec<[i64; 3]> = (0..60).map(|_| seeds.raw3(edge)).collect();
    assert_shape_independent(&mut seeds, &triangles, &queries, "wide soup");
    assert_eq!(meshes + 1, 7);
}

#[wasm_bindgen_test(unsupported = test)]
fn mq_tp_011_shared_edges_and_vertices_resolve_to_the_lowest_index_on_every_tree() {
    let mut seeds = Seeds(POINT_TREE_SEED ^ 0x0B);
    let mut ties = 0;
    for (round, scale) in [(0, ONE_RAW), (1, 1), (2, 3)] {
        let triangles = grid(&mut seeds, 6, scale);
        // Grid vertices, edge midpoints, and points above and below them: many are exactly
        // equidistant from two or more triangles.
        let mut queries = Vec::new();
        for _ in 0..80 {
            let (x, y) = (seeds.signed(8), seeds.signed(8));
            let lift = seeds.signed(3) * scale;
            queries.push([x * scale, y * scale, lift]);
            queries.push([x * scale + scale / 2, y * scale, lift]);
        }
        ties +=
            assert_shape_independent(&mut seeds, &triangles, &queries, &format!("grid {round}"));
    }
    assert!(ties > 200, "the grids should produce many exact ties: {ties}");
}

/// A root over two leaves; `leaves` lists each leaf's primitive indices.
fn two_leaves(triangles: &[TraceTriangle], leaves: [&[u32]; 2]) -> Tree {
    let bounds = |entries: &[u32]| {
        entries
            .iter()
            .map(|&i| triangles[i as usize].bounds())
            .reduce(union_aabb)
            .expect("not empty")
    };
    let [left, right] = leaves;
    let count = |entries: &[u32]| u32::try_from(entries.len()).expect("small");
    let nodes = vec![
        BvhNode {
            bounds: union_aabb(bounds(left), bounds(right)),
            first_child_or_prim_idx: 1,
            prim_count: 0,
        },
        BvhNode { bounds: bounds(left), first_child_or_prim_idx: 0, prim_count: count(left) },
        BvhNode {
            bounds: bounds(right),
            first_child_or_prim_idx: count(left),
            prim_count: count(right),
        },
    ];
    Tree { nodes, prim_indices: [left, right].concat() }
}

#[wasm_bindgen_test(unsupported = test)]
fn mq_tp_012_pruning_keeps_every_subtree_that_can_hold_the_answer() {
    let origin = query([0; 3]);
    // The left box holds the origin, but its triangle (a segment from (-2,0,0) to (0,2,0) raw)
    // is at squared distance 2 raw, at (-1, 1, 0). The right leaf's point triangle at (1,0,0)
    // is at 1: its box bound, 1, is below 2, so it must be searched. A bound one gap unit too
    // large, (1 + 1)^2 = 4, would prune it.
    let triangles =
        [triangle([[-2, 0, 0], [0, 2, 0], [0, 2, 0]]), triangle([[1, 0, 0], [1, 0, 0], [1, 0, 0]])];
    let tree = two_leaves(&triangles, [&[0], &[1]]);
    let found = search(&tree, &triangles, &origin).expect("found");
    assert_eq!((found.primitive, found.point), (1, vec3([1, 0, 0])));
    // Two equal triangles: the right leaf's box bound equals the best distance and holds the
    // lower index, so it is searched although the left leaf is searched first.
    let twin = triangle([[3, -1, 0], [3, 1, 0], [3, 0, 2]]);
    let triangles = [twin, twin];
    let tree = two_leaves(&triangles, [&[1], &[0]]);
    assert_eq!(search(&tree, &triangles, &origin).map(|f| f.primitive), Some(0));
    // The same, with the twins in one leaf in descending order.
    let leaf = BvhNode { bounds: twin.bounds(), first_child_or_prim_idx: 0, prim_count: 2 };
    let tree = Tree { nodes: vec![leaf], prim_indices: vec![1, 0] };
    assert_eq!(search(&tree, &triangles, &origin).map(|f| f.primitive), Some(0));
    // A subtree strictly farther than the best is skipped: its primitive is never tested.
    let triangles = [triangle([[1, 0, 0]; 3]), triangle([[40, 0, 0]; 3])];
    let tree = two_leaves(&triangles, [&[0], &[1]]);
    let mut tested = Vec::new();
    let found = closest_point(tree.view(), &origin, |primitive| {
        tested.push(primitive);
        Some(origin.closest_on_triangle(&triangles[primitive as usize]))
    });
    assert_eq!(found.map(|f| f.map(|f| f.primitive)), Ok(Some(0)));
    assert_eq!(tested, [0]);
}

/// A comb of `levels` interior nodes: at each, the left child is a leaf far from the origin
/// and the right child the next level, whose box holds the origin, so every far leaf waits on
/// the stack while the search descends.
fn comb(levels: u32) -> (Vec<TraceTriangle>, Tree) {
    let around = FixedAabb3::new(vec3([-1000; 3]), vec3([1000; 3]));
    let triangles: Vec<TraceTriangle> =
        (0..=levels).map(|i| triangle([[500 + i64::from(i), 0, 0]; 3])).collect();
    // Node 0 is the root; level k sits at 2k and its children at 2k + 1 (leaf) and 2k + 2.
    let mut nodes = vec![empty_node(); 2 * levels as usize + 1];
    for level in 0..levels {
        nodes[2 * level as usize] =
            BvhNode { bounds: around, first_child_or_prim_idx: 2 * level + 1, prim_count: 0 };
        let leaf = triangles[level as usize].bounds();
        nodes[2 * level as usize + 1] =
            BvhNode { bounds: leaf, first_child_or_prim_idx: level, prim_count: 1 };
    }
    let last = triangles[levels as usize].bounds();
    nodes[2 * levels as usize] =
        BvhNode { bounds: last, first_child_or_prim_idx: levels, prim_count: 1 };
    (triangles, Tree { nodes, prim_indices: (0..=levels).collect() })
}

#[wasm_bindgen_test(unsupported = test)]
fn mq_tp_013_malformed_trees_are_the_traversal_errors_and_empty_trees_find_nothing() {
    let origin = query([0; 3]);
    let everything = FixedAabb3::new(vec3([-(1 << 39); 3]), vec3([1 << 39; 3]));
    let none = |_: u32| None;
    assert_eq!(closest_point(BvhView::new(&[], &[]), &origin, none), Ok(None));
    let dangling = [BvhNode { bounds: everything, first_child_or_prim_idx: 7, prim_count: 0 }];
    assert_eq!(
        closest_point(BvhView::new(&dangling, &[]), &origin, none),
        Err(TraversalError::InvalidNodeIndex)
    );
    let wrapping =
        [BvhNode { bounds: everything, first_child_or_prim_idx: u32::MAX, prim_count: 0 }];
    assert_eq!(
        closest_point(BvhView::new(&wrapping, &[]), &origin, none),
        Err(TraversalError::InvalidNodeIndex)
    );
    let short_leaf = [BvhNode { bounds: everything, first_child_or_prim_idx: 0, prim_count: 3 }];
    assert_eq!(
        closest_point(BvhView::new(&short_leaf, &[0]), &origin, none),
        Err(TraversalError::InvalidPrimitiveRange)
    );
    // A cycle: the root's children are itself and a leaf. It ends after `n / 2` interior visits.
    let cycle = [
        BvhNode { bounds: everything, first_child_or_prim_idx: 0, prim_count: 0 },
        BvhNode { bounds: everything, first_child_or_prim_idx: 0, prim_count: 1 },
    ];
    assert_eq!(
        closest_point(BvhView::new(&cycle, &[0]), &origin, none),
        Err(TraversalError::InvalidNodeIndex)
    );
    // A comb of depth 63 holds; depth 64 needs a 65th stack entry.
    let (triangles, tree) = comb(63);
    assert_eq!(search(&tree, &triangles, &origin), brute_force(&triangles, &origin));
    let (triangles, tree) = comb(64);
    let found = closest_point(tree.view(), &origin, |p| {
        Some(origin.closest_on_triangle(&triangles[p as usize]))
    });
    assert_eq!(found, Err(TraversalError::StackOverflow));
}

#[wasm_bindgen_test(unsupported = test)]
fn mq_tp_014_primitives_the_closure_leaves_out_are_ignored() {
    let mut seeds = Seeds(POINT_TREE_SEED ^ 0x0E);
    let triangles = soup(&mut seeds, 90, 8 * ONE_RAW);
    let order: Vec<usize> = (0..triangles.len()).collect();
    let tree = Tree::built(&triangles, &order, true);
    for case in 0..40 {
        let p = query(seeds.raw3(10 * ONE_RAW));
        assert_eq!(closest_point(tree.view(), &p, |_| None), Ok(None));
        // Only the odd triangles: the brute force over them, with their own indices.
        let found = closest_point(tree.view(), &p, |i| {
            (i % 2 == 1).then(|| p.closest_on_triangle(&triangles[i as usize]))
        })
        .expect("well formed");
        let odd: Vec<TraceTriangle> = triangles.iter().copied().skip(1).step_by(2).collect();
        let expected =
            brute_force(&odd, &p).map(|f| ClosestPoint { primitive: 2 * f.primitive + 1, ..f });
        assert_eq!(found, expected, "case {case}");
    }
}
