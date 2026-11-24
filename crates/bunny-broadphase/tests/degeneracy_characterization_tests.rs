//! CHARACTERIZATION tests for flyingrobots/bunny#116: what each public `bunny-broadphase`
//! builder, search and helper does TODAY with degenerate input.
//!
//! These tests pin current behaviour, whatever it is. They are not an endorsed contract: a
//! cell pinned here can be inconsistent with a neighbouring cell, and several are (the same
//! inverted box is a pair for `sweep_and_prune` and a refusal for `uniform_grid_pairs`;
//! `get_centroid` saturates at the ends of the raw range). Changing a pinned cell is a
//! behaviour change to a public function, which the repository owner decides (see
//! `docs/topics/degeneracy/README.md`); update the test and the inventory together.
//! `union_aabb` on inverted boxes, flyingrobots/bunny#477, is pinned by BOUNDS-TP-012 in
//! `union_aabb_tests.rs` and not repeated here.

use bunny_broadphase::utils::{aabbs_overlap, get_centroid, surface_area};
use bunny_broadphase::{
    build_bvh, build_bvh_sah, closest_hit, compact_bvh, intersect_aabb, refit_bvh, sweep_and_prune,
    uniform_grid_pairs, BvhNode, BvhView, CompactChild, CompactNode, GridCellSize, GridKey,
    UniformGridError,
};
use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use bunny_query::TraceRay;
use wasm_bindgen_test::wasm_bindgen_test;

const MAX: i64 = i64::MAX;
const MIN: i64 = i64::MIN;
const ONE: i64 = 1 << 32;

type Raw3 = [i64; 3];

fn q(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

fn v(raw: Raw3) -> FixedVec3 {
    FixedVec3::new(q(raw[0]), q(raw[1]), q(raw[2]))
}

fn raw(p: FixedVec3) -> Raw3 {
    [p.x.raw(), p.y.raw(), p.z.raw()]
}

fn aabb(min: Raw3, max: Raw3) -> FixedAabb3 {
    FixedAabb3::new(v(min), v(max))
}

fn empty_node() -> BvhNode {
    BvhNode { bounds: aabb([0; 3], [0; 3]), first_child_or_prim_idx: 0, prim_count: 0 }
}

/// `aabbs_overlap` reads an inverted box as given: it overlaps a box around it but not itself.
/// `surface_area` counts an inverted axis as zero extent, and saturates for the full range.
/// `get_centroid` sums before halving, so the centroid of a point box at `i64::MAX` saturates
/// to about `i64::MAX / 2`.
#[wasm_bindgen_test(unsupported = test)]
fn degen_tp_040_characterization_box_helpers() {
    let inverted = aabb([10, 0, 0], [0, 1, 1]);
    let around = aabb([-100; 3], [100; 3]);
    assert!(aabbs_overlap(&inverted, &around));
    assert!(!aabbs_overlap(&inverted, &inverted), "an inverted box does not overlap itself");
    let point = aabb([5; 3], [5; 3]);
    assert!(aabbs_overlap(&point, &point), "a point box overlaps itself");

    let inverted_x = aabb([2 * ONE, 0, 0], [0, ONE, ONE]);
    assert_eq!(surface_area(&inverted_x), q(2 * ONE), "x extent read as zero: 2 (0 + 1 + 0)");
    assert_eq!(surface_area(&aabb([MIN; 3], [MAX; 3])), q(MAX), "saturated");

    let top = aabb([MAX; 3], [MAX; 3]);
    assert_eq!(raw(get_centroid(&top)), [MAX / 2 + 1; 3], "saturated, not i64::MAX");
    assert_eq!(raw(get_centroid(&aabb([MIN; 3], [MIN; 3]))), [MIN / 2; 3], "saturated");
}

/// Empty input: both builders and `sweep_and_prune` return zero, `uniform_grid_pairs` an empty
/// complete report, `compact_bvh` no root, `refit_bvh` nothing to do, and the searches visit
/// nothing.
#[wasm_bindgen_test(unsupported = test)]
fn degen_tp_041_characterization_empty_input() {
    let mut nodes = [empty_node(); 1];
    let mut indices = [0_u32; 1];
    assert_eq!(build_bvh(&mut nodes, &mut indices, &[]), Some(0));
    assert_eq!(build_bvh_sah(&mut nodes, &mut indices, &[]), Some(0));
    let mut pairs = [(0_u32, 0_u32); 1];
    assert_eq!(sweep_and_prune(&mut pairs, &mut indices, &[]), Some(0));
    let cell = GridCellSize::try_from_raw(ONE).expect("positive");
    let report = uniform_grid_pairs(&[], cell, &mut [], &mut pairs).expect("empty is accepted");
    assert_eq!((report.total(), report.is_complete()), (0, true));

    let empty = aabb([0; 3], [0; 3]);
    let child = CompactChild { first: 0, prim_count: 0 };
    let mut out = [CompactNode { bounds: [empty; 2], children: [child; 2] }; 1];
    assert_eq!(compact_bvh(&[], &mut out), Ok(None));
    assert_eq!(refit_bvh(&mut [], &[], &[]), Ok(()));
    let mut visits = 0;
    assert_eq!(intersect_aabb(&[], &[], &aabb([MIN; 3], [MAX; 3]), |_| visits += 1), Ok(()));
    let ray = TraceRay::new(v([0; 3]), v([ONE, 0, 0])).expect("in the trace domain");
    let view = BvhView::new(&[], &[]);
    assert_eq!(closest_hit(view, &ray, q(ONE), |_, _| Some(q(0))), Ok(None));
    assert_eq!(visits, 0);
}

/// An inverted box is taken as given by `sweep_and_prune`, which pairs it with a box around
/// it, and by both builders, but refused by `uniform_grid_pairs`, whose documentation
/// promises the pairs of `sweep_and_prune`.
#[wasm_bindgen_test(unsupported = test)]
fn degen_tp_042_characterization_inverted_boxes_in_broadphase() {
    let boxes = [aabb([-100; 3], [100; 3]), aabb([10, 0, 0], [0, 1, 1])];
    let mut pairs = [(0_u32, 0_u32); 4];
    let mut indices = [0_u32; 2];
    assert_eq!(sweep_and_prune(&mut pairs, &mut indices, &boxes), Some(1));
    assert_eq!(pairs[0], (0, 1));

    let cell = GridCellSize::try_from_raw(64).expect("positive");
    let mut keys = [GridKey::new([0; 3], 0); 64];
    assert_eq!(
        uniform_grid_pairs(&boxes, cell, &mut keys, &mut pairs),
        Err(UniformGridError::InvertedBox { index: 1 })
    );

    let mut nodes = [empty_node(); 3];
    assert_eq!(build_bvh(&mut nodes, &mut indices, &boxes), Some(1), "one leaf of both");
    assert_eq!(nodes[0].bounds, boxes[0], "the root is the as-given union");
    assert_eq!(build_bvh_sah(&mut nodes, &mut indices, &boxes), Some(1));
    assert_eq!(nodes[0].bounds, boxes[0]);
    let mut found = [false; 2];
    let query = aabb([5, 0, 0], [5, 0, 0]);
    let leaf = &nodes[..1];
    assert_eq!(intersect_aabb(leaf, &indices, &query, |p| found[p as usize] = true), Ok(()));
    // The search reports every primitive of a leaf whose bounds meet the query; it does not
    // test the primitives themselves, so the inverted box is reported too.
    assert_eq!(found, [true, true]);
}

/// Zero-size boxes: two touching point boxes are a pair for both pair finders, and a cell size
/// of zero or below is refused.
#[wasm_bindgen_test(unsupported = test)]
fn degen_tp_043_characterization_zero_size_boxes_and_cells() {
    let boxes = [aabb([7; 3], [7; 3]), aabb([7; 3], [7; 3])];
    let mut pairs = [(0_u32, 0_u32); 2];
    let mut indices = [0_u32; 2];
    assert_eq!(sweep_and_prune(&mut pairs, &mut indices, &boxes), Some(1));
    let cell = GridCellSize::try_from_raw(1).expect("positive");
    let mut keys = [GridKey::new([0; 3], 0); 2];
    let report = uniform_grid_pairs(&boxes, cell, &mut keys, &mut pairs).expect("accepted");
    assert_eq!((report.total(), pairs[0]), (1, (0, 1)));
    assert_eq!(
        GridCellSize::try_from_raw(0),
        Err(UniformGridError::NonPositiveCellSize { raw: 0 })
    );
    assert_eq!(
        GridCellSize::try_new(q(-1)),
        Err(UniformGridError::NonPositiveCellSize { raw: -1 })
    );
}
