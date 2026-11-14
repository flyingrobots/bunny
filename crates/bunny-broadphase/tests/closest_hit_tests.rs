//! `closest_hit` and `any_hit` against a test of every primitive.

use bunny_broadphase::utils::union_aabb;
use bunny_broadphase::{
    any_hit, build_bvh, closest_hit, BvhNode, BvhView, ClosestHit, TraversalError,
};
use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_query::{TraceRay, TraceTriangle};
use std::cell::Cell;
use wasm_bindgen_test::wasm_bindgen_test;

const CLOSEST_SEED: u64 = 0xB002_C105_0000_0001;
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

    fn closest(&self, ray: &TraceRay, t_max: FixedQ32_32) -> Option<ClosestHit> {
        closest_hit(self.bvh(), ray, t_max, |primitive, limit| self.distance(ray, primitive, limit))
            .expect("the tree is well formed")
    }

    fn any(&self, ray: &TraceRay, t_max: FixedQ32_32) -> bool {
        any_hit(self.bvh(), ray, t_max, |primitive| self.distance(ray, primitive, t_max).is_some())
            .expect("the tree is well formed")
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

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_101_closest_hit_equals_a_test_of_every_primitive() {
    let mut seeds = Seeds(CLOSEST_SEED);
    let scene = Scene::seeded(&mut seeds, 600);
    let (mut hits, mut rays) = (0, 0);
    for case in 0..3000 {
        let Some(ray) = seeded_ray(&mut seeds) else { continue };
        let expected = scene.brute_force(&ray, FAR);
        assert_eq!(scene.closest(&ray, FAR), expected, "seed={CLOSEST_SEED:#018x} case={case}");
        assert_eq!(scene.any(&ray, FAR), expected.is_some(), "any_hit; case={case}");
        hits += usize::from(expected.is_some());
        rays += 1;
    }
    assert!(hits > 500 && rays - hits > 500, "corpus needs hits and misses: {hits} of {rays}");
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_102_the_distance_limit_is_inclusive_and_agrees_with_every_primitive() {
    let mut seeds = Seeds(CLOSEST_SEED ^ 2);
    let scene = Scene::seeded(&mut seeds, 300);
    let mut limited = 0;
    for case in 0..2000 {
        let Some(ray) = seeded_ray(&mut seeds) else { continue };
        let Some(nearest) = scene.brute_force(&ray, FAR) else { continue };
        assert_eq!(scene.closest(&ray, nearest.distance), Some(nearest), "case={case}");
        assert!(scene.any(&ray, nearest.distance), "case={case}");
        let short = FixedQ32_32::from_raw(nearest.distance.raw() - 1);
        assert_eq!(scene.closest(&ray, short), None, "case={case}");
        assert!(!scene.any(&ray, short), "case={case}");
        limited += 1;
    }
    assert!(limited > 300, "corpus needs hits: {limited}");
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_103_equal_distances_choose_the_lowest_primitive_index() {
    let mut seeds = Seeds(CLOSEST_SEED ^ 3);
    let mut triangles = Scene::seeded(&mut seeds, 120).triangles;
    // Every triangle appears twice, so every hit is a tie between two indices.
    let copies = triangles.clone();
    triangles.extend(copies);
    let half = u32::try_from(triangles.len() / 2).expect("small scene");
    let scene = Scene::new(triangles);
    let mut hits = 0;
    for case in 0..2000 {
        let Some(ray) = seeded_ray(&mut seeds) else { continue };
        let found = scene.closest(&ray, FAR);
        assert_eq!(found, scene.brute_force(&ray, FAR), "case={case}");
        if let Some(hit) = found {
            assert!(hit.primitive < half, "case={case}: the first copy must win");
            hits += 1;
        }
    }
    assert!(hits > 200, "corpus needs hits: {hits}");
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_104_malformed_trees_are_errors_and_empty_trees_miss() {
    let mut seeds = Seeds(CLOSEST_SEED ^ 4);
    let scene = Scene::seeded(&mut seeds, 40);
    let ray = TraceRay::new(vec3([0, 0, 20 * ONE_RAW]), vec3([0, 0, -ONE_RAW])).expect("in domain");
    let miss = |_: u32, _: FixedQ32_32| None;
    let empty = BvhView::new(&[], &[]);
    assert_eq!(closest_hit(empty, &ray, FAR, miss), Ok(None));
    assert_eq!(any_hit(empty, &ray, FAR, |_| true), Ok(false));

    let everything = FixedAabb3::new(vec3([-(1 << 39); 3]), vec3([1 << 39; 3]));
    let dangling = [BvhNode { bounds: everything, first_child_or_prim_idx: 7, prim_count: 0 }];
    let dangling = BvhView::new(&dangling, &[]);
    assert_eq!(closest_hit(dangling, &ray, FAR, miss), Err(TraversalError::InvalidNodeIndex));
    let short_leaf = [BvhNode { bounds: everything, first_child_or_prim_idx: 0, prim_count: 3 }];
    assert_eq!(
        closest_hit(BvhView::new(&short_leaf, &[0]), &ray, FAR, miss),
        Err(TraversalError::InvalidPrimitiveRange)
    );
    assert!(scene.closest(&ray, FAR).is_none() || scene.any(&ray, FAR));
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_105_distances_outside_the_limit_from_the_primitive_test_are_ignored() {
    let mut seeds = Seeds(CLOSEST_SEED ^ 5);
    let scene = Scene::seeded(&mut seeds, 60);
    let ray = TraceRay::new(vec3([0, 0, 20 * ONE_RAW]), vec3([0, 0, -ONE_RAW])).expect("in domain");
    let ten = FixedQ32_32::from_raw(10 * ONE_RAW);
    let beyond = closest_hit(scene.bvh(), &ray, ten, |_, _| Some(FAR));
    assert_eq!(beyond, Ok(None));
    let negative = closest_hit(scene.bvh(), &ray, ten, |_, _| Some(FixedQ32_32::from_raw(-1)));
    assert_eq!(negative, Ok(None));
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_106_differently_shaped_trees_give_the_same_answers() {
    let mut seeds = Seeds(CLOSEST_SEED ^ 6);
    let mut triangles = Scene::seeded(&mut seeds, 150).triangles;
    // Duplicates make ties, which a shape-dependent search would resolve differently.
    let copies: Vec<TraceTriangle> = triangles.iter().step_by(3).copied().collect();
    triangles.extend(copies);
    let scene = Scene::new(triangles);
    let count = scene.triangles.len();
    let reversed: Vec<usize> = (0..count).rev().collect();
    let interleaved: Vec<usize> = (0..count).step_by(2).chain((1..count).step_by(2)).collect();
    let mut shuffled: Vec<usize> = (0..count).collect();
    for slot in (1..count).rev() {
        shuffled.swap(slot, usize::try_from(seeds.next()).expect("fits") % (slot + 1));
    }
    let shapes = [
        scene.reshaped(&reversed),
        scene.reshaped(&interleaved),
        scene.reshaped(&shuffled),
        scene.one_leaf(),
    ];
    let mut hits = 0;
    for case in 0..1500 {
        let Some(ray) = seeded_ray(&mut seeds) else { continue };
        let expected = scene.closest(&ray, FAR);
        assert_eq!(expected, scene.brute_force(&ray, FAR), "case={case}");
        for (shape, other) in shapes.iter().enumerate() {
            assert_eq!(other.closest(&ray, FAR), expected, "case={case} shape={shape}");
            assert_eq!(other.any(&ray, FAR), expected.is_some(), "case={case} shape={shape}");
        }
        hits += usize::from(expected.is_some());
    }
    assert!(hits > 200, "corpus needs hits: {hits}");
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

/// Runs `closest_hit` and records which primitives the search tested, in order.
fn traced(scene: &Scene, ray: &TraceRay) -> (Option<ClosestHit>, Vec<u32>) {
    let mut tested = Vec::new();
    let found = closest_hit(scene.bvh(), ray, FAR, |primitive, limit| {
        tested.push(primitive);
        scene.distance(ray, primitive, limit)
    });
    (found.expect("well formed"), tested)
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_107_the_nearer_child_is_searched_first_and_the_farther_one_is_skipped() {
    // Primitive 0 is 8.0 away, primitive 1 is 5.0 away; the far leaf is the left child.
    let (triangles, ray) = stack_of(&[-3, 0]);
    let scene = two_leaves(&triangles, [&[0], &[1]]);
    let (found, tested) = traced(&scene, &ray);
    let five = FixedQ32_32::from_raw(5 * ONE_RAW);
    assert_eq!(found, Some(ClosestHit { primitive: 1, distance: five }));
    assert_eq!(tested, [1], "the far leaf starts beyond 5.0 and must not be tested");
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_108_a_subtree_that_starts_at_the_best_distance_is_still_searched() {
    // Two coincident triangles: their leaves' entry bounds equal the hit distance, 5.0
    // exactly. The left leaf, searched first, holds the higher index.
    let (triangles, ray) = stack_of(&[0, 0]);
    let five = FixedQ32_32::from_raw(5 * ONE_RAW);
    let lowest = Some(ClosestHit { primitive: 0, distance: five });
    let split = two_leaves(&triangles, [&[1], &[0]]);
    assert_eq!(traced(&split, &ray), (lowest, vec![1, 0]));
    let mut one_leaf = split.one_leaf();
    one_leaf.nodes[0].bounds = triangles[0].bounds();
    assert_eq!(traced(&one_leaf, &ray), (lowest, vec![1, 0]), "a tie inside one leaf");
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_109_any_hit_stops_at_the_first_hit() {
    let (triangles, ray) = stack_of(&[0, -1, -2, -3]);
    let scene = two_leaves(&triangles, [&[0, 1], &[2, 3]]);
    let calls = Cell::new(0);
    let found = any_hit(scene.bvh(), &ray, FAR, |primitive| {
        calls.set(calls.get() + 1);
        scene.distance(&ray, primitive, FAR).is_some()
    });
    assert_eq!(found, Ok(true));
    assert_eq!(calls.get(), 1, "every primitive is hit, so one test settles it");
    let beneath = FixedQ32_32::from_raw(4 * ONE_RAW);
    calls.set(0);
    let short = any_hit(scene.bvh(), &ray, beneath, |primitive| {
        calls.set(calls.get() + 1);
        scene.distance(&ray, primitive, beneath).is_some()
    });
    assert_eq!((short, calls.get()), (Ok(false), 0), "every box starts beyond 4.0");
}
