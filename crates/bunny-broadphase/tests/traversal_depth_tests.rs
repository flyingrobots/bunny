//! The depth the traversals' 64-entry stacks hold (TRACE-REQ-033) and the depth `compact_bvh`
//! converts (TRACE-REQ-034), at both boundaries, on hand-built trees.
//!
//! The depth of a tree is the number of edges from the root to its deepest leaf (the root is
//! at depth 0, so a tree of depth `d` has `d + 1` levels). Every expected count below follows
//! from the trees' shapes; the answers below the bound are those of a test of every primitive.

use bunny_broadphase::utils::{aabbs_overlap, union_aabb};
use bunny_broadphase::{
    any_hit, closest_hit, compact_any_hit, compact_bvh, compact_closest_hit, intersect_aabb,
    intersect_ray, BvhNode, BvhView, ClosestHit, CompactError, CompactNode, CompactView,
    TraversalError,
};
use bunny_geom::{FixedAabb3, FixedRay3};
use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::FixedQ32_32;
use bunny_query::TraceRay;
use wasm_bindgen_test::wasm_bindgen_test;

const ONE: i64 = bunny_num::fixed_q32_32::ONE_RAW;
const FAR: FixedQ32_32 = FixedQ32_32::from_raw(i64::MAX);

fn vec3(raw: [i64; 3]) -> FixedVec3 {
    FixedVec3::new(
        FixedQ32_32::from_raw(raw[0]),
        FixedQ32_32::from_raw(raw[1]),
        FixedQ32_32::from_raw(raw[2]),
    )
}

fn aabb(min: [i64; 3], max: [i64; 3]) -> FixedAabb3 {
    FixedAabb3::new(vec3(min), vec3(max))
}

/// A tree to flatten: a leaf of one primitive, or an interior node.
enum Shape {
    Leaf(u32),
    Node(Box<Self>, Box<Self>),
}

/// A flattened tree in the format both builders write: root 0, children an adjacent pair
/// reserved in depth-first order, one primitive per leaf, bounds the union below.
struct Flat {
    nodes: Vec<BvhNode>,
    prim_indices: Vec<u32>,
}

fn flatten(shape: &Shape, boxes: &[FixedAabb3]) -> Flat {
    let placeholder = BvhNode { bounds: boxes[0], first_child_or_prim_idx: 0, prim_count: 0 };
    let mut flat = Flat { nodes: vec![placeholder], prim_indices: Vec::new() };
    write(shape, 0, boxes, &mut flat);
    flat
}

fn write(shape: &Shape, index: usize, boxes: &[FixedAabb3], flat: &mut Flat) -> FixedAabb3 {
    let placeholder = flat.nodes[0];
    let (node, bounds) = match shape {
        Shape::Leaf(primitive) => {
            let first = u32::try_from(flat.prim_indices.len()).expect("small");
            flat.prim_indices.push(*primitive);
            let bounds = boxes[*primitive as usize];
            (BvhNode { bounds, first_child_or_prim_idx: first, prim_count: 1 }, bounds)
        }
        Shape::Node(left, right) => {
            let first = flat.nodes.len();
            flat.nodes.extend([placeholder, placeholder]);
            let bounds =
                union_aabb(write(left, first, boxes, flat), write(right, first + 1, boxes, flat));
            let first = u32::try_from(first).expect("small");
            (BvhNode { bounds, first_child_or_prim_idx: first, prim_count: 0 }, bounds)
        }
    };
    flat.nodes[index] = node;
    bounds
}

/// A comb of depth `depth`: at level `k` the left child is the leaf of primitive `k` and the
/// right child is the next level; the deepest leaf is primitive `depth`. Primitive `depth` is at
/// `x` in `[0, 1/2]` and primitive `k < depth` at `x` in `[depth - k + 1, depth - k + 3/2]`
/// units, so along `+x` from `x = -2` every right subtree is entered (at 2 units) before its
/// left leaf. Every search therefore descends right first and keeps the left leaf of each
/// level pending: the deepest interior node pushes its two children on top of `depth - 1`
/// pending leaves, `depth + 1` entries.
fn comb(depth: u32) -> (Flat, Vec<FixedAabb3>) {
    let boxes: Vec<FixedAabb3> = (0..=depth)
        .map(|k| {
            let low = if k == depth { 0 } else { i64::from(depth - k + 1) * ONE };
            aabb([low, -ONE, -ONE], [low + ONE / 2, ONE, ONE])
        })
        .collect();
    let mut shape = Shape::Leaf(depth);
    for k in (0..depth).rev() {
        shape = Shape::Node(Box::new(Shape::Leaf(k)), Box::new(shape));
    }
    (flatten(&shape, &boxes), boxes)
}

/// What the six searches return.
#[derive(Debug, PartialEq, Eq)]
struct Answers {
    closest: Result<Option<ClosestHit>, TraversalError>,
    any: Result<bool, TraversalError>,
    boxed: Result<Vec<u32>, TraversalError>,
    rayed: Result<Vec<u32>, TraversalError>,
    compact_closest: Result<Option<ClosestHit>, TraversalError>,
    compact_any: Result<bool, TraversalError>,
}

fn trace_ray() -> TraceRay {
    TraceRay::new(vec3([-2 * ONE, 0, 0]), vec3([ONE, 0, 0])).expect("a trace ray")
}

fn everything() -> FixedAabb3 {
    aabb([-1 << 40; 3], [1 << 40; 3])
}

/// The six searches over `flat` (the compact two on its conversion), with box entry as the
/// primitive test.
fn search(flat: &Flat, boxes: &[FixedAabb3]) -> Answers {
    let ray = trace_ray();
    let entry = |primitive: u32, limit| ray.box_entry(&boxes[primitive as usize], limit);
    let view = BvhView::new(&flat.nodes, &flat.prim_indices);
    let mut out = vec![CompactNode { bounds: [boxes[0]; 2], children: Default::default() }; 128];
    let (root, written) = compact_bvh(&flat.nodes, &mut out).expect("converts").expect("a root");
    let compact =
        CompactView { root: Some(root), nodes: &out[..written], prim_indices: &flat.prim_indices };
    let segment = FixedRay3::new(vec3([-2 * ONE, 0, 0]), FixedUnitVec3::UNIT_X);
    let (nodes, prims) = (&flat.nodes, &flat.prim_indices);
    Answers {
        closest: closest_hit(view, &ray, FAR, entry),
        any: any_hit(view, &ray, FAR, |primitive| entry(primitive, FAR).is_some()),
        boxed: visited(|seen| intersect_aabb(nodes, prims, &everything(), |p| seen.push(p))),
        rayed: visited(|seen| intersect_ray(nodes, prims, &segment, |p| seen.push(p))),
        compact_closest: compact_closest_hit(compact, &ray, FAR, entry),
        compact_any: compact_any_hit(compact, &ray, FAR, |primitive| {
            entry(primitive, FAR).is_some()
        }),
    }
}

/// The primitives a visitor search reported, in ascending order.
fn visited(
    run: impl FnOnce(&mut Vec<u32>) -> Result<(), TraversalError>,
) -> Result<Vec<u32>, TraversalError> {
    let mut seen = Vec::new();
    run(&mut seen)?;
    seen.sort_unstable();
    Ok(seen)
}

/// What a test of every primitive returns for the six searches.
fn every_primitive(boxes: &[FixedAabb3]) -> Answers {
    let ray = trace_ray();
    let segment = FixedRay3::new(vec3([-2 * ONE, 0, 0]), FixedUnitVec3::UNIT_X);
    let indices = (0..boxes.len()).map(|i| (u32::try_from(i).expect("small"), boxes[i]));
    let nearest = indices
        .clone()
        .filter_map(|(primitive, b)| ray.box_entry(&b, FAR).map(|t| (t, primitive)))
        .min()
        .map(|(distance, primitive)| ClosestHit { primitive, distance });
    let overlapping = indices.clone().filter(|(_, b)| aabbs_overlap(b, &everything()));
    let crossed = indices.filter(|(_, b)| bunny_query::ray_intersects_aabb(&segment, b).is_some());
    Answers {
        closest: Ok(nearest),
        any: Ok(nearest.is_some()),
        boxed: Ok(overlapping.map(|(primitive, _)| primitive).collect()),
        rayed: Ok(crossed.map(|(primitive, _)| primitive).collect()),
        compact_closest: Ok(nearest),
        compact_any: Ok(nearest.is_some()),
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_412_a_comb_of_depth_63_is_searched_and_one_of_depth_64_overflows() {
    // Depth 63: 62 pending leaves plus two children, 64 entries, exactly the stack.
    let (flat, boxes) = comb(63);
    let expected = every_primitive(&boxes);
    let nearest = expected.closest.map(|hit| hit.map(|hit| hit.primitive));
    assert_eq!(nearest, Ok(Some(63)), "the deepest leaf is entered first");
    assert_eq!(expected.boxed.as_ref().map(Vec::len), Ok(64), "every primitive is reached");
    assert_eq!(search(&flat, &boxes), expected, "depth 63");
    // Depth 64: 65 entries.
    let (flat, boxes) = comb(64);
    let overflow = TraversalError::StackOverflow;
    let expected = Answers {
        closest: Err(overflow),
        any: Err(overflow),
        boxed: Err(overflow),
        rayed: Err(overflow),
        compact_closest: Err(overflow),
        compact_any: Err(overflow),
    };
    assert_eq!(search(&flat, &boxes), expected, "depth 64");
}

/// A spine of depth `depth` whose left child at every level is the next level and whose right
/// child is an interior node of two leaves, and the bottom level two leaves. `compact_bvh`
/// writes the left subtree first and queues each right one: the deepest interior node of the
/// spine (depth `depth - 2`) queues its two children on top of the `depth - 2` right subtrees
/// still waiting above it, `depth` entries.
fn twig_spine(depth: u32) -> Flat {
    let pair =
        |first: u32| Shape::Node(Box::new(Shape::Leaf(first)), Box::new(Shape::Leaf(first + 1)));
    let mut shape = pair(0);
    for level in 1..depth {
        shape = Shape::Node(Box::new(shape), Box::new(pair(2 * level)));
    }
    let cube = aabb([0; 3], [ONE; 3]);
    flatten(&shape, &vec![cube; 2 * depth as usize])
}

fn convert(flat: &Flat) -> Result<usize, CompactError> {
    let cube = aabb([0; 3], [ONE; 3]);
    let mut out = vec![CompactNode { bounds: [cube; 2], children: Default::default() }; 256];
    Ok(compact_bvh(&flat.nodes, &mut out)?.expect("a root").1)
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_413_compact_conversion_queues_at_most_one_subtree_per_level() {
    // 64 entries at depth 64: it converts, writing all 2 * 63 + 1 interior nodes.
    assert_eq!(convert(&twig_spine(64)), Ok(127));
    assert_eq!(convert(&twig_spine(65)), Err(CompactError::TooDeep));
    // A comb queues only its next level: depth 101 converts although no search holds it.
    assert_eq!(convert(&comb(101).0), Ok(101));
}
