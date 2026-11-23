//! `try_build_bvh_sah`: the typed buffer refusals, agreement with `build_bvh_sah`, and the
//! primitive-index buffer rule (flyingrobots/bunny#445).
//!
//! The oracle is the buffer rule written out here from the three lengths (`2n - 1` nodes,
//! `n` indices); it shares no helper with the builder. The seeded cases build the same input
//! into equal buffers with both forms and compare the result and every word of both buffers.

mod common;

use core::error::Error;

use bunny_broadphase::{build_bvh_sah, try_build_bvh_sah, BvhBuildError, BvhNode};
use bunny_geom::FixedAabb3;
use common::{aabb, Seeds, ONE};
use wasm_bindgen_test::wasm_bindgen_test;

/// The seed of the generated cases; printed in every failure message.
const BUFFER_SEED: u64 = 0xB002_0445_0000_00B0;
const BUFFER_CASES: usize = 4_000;
/// A primitive index no build writes: every entry a build writes is below `n`.
const SENTINEL: u32 = 0xDEAD_BEEF;

fn stale_node() -> BvhNode {
    BvhNode { bounds: aabb([7; 3], [9; 3]), first_child_or_prim_idx: 77, prim_count: 5 }
}

fn node_words(node: &BvhNode) -> [i64; 8] {
    let b = node.bounds;
    [
        b.min.x.raw(),
        b.min.y.raw(),
        b.min.z.raw(),
        b.max.x.raw(),
        b.max.y.raw(),
        b.max.z.raw(),
        i64::from(node.first_child_or_prim_idx),
        i64::from(node.prim_count),
    ]
}

fn words(nodes: &[BvhNode]) -> Vec<[i64; 8]> {
    nodes.iter().map(node_words).collect()
}

/// `count` boxes from the seeds, some identical, some flat.
fn boxes(seeds: &mut Seeds, count: usize) -> Vec<FixedAabb3> {
    (0..count)
        .map(|_| {
            let corner = [0; 3].map(|_| seeds.signed(40 * ONE));
            let size = [0; 3].map(|_| seeds.signed(3 * ONE).abs() * i64::from(seeds.next() % 4 != 0));
            aabb(corner, [corner[0] + size[0], corner[1] + size[1], corner[2] + size[2]])
        })
        .collect()
}

/// What the oracle expects of `try_build_bvh_sah`, from the lengths alone.
fn buffer_rule(nodes: usize, indices: usize, primitives: usize) -> Option<BvhBuildError> {
    if primitives == 0 {
        None
    } else if nodes < 2 * primitives - 1 {
        Some(BvhBuildError::NodeBufferTooSmall { nodes, primitives })
    } else if indices < primitives {
        Some(BvhBuildError::IndexBufferTooSmall { indices, primitives })
    } else {
        None
    }
}

/// Builds `primitives` into fresh buffers of the given lengths with both forms and checks the
/// result against the oracle, the two forms against each other, and the buffer rule.
fn check(primitives: &[FixedAabb3], nodes_len: usize, indices_len: usize, context: &str) -> bool {
    let n = primitives.len();
    let (mut try_nodes, mut try_indices) = (vec![stale_node(); nodes_len], vec![SENTINEL; indices_len]);
    let (mut old_nodes, mut old_indices) = (try_nodes.clone(), try_indices.clone());
    let result = try_build_bvh_sah(&mut try_nodes, &mut try_indices, primitives);
    let old = build_bvh_sah(&mut old_nodes, &mut old_indices, primitives);
    assert_eq!(result.err(), buffer_rule(nodes_len, indices_len, n), "{context}");
    assert_eq!(old, result.ok(), "{context}: Option form differs");
    assert_eq!(words(&try_nodes), words(&old_nodes), "{context}: node buffers differ");
    assert_eq!(try_indices, old_indices, "{context}: index buffers differ");
    match result {
        Ok(used) => {
            let mut written: Vec<u32> = try_indices[..n].to_vec();
            written.sort_unstable();
            let all: Vec<u32> = (0..u32::try_from(n).expect("small")).collect();
            assert_eq!(written, all, "{context}: the first n entries are 0..n, each once");
            assert!(try_indices[n..].iter().all(|&e| e == SENTINEL), "{context}: tail untouched");
            assert!(
                words(&try_nodes[used..]).iter().all(|w| *w == node_words(&stale_node())),
                "{context}: unused nodes untouched"
            );
            true
        }
        Err(_) => {
            assert!(try_indices.iter().all(|&e| e == SENTINEL), "{context}: indices untouched");
            assert!(
                words(&try_nodes).iter().all(|w| *w == node_words(&stale_node())),
                "{context}: nodes untouched"
            );
            false
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_029_a_node_buffer_below_two_n_minus_one_is_named() {
    let primitives = boxes(&mut Seeds(BUFFER_SEED ^ 1), 5);
    assert!(!check(&primitives, 8, 5, "2n - 2 nodes"));
    assert!(check(&primitives, 9, 5, "2n - 1 nodes"));
    assert!(check(&primitives, 10, 5, "2n nodes"));
    assert!(!check(&primitives[..1], 0, 1, "one primitive, no nodes"));
    assert!(check(&primitives[..1], 1, 1, "one primitive, one node"));
    let mut nodes = vec![stale_node(); 8];
    let refused = try_build_bvh_sah(&mut nodes, &mut [0; 5], &primitives);
    assert_eq!(refused, Err(BvhBuildError::NodeBufferTooSmall { nodes: 8, primitives: 5 }));
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_030_an_index_buffer_below_n_is_named_after_the_node_buffer() {
    let primitives = boxes(&mut Seeds(BUFFER_SEED ^ 2), 5);
    assert!(!check(&primitives, 9, 4, "n - 1 indices"));
    assert!(check(&primitives, 9, 5, "n indices"));
    assert!(check(&primitives, 9, 6, "n + 1 indices"));
    // Both too small: the node buffer is named.
    assert!(!check(&primitives, 8, 4, "both one short"));
    let both = try_build_bvh_sah(&mut [stale_node(); 8], &mut [0; 4], &primitives);
    assert_eq!(both, Err(BvhBuildError::NodeBufferTooSmall { nodes: 8, primitives: 5 }));
    let indices = try_build_bvh_sah(&mut [stale_node(); 9], &mut [0; 4], &primitives);
    assert_eq!(indices, Err(BvhBuildError::IndexBufferTooSmall { indices: 4, primitives: 5 }));
    // No primitives: any buffers, even empty ones, give zero nodes.
    assert_eq!(try_build_bvh_sah(&mut [], &mut [], &[]), Ok(0));
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_032_option_and_result_forms_agree_on_generated_buffers() {
    let mut seeds = Seeds(BUFFER_SEED);
    let mut counts = [0_usize; 3];
    for case in 0..BUFFER_CASES {
        let n = usize::try_from(seeds.next() % 40).expect("small");
        let primitives = boxes(&mut seeds, n);
        let shift = |seeds: &mut Seeds, base: usize| {
            let offset = usize::try_from(seeds.next() % 5).expect("small");
            (base + offset).saturating_sub(2)
        };
        let nodes_len = shift(&mut seeds, (2 * n).saturating_sub(1));
        let indices_len = shift(&mut seeds, n);
        let context = format!("seed {BUFFER_SEED:#x}, case {case}: n {n}, {nodes_len}, {indices_len}");
        let slot = match buffer_rule(nodes_len, indices_len, n) {
            None => 0,
            Some(BvhBuildError::NodeBufferTooSmall { .. }) => 1,
            Some(_) => 2,
        };
        check(&primitives, nodes_len, indices_len, &context);
        counts[slot] += 1;
    }
    println!("seed {BUFFER_SEED:#x}: {BUFFER_CASES} builds, [built, nodes, indices] {counts:?}");
    assert!(counts.iter().all(|&n| n >= 500), "every outcome is reached often: {counts:?}");
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_033_the_index_buffer_holds_only_indices_of_the_input() {
    // Stale indices that name no primitive, buffers longer than needed, identical boxes.
    let mut primitives = boxes(&mut Seeds(BUFFER_SEED ^ 3), 30);
    primitives.extend([primitives[0]; 6]);
    assert!(check(&primitives, 2 * primitives.len() + 5, primitives.len() + 9, "roomy"));
    assert!(check(&primitives, 2 * primitives.len() - 1, primitives.len(), "exact"));
    let mut nodes = vec![stale_node(); 2 * primitives.len() - 1];
    let mut indices = vec![SENTINEL; primitives.len()];
    let used = try_build_bvh_sah(&mut nodes, &mut indices, &primitives).expect("fits");
    let n = u32::try_from(primitives.len()).expect("small");
    for node in nodes[..used].iter().filter(|node| node.prim_count > 0) {
        let first = usize::try_from(node.first_child_or_prim_idx).expect("u32");
        let count = usize::try_from(node.prim_count).expect("u32");
        assert!(indices[first..first + count].iter().all(|&index| index < n), "a leaf names an input");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_034_buffer_refusals_display_their_lengths_and_are_errors() {
    let nodes = BvhBuildError::NodeBufferTooSmall { nodes: 8, primitives: 5 };
    assert_eq!(nodes.to_string(), "node buffer of 8 is below 2 * 5 - 1 nodes");
    let indices = BvhBuildError::IndexBufferTooSmall { indices: 4, primitives: 5 };
    assert_eq!(indices.to_string(), "primitive-index buffer of 4 is below 5 entries");
    let many = BvhBuildError::TooManyPrimitives { primitives: (1 << 31) + 1 };
    assert_eq!(many.to_string(), "2147483649 primitives exceed the 2147483648 a u32 tree indexes");
    let internal = BvhBuildError::InternalCheckFailed;
    assert_eq!(internal.to_string(), "an internal check of the build failed");
    let errors: [&dyn Error; 4] = [&nodes, &indices, &many, &internal];
    assert!(errors.iter().all(|e| e.source().is_none()), "no underlying cause");
}
