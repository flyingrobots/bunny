#![deny(unsafe_code)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(missing_docs)]

//! Bounding volume hierarchy (BVH) and spatial broadphase query solvers.

use bunny_geom::{FixedAabb3, FixedRay3};
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;

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

/// Traverses the BVH to find primitives that overlap with a query AABB.
pub fn intersect_aabb<F>(
    nodes: &[BvhNode],
    prim_indices: &[u32],
    query_box: &FixedAabb3,
    mut overlap_leaf: F,
) where
    F: FnMut(u32),
{
    if nodes.is_empty() {
        return;
    }

    let mut stack = [0_u32; 64];
    let mut stack_ptr = 0;

    stack[stack_ptr] = 0;
    stack_ptr += 1;

    while stack_ptr > 0 {
        stack_ptr -= 1;
        let node_idx = stack[stack_ptr] as usize;
        let node = &nodes[node_idx];

        if !aabbs_overlap(&node.bounds, query_box) {
            continue;
        }

        if node.prim_count > 0 {
            let start = node.first_child_or_prim_idx as usize;
            let count = node.prim_count as usize;
            for i in 0..count {
                overlap_leaf(prim_indices[start + i]);
            }
        } else {
            let left_child = node.first_child_or_prim_idx;
            let right_child = left_child + 1;

            if stack_ptr + 2 > 64 {
                continue;
            }

            stack[stack_ptr] = left_child;
            stack_ptr += 1;
            stack[stack_ptr] = right_child;
            stack_ptr += 1;
        }
    }
}

/// Traverses the BVH to find primitives intersected by a ray.
pub fn intersect_ray<F>(
    nodes: &[BvhNode],
    prim_indices: &[u32],
    ray: &FixedRay3,
    mut intersect_leaf: F,
) where
    F: FnMut(u32),
{
    if nodes.is_empty() {
        return;
    }

    let mut stack = [0_u32; 64];
    let mut stack_ptr = 0;

    stack[stack_ptr] = 0;
    stack_ptr += 1;

    while stack_ptr > 0 {
        stack_ptr -= 1;
        let node_idx = stack[stack_ptr] as usize;
        let node = &nodes[node_idx];

        if bunny_query::ray_intersects_aabb(ray, &node.bounds).is_none() {
            continue;
        }

        if node.prim_count > 0 {
            let start = node.first_child_or_prim_idx as usize;
            let count = node.prim_count as usize;
            for i in 0..count {
                intersect_leaf(prim_indices[start + i]);
            }
        } else {
            let left_child = node.first_child_or_prim_idx;
            let right_child = left_child + 1;

            if stack_ptr + 2 > 64 {
                continue;
            }

            stack[stack_ptr] = left_child;
            stack_ptr += 1;
            stack[stack_ptr] = right_child;
            stack_ptr += 1;
        }
    }
}

fn union_aabb(a: FixedAabb3, b: FixedAabb3) -> FixedAabb3 {
    FixedAabb3::new(
        FixedVec3::new(
            std::cmp::min(a.min.x, b.min.x),
            std::cmp::min(a.min.y, b.min.y),
            std::cmp::min(a.min.z, b.min.z),
        ),
        FixedVec3::new(
            std::cmp::max(a.max.x, b.max.x),
            std::cmp::max(a.max.y, b.max.y),
            std::cmp::max(a.max.z, b.max.z),
        ),
    )
}

fn surface_area(aabb: &FixedAabb3) -> FixedQ32_32 {
    let size_x = if aabb.max.x > aabb.min.x {
        aabb.max.x - aabb.min.x
    } else {
        FixedQ32_32::ZERO
    };
    let size_y = if aabb.max.y > aabb.min.y {
        aabb.max.y - aabb.min.y
    } else {
        FixedQ32_32::ZERO
    };
    let size_z = if aabb.max.z > aabb.min.z {
        aabb.max.z - aabb.min.z
    } else {
        FixedQ32_32::ZERO
    };

    let two = FixedQ32_32::from_f32(2.0);
    two * (size_x * size_y + size_y * size_z + size_z * size_x)
}

fn get_centroid(aabb: &FixedAabb3) -> FixedVec3 {
    let two = FixedQ32_32::from_f32(2.0);
    FixedVec3::new(
        (aabb.min.x + aabb.max.x)
            .checked_div(two)
            .unwrap_or(FixedQ32_32::ZERO),
        (aabb.min.y + aabb.max.y)
            .checked_div(two)
            .unwrap_or(FixedQ32_32::ZERO),
        (aabb.min.z + aabb.max.z)
            .checked_div(two)
            .unwrap_or(FixedQ32_32::ZERO),
    )
}

const fn get_axis_val(v: &FixedVec3, axis: usize) -> FixedQ32_32 {
    match axis {
        0 => v.x,
        1 => v.y,
        _ => v.z,
    }
}

fn min_vec(a: FixedVec3, b: FixedVec3) -> FixedVec3 {
    FixedVec3::new(
        std::cmp::min(a.x, b.x),
        std::cmp::min(a.y, b.y),
        std::cmp::min(a.z, b.z),
    )
}

fn max_vec(a: FixedVec3, b: FixedVec3) -> FixedVec3 {
    FixedVec3::new(
        std::cmp::max(a.x, b.x),
        std::cmp::max(a.y, b.y),
        std::cmp::max(a.z, b.z),
    )
}

fn aabbs_overlap(a: &FixedAabb3, b: &FixedAabb3) -> bool {
    a.min.x <= b.max.x
        && a.max.x >= b.min.x
        && a.min.y <= b.max.y
        && a.max.y >= b.min.y
        && a.min.z <= b.max.z
        && a.max.z >= b.min.z
}
