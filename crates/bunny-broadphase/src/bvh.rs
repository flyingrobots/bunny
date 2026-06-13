//! Flat array-backed Bounding Volume Hierarchy (BVH).

use bunny_geom::FixedAabb3;
use bunny_num::FixedQ32_32;

use crate::utils::{get_axis_val, get_centroid, max_vec, min_vec, surface_area, union_aabb};

/// A node in the flat array-backed bounding volume hierarchy (BVH).
#[derive(Clone, Copy, Debug)]
pub struct BvhNode {
    /// Bounding box enclosing this node's children or primitives.
    pub bounds: FixedAabb3,
    /// Left child index (if interior node) or starting primitive index (if leaf).
    pub first_child_or_prim_idx: u32,
    /// Number of primitives in this node. If 0, this is an interior node.
    pub prim_count: u32,
}

/// Builds a flat BVH in-place into the provided node and primitive index buffers.
///
/// Returns the number of nodes actually used, or `None` if buffers are too small.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn build_bvh(
    nodes: &mut [BvhNode],
    prim_indices: &mut [u32],
    primitives: &[FixedAabb3],
) -> Option<usize> {
    if primitives.is_empty() {
        return Some(0);
    }

    let n = primitives.len();
    if nodes.len() < 2 * n - 1 || prim_indices.len() < n {
        return None;
    }

    for (i, idx) in prim_indices.iter_mut().enumerate() {
        *idx = i as u32;
    }

    let mut node_count = 1;
    build_recursive(nodes, &mut node_count, prim_indices, primitives, 0, 0, n);

    Some(node_count)
}

#[allow(clippy::too_many_lines)]
#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::cast_possible_wrap)]
#[allow(clippy::cast_lossless)]
fn build_recursive(
    nodes: &mut [BvhNode],
    node_count: &mut usize,
    prim_indices: &mut [u32],
    primitives: &[FixedAabb3],
    node_idx: usize,
    first_prim: usize,
    num_prims: usize,
) {
    let mut bounds = primitives[prim_indices[first_prim] as usize];
    for i in 1..num_prims {
        bounds = union_aabb(bounds, primitives[prim_indices[first_prim + i] as usize]);
    }

    if num_prims == 1 {
        nodes[node_idx] = BvhNode {
            bounds,
            first_child_or_prim_idx: first_prim as u32,
            prim_count: num_prims as u32,
        };
        return;
    }

    let mut best_axis = 0;
    let mut best_split = FixedQ32_32::ZERO;
    let mut best_cost = FixedQ32_32::from_raw(i64::MAX);

    let parent_sa = surface_area(&bounds);
    let mut centroid_min = get_centroid(&primitives[prim_indices[first_prim] as usize]);
    let mut centroid_max = centroid_min;
    for i in 1..num_prims {
        let c = get_centroid(&primitives[prim_indices[first_prim + i] as usize]);
        centroid_min = min_vec(centroid_min, c);
        centroid_max = max_vec(centroid_max, c);
    }

    for axis in 0..3 {
        let min_c = get_axis_val(&centroid_min, axis);
        let max_c = get_axis_val(&centroid_max, axis);

        if min_c == max_c {
            continue;
        }

        let range = max_c - min_c;
        let step = range
            .checked_div(FixedQ32_32::from_f32(4.0))
            .unwrap_or(FixedQ32_32::ZERO);
        if step == FixedQ32_32::ZERO {
            continue;
        }

        for k in 1..4 {
            let split_coord =
                min_c + step * FixedQ32_32::from_raw(k as i64 * FixedQ32_32::ONE.to_raw());
            let mut left_bounds = None;
            let mut right_bounds = None;
            let mut left_count = 0;
            let mut right_count = 0;

            for i in 0..num_prims {
                let prim_idx = prim_indices[first_prim + i] as usize;
                let prim = &primitives[prim_idx];
                let c = get_axis_val(&get_centroid(prim), axis);
                if c < split_coord {
                    left_bounds = Some(left_bounds.map_or(*prim, |lb| union_aabb(lb, *prim)));
                    left_count += 1;
                } else {
                    right_bounds = Some(right_bounds.map_or(*prim, |rb| union_aabb(rb, *prim)));
                    right_count += 1;
                }
            }

            if left_count > 0 && right_count > 0 {
                let sa_l = surface_area(&left_bounds.unwrap());
                let sa_r = surface_area(&right_bounds.unwrap());
                let cost = sa_l
                    * FixedQ32_32::from_raw(left_count as i64 * FixedQ32_32::ONE.to_raw())
                    + sa_r * FixedQ32_32::from_raw(right_count as i64 * FixedQ32_32::ONE.to_raw());

                if cost < best_cost {
                    best_cost = cost;
                    best_axis = axis;
                    best_split = split_coord;
                }
            }
        }
    }

    let leaf_cost = parent_sa * FixedQ32_32::from_raw(num_prims as i64 * FixedQ32_32::ONE.to_raw());

    if best_cost >= leaf_cost && num_prims <= 4 {
        nodes[node_idx] = BvhNode {
            bounds,
            first_child_or_prim_idx: first_prim as u32,
            prim_count: num_prims as u32,
        };
        return;
    }

    let mut i = first_prim;
    let mut j = first_prim + num_prims - 1;
    while i <= j {
        let prim_idx = prim_indices[i] as usize;
        let c = get_axis_val(&get_centroid(&primitives[prim_idx]), best_axis);
        if c < best_split {
            i += 1;
        } else {
            prim_indices.swap(i, j);
            if j == 0 {
                break;
            }
            j -= 1;
        }
    }

    let left_count = i - first_prim;
    if left_count == 0 || left_count == num_prims {
        nodes[node_idx] = BvhNode {
            bounds,
            first_child_or_prim_idx: first_prim as u32,
            prim_count: num_prims as u32,
        };
        return;
    }

    let left_child_idx = *node_count;
    *node_count += 2;

    nodes[node_idx] = BvhNode {
        bounds,
        first_child_or_prim_idx: left_child_idx as u32,
        prim_count: 0,
    };

    build_recursive(
        nodes,
        node_count,
        prim_indices,
        primitives,
        left_child_idx,
        first_prim,
        left_count,
    );
    build_recursive(
        nodes,
        node_count,
        prim_indices,
        primitives,
        left_child_idx + 1,
        first_prim + left_count,
        num_prims - left_count,
    );
}
