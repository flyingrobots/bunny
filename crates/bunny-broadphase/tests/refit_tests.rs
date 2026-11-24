//! `refit_bvh`: the builders' node order it relies on, its post-condition against an
//! independent validator, identity and round trip, every refusal on its boundary, and the
//! all-or-nothing rule. Query equivalence with rebuilt trees is in `refit_query_tests.rs`.

mod common;
mod refit_support;

use bunny_broadphase::{build_bvh, build_bvh_sah, refit_bvh, BvhNode, RefitError};
use bunny_geom::FixedAabb3;
use common::{aabb, build_with, tree_words, Builder, Seeds, Tree, ONE};
use refit_support::{assert_refitted, corpora, layout, motions, refitted};
use wasm_bindgen_test::wasm_bindgen_test;

const REFIT_SEED: u64 = 0x2EF1_7000_0000_0120;

const BUILDERS: [(&str, Builder); 2] = [("build_bvh", build_bvh), ("build_bvh_sah", build_bvh_sah)];

fn leaf(first: u32, count: u32, bounds: FixedAabb3) -> BvhNode {
    BvhNode { bounds, first_child_or_prim_idx: first, prim_count: count }
}

fn interior(first: u32) -> BvhNode {
    BvhNode { bounds: aabb([0; 3], [0; 3]), first_child_or_prim_idx: first, prim_count: 0 }
}

fn unit(x: i64) -> FixedAabb3 {
    aabb([x * ONE, 0, 0], [(x + 1) * ONE, ONE, ONE])
}

/// A hand-built five-node tree over four primitives: root 0 -> (1, 2), node 1 a leaf of
/// slots 0..2, node 2 -> (3, 4), leaves 3 and 4 one slot each. Slots list 3, 0, 2, 1.
fn small_tree() -> Tree {
    let any = unit(0);
    Tree {
        nodes: vec![interior(1), leaf(0, 2, any), interior(3), leaf(2, 1, any), leaf(3, 1, any)],
        prim_indices: vec![3, 0, 2, 1],
    }
}

fn small_boxes() -> Vec<FixedAabb3> {
    (0..4).map(|index| unit(3 * index)).collect()
}

/// Refits a copy and asserts that a refusal leaves every word of it unchanged.
fn refusal(tree: &Tree, boxes: &[FixedAabb3]) -> RefitError {
    let mut copy = tree.clone();
    let error = refit_bvh(&mut copy.nodes, &copy.prim_indices, boxes).expect_err("refused");
    assert_eq!(tree_words(&copy), tree_words(tree), "a refused refit wrote: {error:?}");
    error
}

// ---- The order refit relies on ----

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_040_both_builders_store_children_after_their_parent() {
    let mut trees = 0;
    for (corpus, boxes) in corpora(REFIT_SEED) {
        for (builder_name, builder) in BUILDERS {
            let tree = build_with(builder, &boxes);
            let (mut interior_nodes, mut leaves) = (0_usize, 0_usize);
            for (index, node) in tree.nodes.iter().enumerate() {
                if node.prim_count > 0 {
                    leaves += 1;
                    continue;
                }
                interior_nodes += 1;
                let first = node.first_child_or_prim_idx as usize;
                assert!(first > index, "{builder_name} {corpus}: node {index} -> {first}");
                assert!(first + 1 < tree.nodes.len(), "{builder_name} {corpus}: node {index}");
            }
            assert_eq!(interior_nodes + 1, leaves, "{builder_name} {corpus}: full binary tree");
            trees += 1;
        }
    }
    assert_eq!(trees, 14, "seven corpora, two builders");
}

// ---- The post-condition ----

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_041_refit_gives_the_exact_union_and_keeps_topology_and_order() {
    let mut seeds = Seeds(REFIT_SEED ^ 0x41);
    let mut refits = 0;
    for (corpus, boxes) in corpora(REFIT_SEED) {
        for (motion, moved) in motions(&mut seeds, &boxes) {
            for (builder_name, builder) in BUILDERS {
                let tree = build_with(builder, &boxes);
                let context = format!("seed={REFIT_SEED:#x} {builder_name} {corpus} {motion}");
                let after = refitted(&tree, &moved).expect("a built tree refits");
                assert_refitted(&after, &moved, &context);
                assert_eq!(layout(&after), layout(&tree), "{context}: topology changed");
                assert_eq!(after.prim_indices, tree.prim_indices, "{context}: order changed");
                refits += 1;
            }
        }
    }
    assert_eq!(refits, 70, "seven corpora, five motions, two builders");
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_042_refit_to_the_build_input_is_the_identity_bit_for_bit() {
    for (corpus, boxes) in corpora(REFIT_SEED ^ 0x42) {
        for (builder_name, builder) in BUILDERS {
            let tree = build_with(builder, &boxes);
            let after = refitted(&tree, &boxes).expect("a built tree refits");
            assert_eq!(tree_words(&after), tree_words(&tree), "{builder_name} {corpus}");
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_043_refit_and_refit_back_restores_the_tree_bit_for_bit() {
    let mut seeds = Seeds(REFIT_SEED ^ 0x43);
    for (corpus, boxes) in corpora(REFIT_SEED ^ 0x43) {
        for (motion, moved) in motions(&mut seeds, &boxes) {
            for (builder_name, builder) in BUILDERS {
                let tree = build_with(builder, &boxes);
                let there = refitted(&tree, &moved).expect("refits");
                let back = refitted(&there, &boxes).expect("refits back");
                assert_eq!(
                    tree_words(&back),
                    tree_words(&tree),
                    "{builder_name} {corpus} {motion}"
                );
            }
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_044_an_empty_tree_refits_only_to_no_boxes() {
    let empty = Tree { nodes: Vec::new(), prim_indices: Vec::new() };
    assert_eq!(refitted(&empty, &[]).map(|tree| tree.nodes.len()), Ok(0));
    assert_eq!(
        refusal(&empty, &[unit(0)]),
        RefitError::PrimitiveCountMismatch { listed: 0, boxes: 1 }
    );
    let tree = small_tree();
    let after = refitted(&tree, &small_boxes()).expect("the hand-built tree refits");
    assert_refitted(&after, &small_boxes(), "small tree");
}

// ---- Refusals, each on its boundary ----

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_045_a_box_count_other_than_the_listed_slots_is_refused() {
    let tree = small_tree();
    let boxes = small_boxes();
    assert!(refitted(&tree, &boxes).is_ok());
    let short = refusal(&tree, &boxes[..3]);
    assert_eq!(short, RefitError::PrimitiveCountMismatch { listed: 4, boxes: 3 });
    let mut long = boxes.clone();
    long.push(unit(20));
    assert_eq!(refusal(&tree, &long), RefitError::PrimitiveCountMismatch { listed: 4, boxes: 5 });
    // A built tree: n - 1 and n + 1 boxes are refused.
    let scattered = refit_support::scattered(&mut Seeds(REFIT_SEED ^ 0x45), 50);
    let built = build_with(build_bvh_sah, &scattered);
    let (fewer, mut more) = (scattered[..49].to_vec(), scattered.clone());
    more.push(unit(0));
    assert_eq!(
        refusal(&built, &fewer),
        RefitError::PrimitiveCountMismatch { listed: 50, boxes: 49 }
    );
    assert_eq!(
        refusal(&built, &more),
        RefitError::PrimitiveCountMismatch { listed: 50, boxes: 51 }
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_046_children_outside_the_slice_are_refused() {
    let boxes = small_boxes();
    // Node 2's children at 3 and 4 are the last two nodes; one further is out of range.
    let mut tree = small_tree();
    tree.nodes[2].first_child_or_prim_idx = 4;
    assert_eq!(refusal(&tree, &boxes), RefitError::InvalidNodeIndex { node: 2 });
    tree.nodes[2].first_child_or_prim_idx = u32::MAX;
    assert_eq!(refusal(&tree, &boxes), RefitError::InvalidNodeIndex { node: 2 });
    // The same tree truncated by one node.
    let mut short = small_tree();
    short.nodes.pop();
    assert_eq!(refusal(&short, &boxes), RefitError::InvalidNodeIndex { node: 2 });
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_047_a_child_at_or_before_its_parent_is_refused() {
    let boxes = small_boxes();
    // Node 2 -> (3, 4) is accepted; (2, 3) names itself and (1, 2) an earlier node.
    for (first, refused) in [(3, false), (2, true), (1, true), (0, true)] {
        let mut tree = small_tree();
        tree.nodes[2].first_child_or_prim_idx = first;
        if refused {
            assert_eq!(refusal(&tree, &boxes), RefitError::ChildNotAfterParent { node: 2 });
        } else {
            assert!(refitted(&tree, &boxes).is_ok());
        }
    }
    // A built tree passed with its unused stale nodes: the first stale node names node 0.
    // Identical boxes: halved down to leaves of three or four.
    let scattered = refit_support::identical(30);
    let mut nodes = vec![common::empty_node(); 2 * scattered.len() - 1];
    let mut prim_indices = vec![0_u32; scattered.len()];
    let used = build_bvh_sah(&mut nodes, &mut prim_indices, &scattered).expect("fits");
    assert!(used < nodes.len(), "the SAH tree has leaves of several primitives");
    let stale = Tree { nodes, prim_indices };
    assert_eq!(refusal(&stale, &scattered), RefitError::ChildNotAfterParent { node: used });
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_048_a_leaf_range_outside_the_index_buffer_is_refused() {
    let boxes = small_boxes();
    // Leaf 4 lists slot 3, the last; ending one slot later is refused.
    let mut tree = small_tree();
    tree.nodes[4].prim_count = 2;
    assert_eq!(refusal(&tree, &boxes), RefitError::InvalidPrimitiveRange { node: 4 });
    tree.nodes[4] = leaf(4, 1, unit(0));
    assert_eq!(refusal(&tree, &boxes), RefitError::InvalidPrimitiveRange { node: 4 });
    // A sum that overflows a 32-bit `usize`.
    tree.nodes[4] = leaf(u32::MAX, u32::MAX, unit(0));
    assert_eq!(refusal(&tree, &boxes), RefitError::InvalidPrimitiveRange { node: 4 });
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_049_a_primitive_index_outside_the_boxes_is_refused() {
    let boxes = small_boxes();
    // Slot 1 lists primitive 0; 3 (the last box) is accepted, 4 and u32::MAX are refused.
    for (primitive, refused) in [(3, false), (4, true), (u32::MAX, true)] {
        let mut tree = small_tree();
        tree.prim_indices[1] = primitive;
        if refused {
            assert_eq!(refusal(&tree, &boxes), RefitError::InvalidPrimitiveIndex { node: 1 });
        } else {
            // A repeated primitive is not detected: the post-condition still holds.
            let after = refitted(&tree, &boxes).expect("indices in range");
            assert_refitted(&after, &boxes, "repeated primitive");
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_050_a_leaf_count_other_than_interior_plus_one_is_refused() {
    let mut boxes = small_boxes();
    // An extra leaf after the tree: four leaves for two interior nodes.
    let mut extra = small_tree();
    extra.nodes.push(leaf(0, 1, unit(0)));
    extra.prim_indices.push(0);
    boxes.push(unit(30));
    assert_eq!(refusal(&extra, &boxes), RefitError::NotATree { interior: 2, leaves: 4 });
    // The count is not a proof: node 2 sharing node 1's pair, with two unreachable leaves, has
    // the right count and is refitted node by node.
    let any = unit(0);
    let shared = Tree {
        nodes: vec![
            interior(1),
            interior(3),
            interior(3),
            leaf(0, 1, any),
            leaf(1, 1, any),
            leaf(2, 1, any),
            leaf(3, 1, any),
        ],
        prim_indices: vec![0, 1, 2, 3],
    };
    assert!(refitted(&shared, &small_boxes()).is_ok());
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_051_the_lowest_faulty_node_and_structure_before_counts_are_reported() {
    let boxes = small_boxes();
    let mut tree = small_tree();
    tree.nodes[4].prim_count = 9;
    tree.nodes[2].first_child_or_prim_idx = 2;
    assert_eq!(refusal(&tree, &boxes), RefitError::ChildNotAfterParent { node: 2 });
    // A structural fault before a count fault; a count fault before a primitive index fault.
    let mut both = small_tree();
    both.nodes[4] = leaf(9, 1, unit(0));
    assert_eq!(refusal(&both, &boxes[..2]), RefitError::InvalidPrimitiveRange { node: 4 });
    let mut index = small_tree();
    index.prim_indices[0] = 77;
    assert_eq!(
        refusal(&index, &boxes[..2]),
        RefitError::PrimitiveCountMismatch { listed: 4, boxes: 2 }
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_052_a_refused_refit_of_a_built_tree_writes_nothing() {
    let scattered = refit_support::scattered(&mut Seeds(REFIT_SEED ^ 0x52), 120);
    let moved: Vec<FixedAabb3> = scattered.iter().rev().copied().collect();
    for (builder_name, builder) in BUILDERS {
        let tree = build_with(builder, &scattered);
        let lowest_leaf = tree.nodes.iter().position(|node| node.prim_count > 0).expect("a leaf");
        let mut faults = Vec::new();
        let mut bad_root = tree.clone();
        bad_root.nodes[0].first_child_or_prim_idx = u32::MAX;
        faults.push((bad_root, RefitError::InvalidNodeIndex { node: 0 }));
        let mut bad_leaf = tree.clone();
        bad_leaf.nodes[lowest_leaf].prim_count = u32::MAX;
        faults.push((bad_leaf, RefitError::InvalidPrimitiveRange { node: lowest_leaf }));
        let mut bad_index = tree.clone();
        let slot = bad_index.nodes[lowest_leaf].first_child_or_prim_idx as usize;
        bad_index.prim_indices[slot] = 120;
        faults.push((bad_index, RefitError::InvalidPrimitiveIndex { node: lowest_leaf }));
        for (faulty, expected) in faults {
            assert_eq!(refusal(&faulty, &moved), expected, "{builder_name}");
        }
        assert_eq!(
            refusal(&tree, &moved[1..]),
            RefitError::PrimitiveCountMismatch { listed: 120, boxes: 119 }
        );
    }
}
