//! Multi-axis Sweep-and-Prune collision solver.

use bunny_geom::FixedAabb3;

use crate::utils::{aabbs_overlap, get_axis_val, get_centroid, max_vec, min_vec};

/// Computes all overlapping pairs of AABBs using Sweep-and-Prune.
///
/// Returns the number of pairs found, or `None` if the output `pairs` buffer is too small.
/// Emitted pairs are guaranteed to be sorted such that `a < b` for each pair `(a, b)`,
/// and the list of pairs is sorted lexicographically.
#[must_use]
#[allow(clippy::cast_possible_wrap)]
#[allow(clippy::cast_possible_truncation)]
pub fn sweep_and_prune(
    pairs: &mut [(u32, u32)],
    prim_indices: &mut [u32],
    primitives: &[FixedAabb3],
) -> Option<usize> {
    if primitives.is_empty() {
        return Some(0);
    }

    let n = primitives.len();
    if prim_indices.len() < n {
        return None;
    }

    // Initialize indices
    for (i, idx) in prim_indices.iter_mut().enumerate() {
        *idx = i as u32;
    }

    // 1. Choose optimal axis with largest centroid span
    let mut centroid_min = get_centroid(&primitives[0]);
    let mut centroid_max = centroid_min;
    for prim in primitives.iter().skip(1) {
        let c = get_centroid(prim);
        centroid_min = min_vec(centroid_min, c);
        centroid_max = max_vec(centroid_max, c);
    }
    let span_x = centroid_max.x - centroid_min.x;
    let span_y = centroid_max.y - centroid_min.y;
    let span_z = centroid_max.z - centroid_min.z;

    let axis = if span_x >= span_y && span_x >= span_z {
        0
    } else if span_y >= span_z {
        1
    } else {
        2
    };

    // 2. Sort indices stably along the chosen axis using unstable sort with unique fallback
    prim_indices[..n].sort_unstable_by(|&a, &b| {
        let val_a = get_axis_val(&primitives[a as usize].min, axis);
        let val_b = get_axis_val(&primitives[b as usize].min, axis);
        val_a.cmp(&val_b).then_with(|| a.cmp(&b))
    });

    // 3. Sweep and check overlaps
    let mut pair_count = 0;
    for i in 0..n {
        let idx_a = prim_indices[i];
        let bounds_a = &primitives[idx_a as usize];

        for &idx_b in &prim_indices[(i + 1)..n] {
            let bounds_b = &primitives[idx_b as usize];

            // Prune if coordinates along the chosen axis no longer overlap
            let val_b_min = get_axis_val(&bounds_b.min, axis);
            let val_a_max = get_axis_val(&bounds_a.max, axis);
            if val_b_min > val_a_max {
                break;
            }

            if aabbs_overlap(bounds_a, bounds_b) {
                if pair_count >= pairs.len() {
                    return None;
                }

                let pair = if idx_a < idx_b {
                    (idx_a, idx_b)
                } else {
                    (idx_b, idx_a)
                };

                pairs[pair_count] = pair;
                pair_count += 1;
            }
        }
    }

    // 4. Stable sort the final pair output lexicographically
    pairs[..pair_count].sort_unstable_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

    Some(pair_count)
}
