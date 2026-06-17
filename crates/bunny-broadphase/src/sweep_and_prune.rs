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
    init_indices(prim_indices, n)?;
    let axis = choose_axis(primitives)?;
    sort_indices(prim_indices, primitives, n, axis)?;
    let input = SweepInput { prim_indices, primitives, n, axis };
    let pair_count = collect_pairs(pairs, input)?;
    sort_pairs(pairs, pair_count)?;
    Some(pair_count)
}

fn init_indices(prim_indices: &mut [u32], n: usize) -> Option<()> {
    if prim_indices.len() < n || n > u32::MAX as usize {
        return None;
    }
    for (i, idx) in prim_indices.iter_mut().take(n).enumerate() {
        *idx = u32::try_from(i).ok()?;
    }
    Some(())
}

fn choose_axis(primitives: &[FixedAabb3]) -> Option<usize> {
    let first = primitives.first()?;
    let mut centroid_min = get_centroid(first);
    let mut centroid_max = centroid_min;
    for prim in primitives.iter().skip(1) {
        let centroid = get_centroid(prim);
        centroid_min = min_vec(centroid_min, centroid);
        centroid_max = max_vec(centroid_max, centroid);
    }
    Some(largest_span_axis(centroid_min, centroid_max))
}

fn largest_span_axis(
    centroid_min: bunny_linalg::FixedVec3,
    centroid_max: bunny_linalg::FixedVec3,
) -> usize {
    let span_x = centroid_max.x - centroid_min.x;
    let span_y = centroid_max.y - centroid_min.y;
    let span_z = centroid_max.z - centroid_min.z;
    if span_x >= span_y && span_x >= span_z {
        0
    } else if span_y >= span_z {
        1
    } else {
        2
    }
}

fn sort_indices(
    prim_indices: &mut [u32],
    primitives: &[FixedAabb3],
    n: usize,
    axis: usize,
) -> Option<()> {
    let active = prim_indices.get_mut(..n)?;
    active.sort_unstable_by(|&a, &b| {
        let val_a = primitive_min_axis(primitives, a, axis);
        let val_b = primitive_min_axis(primitives, b, axis);
        val_a.cmp(&val_b).then_with(|| a.cmp(&b))
    });
    Some(())
}

fn primitive_min_axis(
    primitives: &[FixedAabb3],
    index: u32,
    axis: usize,
) -> bunny_num::FixedQ32_32 {
    usize::try_from(index)
        .ok()
        .and_then(|idx| primitives.get(idx))
        .map_or(bunny_num::FixedQ32_32::ZERO, |bounds| get_axis_val(&bounds.min, axis))
}

#[derive(Clone, Copy)]
struct SweepInput<'a> {
    prim_indices: &'a [u32],
    primitives: &'a [FixedAabb3],
    n: usize,
    axis: usize,
}

impl<'a> SweepInput<'a> {
    fn primitive_index(&self, i: usize) -> Option<u32> {
        self.prim_indices.get(i).copied()
    }

    fn remaining_indices(&self, i: usize) -> Option<&'a [u32]> {
        self.prim_indices.get((i + 1)..self.n)
    }
}

fn collect_pairs(pairs: &mut [(u32, u32)], input: SweepInput<'_>) -> Option<usize> {
    let mut collector = PairCollector { pairs, count: 0 };
    for i in 0..input.n {
        collect_pairs_for_index(&mut collector, input, i)?;
    }
    Some(collector.count)
}

struct PairCollector<'a> {
    pairs: &'a mut [(u32, u32)],
    count: usize,
}

impl PairCollector<'_> {
    fn push(&mut self, idx_a: u32, idx_b: u32) -> Option<()> {
        let pair = if idx_a < idx_b { (idx_a, idx_b) } else { (idx_b, idx_a) };
        *self.pairs.get_mut(self.count)? = pair;
        self.count += 1;
        Some(())
    }
}

fn collect_pairs_for_index(
    collector: &mut PairCollector<'_>,
    input: SweepInput<'_>,
    i: usize,
) -> Option<()> {
    let idx_a = input.primitive_index(i)?;
    let bounds_a = primitive_bounds(input.primitives, idx_a)?;
    for &idx_b in input.remaining_indices(i)? {
        let bounds_b = primitive_bounds(input.primitives, idx_b)?;
        if should_stop_sweep(bounds_a, bounds_b, input.axis) {
            break;
        }
        if aabbs_overlap(bounds_a, bounds_b) {
            collector.push(idx_a, idx_b)?;
        }
    }
    Some(())
}

fn primitive_bounds(primitives: &[FixedAabb3], index: u32) -> Option<&FixedAabb3> {
    usize::try_from(index).ok().and_then(|idx| primitives.get(idx))
}

fn should_stop_sweep(bounds_a: &FixedAabb3, bounds_b: &FixedAabb3, axis: usize) -> bool {
    get_axis_val(&bounds_b.min, axis) > get_axis_val(&bounds_a.max, axis)
}

fn sort_pairs(pairs: &mut [(u32, u32)], count: usize) -> Option<()> {
    pairs.get_mut(..count)?.sort_unstable_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    Some(())
}
