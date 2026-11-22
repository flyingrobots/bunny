//! The compact node layout (`compact_bvh`, `compact_closest_hit`, `compact_any_hit`) on the
//! benchmark scene, against `closest_hit` and `any_hit` on the builder's tree.
//!
//! `trace/closest_hit_compact` and `trace/any_hit_compact` trace the same 1 024 rays as
//! `trace/closest_hit` and `trace/any_hit`. `trace/compact_convert` times one conversion of
//! the scene's tree (one call per batch). Three more cases place the same compact nodes in
//! other orders, to see whether placement matters: the builder's order of the source nodes
//! (`_builder_order`), depth first with the child nearer the camera placed right after its
//! parent (`_near_first`), and a seeded random order (`_shuffled`).

use std::collections::BTreeMap;
use std::hint::black_box;
use std::rc::Rc;

use bunny_broadphase::{
    compact_any_hit, compact_bvh, compact_closest_hit, BvhNode, ClosestHit, CompactChild,
    CompactNode, CompactRoot, CompactView,
};
use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use bunny_query::TraceRay;

use crate::cases::{case, Case};
use crate::scene::{scene, trace_rays, trace_triangles, Scene};

const ZERO3: FixedVec3 = FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO);
const FAR: FixedQ32_32 = FixedQ32_32::from_raw(i64::MAX);
const SHUFFLE_SEED: u64 = 0x5EED_0215_0000_0002;

/// A compact tree: its root and nodes.
pub(crate) struct Compact {
    pub(crate) root: Option<CompactRoot>,
    pub(crate) nodes: Vec<CompactNode>,
}

pub(crate) fn convert(nodes: &[BvhNode]) -> Compact {
    let empty = CompactNode {
        bounds: [FixedAabb3::new(ZERO3, ZERO3); 2],
        children: [CompactChild::default(); 2],
    };
    let mut out = vec![empty; nodes.len() / 2];
    let (root, used) = match compact_bvh(nodes, &mut out) {
        Ok(Some((root, used))) => (Some(root), used),
        _ => (None, 0),
    };
    out.truncate(used);
    Compact { root, nodes: out }
}

pub(crate) fn closest_on(
    tree: &Compact,
    scene: &Scene,
    triangles: &[bunny_query::TraceTriangle],
    ray: &TraceRay,
) -> Option<ClosestHit> {
    let ray = black_box(ray);
    let view =
        CompactView { root: tree.root, nodes: &tree.nodes, prim_indices: &scene.prim_indices };
    let hit = compact_closest_hit(view, ray, FAR, |primitive, limit| {
        let triangle = triangles.get(primitive as usize)?;
        ray.hit_triangle(triangle, FixedQ32_32::ZERO, limit).map(|hit| hit.distance)
    });
    black_box(hit).ok().flatten()
}

/// Moves compact node `i` to `slot[i]`, rewriting the references; `slot[0]` must be 0.
fn placed(tree: &Compact, slot: &[usize]) -> Compact {
    let mut nodes = tree.nodes.clone();
    for (old, node) in tree.nodes.iter().enumerate() {
        let mut node = *node;
        for child in &mut node.children {
            if !child.is_leaf() {
                child.first = u32::try_from(slot[child.first as usize]).unwrap_or(u32::MAX);
            }
        }
        nodes[slot[old]] = node;
    }
    Compact { root: tree.root, nodes }
}

/// For each compact node, its source node index (both trees walked together).
fn source_indices(source: &[BvhNode], tree: &Compact) -> Vec<usize> {
    let mut out = vec![0; tree.nodes.len()];
    let mut stack = vec![(0_usize, 0_usize)];
    while let Some((compact, from)) = stack.pop() {
        out[compact] = from;
        let first = source[from].first_child_or_prim_idx as usize;
        for (side, child) in tree.nodes[compact].children.iter().enumerate() {
            if !child.is_leaf() {
                stack.push((child.first as usize, first + side));
            }
        }
    }
    out
}

/// Compact nodes in the order of their source nodes in the builder's array.
fn builder_order(source: &[BvhNode], tree: &Compact) -> Compact {
    let sources = source_indices(source, tree);
    let ranks: BTreeMap<usize, usize> = sources.iter().enumerate().map(|(c, &s)| (s, c)).collect();
    let mut slot = vec![0; tree.nodes.len()];
    for (rank, &compact) in ranks.values().enumerate() {
        slot[compact] = rank;
    }
    placed(tree, &slot)
}

fn squared_distance(bounds: &FixedAabb3, from: [i128; 3]) -> i128 {
    let (lo, hi) = (bounds.min, bounds.max);
    let centre = [lo.x.raw() + hi.x.raw(), lo.y.raw() + hi.y.raw(), lo.z.raw() + hi.z.raw()];
    centre.iter().zip(from).map(|(&c, f)| (i128::from(c) - 2 * f).pow(2)).sum()
}

/// Depth first, with the child whose box centre is nearer the camera right after its parent.
fn near_first_order(tree: &Compact, camera: [i128; 3]) -> Compact {
    let mut slot = vec![0; tree.nodes.len()];
    let mut next = 0;
    let mut stack = vec![0_usize];
    while let Some(compact) = stack.pop() {
        slot[compact] = next;
        next += 1;
        let node = &tree.nodes[compact];
        let mut sides = [0, 1];
        sides.sort_by_key(|&side| std::cmp::Reverse(squared_distance(&node.bounds[side], camera)));
        for side in sides {
            let child = node.children[side];
            if !child.is_leaf() {
                stack.push(child.first as usize);
            }
        }
    }
    placed(tree, &slot)
}

fn shuffled_order(tree: &Compact, seed: u64) -> Compact {
    let mut slot: Vec<usize> = (0..tree.nodes.len()).collect();
    let mut state = seed;
    for i in (2..slot.len()).rev() {
        state =
            state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        slot.swap(i, 1 + usize::try_from(state >> 33).unwrap_or(0) % i);
    }
    placed(tree, &slot)
}

/// The same compact tree in the three other orders, with their case names.
fn variants(scene: &Scene, tree: &Compact, rays: &[TraceRay]) -> [(&'static str, Compact); 3] {
    let origin = rays.first().map_or(ZERO3, TraceRay::origin);
    let camera = [origin.x, origin.y, origin.z].map(|c| i128::from(c.raw()));
    [
        ("trace/closest_hit_compact_builder_order", builder_order(&scene.nodes, tree)),
        ("trace/closest_hit_compact_near_first", near_first_order(tree, camera)),
        ("trace/closest_hit_compact_shuffled", shuffled_order(tree, SHUFFLE_SEED)),
    ]
}

/// The scene, its prepared triangles and the camera rays, shared by the cases.
struct Data {
    scene: Scene,
    triangles: Vec<bunny_query::TraceTriangle>,
    rays: Vec<TraceRay>,
}

/// The compact cases.
pub(crate) fn compact_cases() -> Vec<Case> {
    let scene = scene();
    let data = Rc::new(Data { triangles: trace_triangles(&scene), scene, rays: trace_rays() });
    let tree = convert(&data.scene.nodes);
    let variants = variants(&data.scene, &tree, &data.rays);
    let tree = Rc::new(tree);
    let convert_case = {
        let data = Rc::clone(&data);
        let mut run = case("trace/compact_convert", move || {
            let tree = convert(black_box(&data.scene.nodes));
            i64::try_from(tree.nodes.len()).unwrap_or(0)
        });
        run.calls = 1;
        run
    };
    let mut cases = vec![
        closest_case("trace/closest_hit_compact", &tree, &data),
        any_case(&tree, &data),
        convert_case,
    ];
    for (name, variant) in variants {
        cases.push(closest_case(name, &Rc::new(variant), &data));
    }
    cases
}

fn closest_case(name: &'static str, tree: &Rc<Compact>, data: &Rc<Data>) -> Case {
    let (tree, data) = (Rc::clone(tree), Rc::clone(data));
    case(name, move || {
        data.rays
            .iter()
            .map(|ray| {
                closest_on(&tree, &data.scene, &data.triangles, ray)
                    .map_or(0, |hit| hit.distance.raw())
            })
            .fold(0, i64::wrapping_add)
    })
}

fn any_case(tree: &Rc<Compact>, data: &Rc<Data>) -> Case {
    let (tree, data) = (Rc::clone(tree), Rc::clone(data));
    case("trace/any_hit_compact", move || {
        data.rays
            .iter()
            .map(|ray| {
                let ray = black_box(ray);
                let view = CompactView {
                    root: tree.root,
                    nodes: &tree.nodes,
                    prim_indices: &data.scene.prim_indices,
                };
                let hit = compact_any_hit(view, ray, FAR, |primitive| {
                    data.triangles.get(primitive as usize).is_some_and(|triangle| {
                        ray.hit_triangle(triangle, FixedQ32_32::ZERO, FAR).is_some()
                    })
                });
                i64::from(black_box(hit).unwrap_or(false))
            })
            .fold(0, i64::wrapping_add)
    })
}

/// Rays on which a compact search disagrees with the builder's tree: a different
/// `closest_hit` answer, a different sequence of primitive calls, or a different `any_hit`
/// answer, on any of the compact trees the cases time.
pub(crate) fn disagreeing_rays() -> Vec<usize> {
    let scene = scene();
    let triangles = trace_triangles(&scene);
    let rays = trace_rays();
    let tree = convert(&scene.nodes);
    let mut trees: Vec<Compact> = variants(&scene, &tree, &rays).into_iter().map(|v| v.1).collect();
    trees.push(tree);
    let mut bad = Vec::new();
    for (index, ray) in rays.iter().enumerate() {
        let expected = calls_closest(&scene, &triangles, ray, None);
        let expected_any = calls_any(&scene, &triangles, ray, None);
        let same = trees.iter().all(|tree| {
            calls_closest(&scene, &triangles, ray, Some(tree)) == expected
                && calls_any(&scene, &triangles, ray, Some(tree)) == expected_any
        });
        if !same {
            bad.push(index);
        }
    }
    bad
}

type Calls<T> = (Vec<T>, Option<(u32, i64)>);

fn calls_closest(
    scene: &Scene,
    triangles: &[bunny_query::TraceTriangle],
    ray: &TraceRay,
    tree: Option<&Compact>,
) -> Calls<(u32, FixedQ32_32)> {
    let mut calls = Vec::new();
    let test = |primitive: u32, limit: FixedQ32_32| {
        calls.push((primitive, limit));
        let triangle = triangles.get(primitive as usize)?;
        ray.hit_triangle(triangle, FixedQ32_32::ZERO, limit).map(|hit| hit.distance)
    };
    let found = match tree {
        None => bunny_broadphase::closest_hit(
            bunny_broadphase::BvhView::new(&scene.nodes, &scene.prim_indices),
            ray,
            FAR,
            test,
        ),
        Some(tree) => compact_closest_hit(
            CompactView { root: tree.root, nodes: &tree.nodes, prim_indices: &scene.prim_indices },
            ray,
            FAR,
            test,
        ),
    };
    let found = found.ok().flatten().map(|hit| (hit.primitive, hit.distance.raw()));
    (calls, found)
}

fn calls_any(
    scene: &Scene,
    triangles: &[bunny_query::TraceTriangle],
    ray: &TraceRay,
    tree: Option<&Compact>,
) -> Calls<u32> {
    let mut calls = Vec::new();
    let test = |primitive: u32| {
        calls.push(primitive);
        triangles
            .get(primitive as usize)
            .is_some_and(|t| ray.hit_triangle(t, FixedQ32_32::ZERO, FAR).is_some())
    };
    let found = match tree {
        None => bunny_broadphase::any_hit(
            bunny_broadphase::BvhView::new(&scene.nodes, &scene.prim_indices),
            ray,
            FAR,
            test,
        ),
        Some(tree) => compact_any_hit(
            CompactView { root: tree.root, nodes: &tree.nodes, prim_indices: &scene.prim_indices },
            ray,
            FAR,
            test,
        ),
    };
    (calls, found.ok().filter(|&hit| hit).map(|_| (0, 0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_compact_tree_makes_the_same_calls_and_answers_as_the_builders_tree() {
        assert_eq!(disagreeing_rays(), Vec::<usize>::new());
    }

    #[test]
    fn the_compact_tree_has_one_node_per_interior_source_node() {
        let scene = scene();
        let tree = convert(&scene.nodes);
        let interior = scene.nodes.iter().filter(|node| node.prim_count == 0).count();
        assert_eq!(tree.nodes.len(), interior);
        assert!(tree.root.is_some_and(|root| !root.child.is_leaf()));
    }
}
