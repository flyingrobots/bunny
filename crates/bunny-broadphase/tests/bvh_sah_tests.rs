//! `build_bvh_sah`: buffer contract, structure, degenerate inputs, tie rules, agreement of
//! `closest_hit` and `any_hit` with a test of every primitive, and tree quality.

mod common;

use bunny_broadphase::bvh::{SAH_MAX_DEPTH, SAH_MAX_LEAF_SIZE};
use bunny_broadphase::{
    any_hit, build_bvh, build_bvh_sah, closest_hit, intersect_aabb, BvhNode, BvhView, ClosestHit,
};
use bunny_geom::FixedAabb3;
use bunny_num::FixedQ32_32;
use bunny_query::{TraceRay, TraceTriangle};
use common::{
    aabb, build_with, check_structure, empty_node, sah_cost, tree_words, Seeds, Tree, ONE,
};
use wasm_bindgen_test::wasm_bindgen_test;

const SAH_SEED: u64 = 0x5A4B_B0D5_0000_0213;
const FAR: FixedQ32_32 = FixedQ32_32::from_raw(i64::MAX);

fn sah(primitives: &[FixedAabb3]) -> Tree {
    build_with(build_bvh_sah, primitives)
}

fn unit_box(x: i64, y: i64, z: i64) -> FixedAabb3 {
    aabb([x * ONE, y * ONE, z * ONE], [(x + 1) * ONE, (y + 1) * ONE, (z + 1) * ONE])
}

/// Each node as `(first_child_or_prim_idx, prim_count)`.
fn layout(tree: &Tree) -> Vec<(u32, u32)> {
    tree.nodes.iter().map(|node| (node.first_child_or_prim_idx, node.prim_count)).collect()
}

// ---- Buffers and trivial inputs ----

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_001_empty_input_uses_no_nodes() {
    let mut nodes: [BvhNode; 0] = [];
    let mut prim_indices = [0_u32; 0];
    assert_eq!(build_bvh_sah(&mut nodes, &mut prim_indices, &[]), Some(0));
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_002_one_primitive_is_one_leaf() {
    let primitives = [unit_box(-2, 3, 4)];
    let mut nodes = [empty_node(); 1];
    let mut prim_indices = [u32::MAX; 1];
    assert_eq!(build_bvh_sah(&mut nodes, &mut prim_indices, &primitives), Some(1));
    assert_eq!(nodes[0].bounds, primitives[0]);
    assert_eq!((nodes[0].first_child_or_prim_idx, nodes[0].prim_count), (0, 1));
    assert_eq!(prim_indices, [0]);
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_003_buffers_too_small_are_refused() {
    let primitives: Vec<FixedAabb3> = (0..5).map(|i| unit_box(3 * i, 0, 0)).collect();
    let mut short_nodes = vec![empty_node(); 8];
    let mut prim_indices = vec![0_u32; 5];
    assert_eq!(build_bvh_sah(&mut short_nodes, &mut prim_indices, &primitives), None);
    let mut nodes = vec![empty_node(); 9];
    let mut short_indices = vec![0_u32; 4];
    assert_eq!(build_bvh_sah(&mut nodes, &mut short_indices, &primitives), None);
    let mut no_nodes: [BvhNode; 0] = [];
    let mut one_index = [0_u32; 1];
    assert_eq!(build_bvh_sah(&mut no_nodes, &mut one_index, &primitives[..1]), None);
    // Exactly 2n - 1 nodes and n indices fit; larger buffers are fine too.
    assert!(build_bvh_sah(&mut nodes, &mut prim_indices, &primitives).is_some());
    let mut roomy = vec![empty_node(); 20];
    let mut roomy_indices = vec![0_u32; 7];
    assert!(build_bvh_sah(&mut roomy, &mut roomy_indices, &primitives).is_some());
}

// ---- Structure ----

/// Small boxes scattered through a cube, plus one in forty spanning most of it.
fn scattered_boxes(seeds: &mut Seeds, count: usize) -> Vec<FixedAabb3> {
    (0..count)
        .map(|index| {
            let reach = if index % 40 == 0 { 6 * ONE } else { ONE / 2 };
            let corner = seeds.point(6 * ONE);
            let size = seeds.point(reach);
            let far =
                corner + common::vec3([size.x.raw().abs(), size.y.raw().abs(), size.z.raw().abs()]);
            FixedAabb3::new(corner, far)
        })
        .collect()
}

/// Eight dense clusters of small boxes and a few long thin ones crossing them.
fn clustered_boxes(seeds: &mut Seeds) -> Vec<FixedAabb3> {
    let mut boxes = Vec::new();
    for cluster in 0..8 {
        let centre = seeds.point(20 * ONE);
        for _ in 0..60 + 10 * cluster {
            let corner = centre + seeds.point(ONE);
            let size = seeds.point(ONE / 8);
            let far =
                corner + common::vec3([size.x.raw().abs(), size.y.raw().abs(), size.z.raw().abs()]);
            boxes.push(FixedAabb3::new(corner, far));
        }
    }
    for index in 0..5 {
        let y = seeds.signed(20 * ONE);
        boxes.insert(index * 97, aabb([-20 * ONE, y, -ONE], [20 * ONE, y + ONE / 16, ONE]));
    }
    boxes
}

/// A 24 by 24 grid of small boxes standing on two huge ground boxes, like the benchmark
/// scene's ground triangles.
fn grid_on_ground() -> Vec<FixedAabb3> {
    let mut boxes = vec![
        aabb([-20 * ONE, -ONE, -20 * ONE], [20 * ONE, -ONE, 20 * ONE]),
        aabb([-20 * ONE, -ONE, -20 * ONE], [20 * ONE, -ONE, 20 * ONE]),
    ];
    for i in 0..24 {
        for j in 0..24 {
            let (x, z) = ((i - 12) * ONE / 12, (j - 12) * ONE / 12);
            boxes.push(aabb([x, 0, z], [x + ONE / 13, ONE / 13, z + ONE / 13]));
        }
    }
    boxes
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_004_seeded_trees_are_well_formed() {
    let mut seeds = Seeds(SAH_SEED);
    for count in [2, 3, 4, 5, 7, 16, 17, 100, 600] {
        let primitives = scattered_boxes(&mut seeds, count);
        let shape = check_structure(&sah(&primitives), &primitives, true);
        assert!(shape.largest_leaf <= SAH_MAX_LEAF_SIZE, "count={count}: {shape:?}");
    }
    for primitives in [clustered_boxes(&mut seeds), grid_on_ground()] {
        let shape = check_structure(&sah(&primitives), &primitives, true);
        assert!(shape.largest_leaf <= SAH_MAX_LEAF_SIZE, "{shape:?}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_005_building_twice_gives_the_same_tree() {
    let mut seeds = Seeds(SAH_SEED ^ 0x5);
    let primitives = clustered_boxes(&mut seeds);
    let first = sah(&primitives);
    let copy = primitives.clone();
    let second = sah(&copy);
    assert_eq!(tree_words(&first), tree_words(&second));
    // Stale contents of the output buffers do not leak into the result.
    let mut nodes = vec![common::empty_node(); 2 * primitives.len()];
    for node in &mut nodes {
        node.first_child_or_prim_idx = 77;
        node.prim_count = 9;
    }
    let mut prim_indices: Vec<u32> = (0..u32::try_from(primitives.len()).unwrap()).rev().collect();
    let used = build_bvh_sah(&mut nodes, &mut prim_indices, &primitives).unwrap();
    nodes.truncate(used);
    assert_eq!(tree_words(&Tree { nodes, prim_indices }), tree_words(&first));
}

// ---- Degenerate inputs ----

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_006_identical_boxes_split_in_half_by_index() {
    let primitives = vec![unit_box(1, 2, 3); 1000];
    let tree = sah(&primitives);
    let shape = check_structure(&tree, &primitives, true);
    assert!(shape.largest_leaf <= SAH_MAX_LEAF_SIZE, "{shape:?}");
    // Halving 1000 down to at most four takes eight levels.
    assert_eq!(shape.max_depth, 8);
    assert_eq!(tree.prim_indices, (0..1000).collect::<Vec<u32>>());
    // The root's left child holds slots 0..500.
    let left = tree.nodes[tree.nodes[0].first_child_or_prim_idx as usize];
    assert_eq!(left.prim_count, 0);
    // Ten identical boxes: 5 | 5, then 2 | 3 on each side.
    let ten = sah(&primitives[..10]);
    assert_eq!(layout(&ten), [(1, 0), (3, 0), (5, 0), (0, 2), (2, 3), (5, 2), (7, 3)]);
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_007_equal_centroids_and_zero_extent_axes_build() {
    // Nested boxes around one centre: every centroid equal, sizes differ.
    let nested: Vec<FixedAabb3> = (1..=50).map(|i| aabb([-i * ONE; 3], [i * ONE; 3])).collect();
    let shape = check_structure(&sah(&nested), &nested, true);
    assert!(shape.largest_leaf <= SAH_MAX_LEAF_SIZE);
    // Flat boxes in one plane (zero z extent, all z centroids equal) and points.
    let mut seeds = Seeds(SAH_SEED ^ 0x7);
    let mut flat: Vec<FixedAabb3> = (0..200)
        .map(|_| {
            let p = seeds.point(5 * ONE);
            aabb([p.x.raw(), p.y.raw(), 0], [p.x.raw() + ONE / 4, p.y.raw() + ONE / 4, 0])
        })
        .collect();
    flat.extend((0..100).map(|_| {
        let p = seeds.point(5 * ONE);
        FixedAabb3::new(p, p)
    }));
    check_structure(&sah(&flat), &flat, true);
    // Points on one line along y.
    let line: Vec<FixedAabb3> = (0..64).map(|i| aabb([0, i * ONE, 0], [0, i * ONE, 0])).collect();
    check_structure(&sah(&line), &line, true);
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_008_one_huge_primitive_among_small_ones() {
    let mut primitives = grid_on_ground();
    primitives.truncate(3);
    let mut seeds = Seeds(SAH_SEED ^ 0x8);
    primitives.extend(scattered_boxes(&mut seeds, 2000).into_iter().map(|b| {
        FixedAabb3::new(
            b.min,
            common::vec3([b.min.x.raw() + ONE / 20, b.min.y.raw() + ONE / 20, b.min.z.raw()]),
        )
    }));
    let tree = sah(&primitives);
    check_structure(&tree, &primitives, true);
    assert!(sah_cost(&tree) < sah_cost(&build_with(build_bvh, &primitives)));
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_009_boxes_at_the_ends_of_the_range_build_without_overflow() {
    let (low, high) = (i64::MIN, i64::MAX);
    let primitives = vec![
        aabb([low; 3], [high; 3]),
        aabb([low; 3], [low + 1; 3]),
        aabb([high - 1; 3], [high; 3]),
        aabb([low, 0, high - 5], [0, high, high]),
        aabb([0; 3], [0; 3]),
        aabb([low; 3], [high; 3]),
        aabb([high / 2; 3], [high; 3]),
    ];
    let tree = sah(&primitives);
    check_structure(&tree, &primitives, true);
    let everywhere = aabb([low; 3], [high; 3]);
    let mut visited = 0;
    intersect_aabb(&tree.nodes, &tree.prim_indices, &everywhere, |_| visited += 1).unwrap();
    assert_eq!(visited, primitives.len());
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_010_geometric_spacing_stays_within_the_depth_cap() {
    // Point boxes at 2^k raw units on each axis in turn, the spacing that peels the fewest
    // primitives off per equal-width binning split.
    let mut primitives = Vec::new();
    for axis in 0..3 {
        for k in 0..62 {
            let mut raw = [0_i64; 3];
            raw[axis] = 1_i64 << k;
            primitives.push(aabb(raw, raw));
        }
    }
    let tree = sah(&primitives);
    let shape = check_structure(&tree, &primitives, true);
    assert!(shape.max_depth <= SAH_MAX_DEPTH);
    let everywhere = aabb([i64::MIN; 3], [i64::MAX; 3]);
    let mut visited = 0;
    intersect_aabb(&tree.nodes, &tree.prim_indices, &everywhere, |_| visited += 1).unwrap();
    assert_eq!(visited, primitives.len());
}

// ---- Tie rules and golden layouts ----

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_011_equal_split_and_leaf_costs_make_a_leaf() {
    // Two segments on the x axis: every area is zero, so the split costs exactly the leaf.
    let primitives = [aabb([0; 3], [ONE, 0, 0]), aabb([2 * ONE, 0, 0], [3 * ONE, 0, 0])];
    let tree = sah(&primitives);
    assert_eq!(layout(&tree), [(0, 2)]);
    assert_eq!(tree.prim_indices, [0, 1]);
    // Give them depth and the split pays: 7 + 3 + 3 = 13 against 2 * 7 = 14 (half areas in
    // units^2: the node is 3 by 1 by 1, each cube 1 by 1 by 1).
    let solid = [unit_box(0, 0, 0), unit_box(2, 0, 0)];
    assert_eq!(layout(&sah(&solid)), [(1, 0), (0, 1), (1, 1)]);
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_012_equal_planes_on_one_axis_choose_the_lowest_plane() {
    // Every plane costs 52 (units^2): {0} | {1, 2} below the middle box, {0, 1} | {2}
    // above it. The lowest plane wins, so box 0 is split off first.
    let primitives = [unit_box(0, 0, 0), unit_box(5, 0, 0), unit_box(10, 0, 0)];
    let tree = sah(&primitives);
    assert_eq!(layout(&tree), [(1, 0), (0, 1), (3, 0), (1, 1), (2, 1)]);
    assert_eq!(tree.prim_indices, [0, 1, 2]);
    assert_eq!(tree.nodes[2].bounds, aabb([5 * ONE, 0, 0], [11 * ONE, ONE, ONE]));
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_013_equal_axes_choose_x_before_y() {
    // Four unit cubes on a square: splitting on x or y costs the same; x wins.
    let primitives =
        [unit_box(0, 0, 0), unit_box(10, 0, 0), unit_box(0, 10, 0), unit_box(10, 10, 0)];
    let tree = sah(&primitives);
    assert_eq!(layout(&tree), [(1, 0), (3, 0), (5, 0), (0, 1), (1, 1), (2, 1), (3, 1)]);
    assert_eq!(tree.prim_indices, [0, 2, 1, 3]);
    // Swap the roles: with the square stretched along y, y is cheaper and wins.
    let tall = [unit_box(0, 0, 0), unit_box(10, 0, 0), unit_box(0, 30, 0), unit_box(10, 30, 0)];
    assert_eq!(sah(&tall).prim_indices, [0, 1, 2, 3]);
}

// ---- Rays: agreement with a test of every primitive and with the old tree ----

struct Scene {
    triangles: Vec<TraceTriangle>,
    sah: Tree,
    old: Tree,
}

impl Scene {
    fn new(triangles: Vec<TraceTriangle>) -> Self {
        let boxes: Vec<FixedAabb3> = triangles.iter().map(TraceTriangle::bounds).collect();
        let sah = sah(&boxes);
        check_structure(&sah, &boxes, true);
        Self { old: build_with(build_bvh, &boxes), sah, triangles }
    }

    fn seeded(seeds: &mut Seeds, count: usize) -> Self {
        let mut triangles: Vec<TraceTriangle> = (0..count)
            .filter_map(|index| {
                let reach = if index % 40 == 0 { 6 * ONE } else { ONE };
                let v0 = seeds.point(6 * ONE);
                TraceTriangle::new(v0, v0 + seeds.point(reach), v0 + seeds.point(reach))
            })
            .collect();
        // Two ground triangles spanning the scene, and a duplicate of every tenth triangle
        // so some hits tie.
        let ground = |x: i64, z: i64| common::vec3([x * ONE, -7 * ONE, z * ONE]);
        triangles.extend(TraceTriangle::new(ground(-40, -40), ground(40, -40), ground(40, 40)));
        triangles.extend(TraceTriangle::new(ground(-40, -40), ground(40, 40), ground(-40, 40)));
        let copies: Vec<TraceTriangle> = triangles.iter().step_by(10).copied().collect();
        triangles.extend(copies);
        Self::new(triangles)
    }

    fn distance(&self, ray: &TraceRay, primitive: u32, limit: FixedQ32_32) -> Option<FixedQ32_32> {
        let triangle = self.triangles.get(usize::try_from(primitive).ok()?)?;
        ray.hit_triangle(triangle, FixedQ32_32::ZERO, limit).map(|hit| hit.distance)
    }

    fn closest(&self, tree: &Tree, ray: &TraceRay) -> Option<ClosestHit> {
        let bvh = BvhView::new(&tree.nodes, &tree.prim_indices);
        closest_hit(bvh, ray, FAR, |primitive, limit| self.distance(ray, primitive, limit)).unwrap()
    }

    fn any(&self, tree: &Tree, ray: &TraceRay) -> bool {
        let bvh = BvhView::new(&tree.nodes, &tree.prim_indices);
        any_hit(bvh, ray, FAR, |primitive| self.distance(ray, primitive, FAR).is_some()).unwrap()
    }

    fn brute_force(&self, ray: &TraceRay) -> Option<ClosestHit> {
        (0..self.triangles.len())
            .filter_map(|index| {
                let primitive = u32::try_from(index).ok()?;
                Some(ClosestHit { primitive, distance: self.distance(ray, primitive, FAR)? })
            })
            .min_by_key(|hit| (hit.distance, hit.primitive))
    }
}

fn seeded_ray(seeds: &mut Seeds) -> Option<TraceRay> {
    let origin = seeds.point(8 * ONE);
    let toward = seeds.point(6 * ONE) - origin;
    let mut direction = [toward.x.raw() / 16, toward.y.raw() / 16, toward.z.raw() / 16];
    for component in &mut direction {
        match seeds.next() % 10 {
            0 => *component = 0,
            1 => *component = seeds.signed(2),
            _ => {}
        }
    }
    TraceRay::new(origin, common::vec3(direction))
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_014_closest_and_any_hit_on_the_sah_tree_equal_every_primitive_and_the_old_tree() {
    let mut seeds = Seeds(SAH_SEED ^ 0xE);
    for count in [40, 600] {
        let scene = Scene::seeded(&mut seeds, count);
        let (mut hits, mut rays) = (0, 0);
        for case in 0..2500 {
            let Some(ray) = seeded_ray(&mut seeds) else { continue };
            let expected = scene.brute_force(&ray);
            assert_eq!(
                scene.closest(&scene.sah, &ray),
                expected,
                "seed={SAH_SEED:#x} count={count} case={case}"
            );
            assert_eq!(scene.closest(&scene.old, &ray), expected, "old tree; case={case}");
            assert_eq!(scene.any(&scene.sah, &ray), expected.is_some(), "any_hit; case={case}");
            assert_eq!(scene.any(&scene.old, &ray), expected.is_some(), "old any_hit; case={case}");
            hits += usize::from(expected.is_some());
            rays += 1;
        }
        assert!(hits > 400 && rays - hits > 300, "corpus needs hits and misses: {hits} of {rays}");
    }
}

// ---- Quality ----

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_015_sah_cost_is_lower_than_the_quartile_builder_on_seeded_scenes() {
    let mut seeds = Seeds(SAH_SEED ^ 0xF);
    let scenes = [
        ("scattered 600", scattered_boxes(&mut seeds, 600)),
        ("clustered", clustered_boxes(&mut seeds)),
        ("grid on ground", grid_on_ground()),
    ];
    for (name, primitives) in scenes {
        let (old, new) = (build_with(build_bvh, &primitives), sah(&primitives));
        let (old_cost, new_cost) = (sah_cost(&old), sah_cost(&new));
        let root = common::half_area(&new.nodes[0].bounds);
        // Cost per unit root area, in thousandths, for the record.
        println!(
            "{name}: old {} nodes, cost {}; sah {} nodes, cost {}",
            old.nodes.len(),
            old_cost * 1000 / root,
            new.nodes.len(),
            new_cost * 1000 / root
        );
        assert!(new_cost < old_cost, "{name}: sah {new_cost} not below old {old_cost}");
    }
}
