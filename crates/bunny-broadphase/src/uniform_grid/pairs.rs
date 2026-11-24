//! Pairs from runs of equal cells, each reported once, and the output sink.

use bunny_geom::FixedAabb3;

use super::cell::{CellRange, GridCellSize};
use super::key::GridKey;
use crate::utils::aabbs_overlap;

/// What [`uniform_grid_pairs`](super::uniform_grid_pairs) found and how much it wrote.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridPairReport {
    written: usize,
    total: u64,
    candidate_tests: u64,
}

impl GridPairReport {
    /// How many pairs were written to the front of the pair buffer.
    #[must_use]
    pub const fn written(&self) -> usize {
        self.written
    }

    /// How many overlapping pairs there are in all, written or not. At most
    /// `n (n - 1) / 2 < 2^63` for `n <= u32::MAX` boxes, so the count is exact.
    #[must_use]
    pub const fn total(&self) -> u64 {
        self.total
    }

    /// How many times two boxes were found in the same cell: one per cell they share, before
    /// the overlap test and the once-only rule. Saturates at `u64::MAX`.
    #[must_use]
    pub const fn candidate_tests(&self) -> u64 {
        self.candidate_tests
    }

    /// `true` when every pair was written: the pair buffer was long enough.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        u64::try_from(self.written).is_ok_and(|written| written == self.total)
    }
}

/// The boxes and the cell size the keys were made from.
#[derive(Clone, Copy)]
pub(super) struct Grid<'a> {
    pub(super) boxes: &'a [FixedAabb3],
    pub(super) cell_size: GridCellSize,
}

impl Grid<'_> {
    fn bounds(&self, key: &GridKey) -> Option<&FixedAabb3> {
        usize::try_from(key.object()).ok().and_then(|index| self.boxes.get(index))
    }

    /// The lowest cell both boxes occupy: on each axis the larger of the cells of their mins.
    /// Two overlapping boxes share every cell from it to the smaller of the cells of their
    /// maxes, so it is a cell both hold a key in.
    fn first_shared_cell(&self, a: &FixedAabb3, b: &FixedAabb3) -> Option<[i64; 3]> {
        let lo_a = CellRange::of(self.cell_size, a)?.lo();
        let lo_b = CellRange::of(self.cell_size, b)?.lo();
        let mut cell = lo_a;
        for (value, other) in cell.iter_mut().zip(lo_b) {
            *value = (*value).max(other);
        }
        Some(cell)
    }
}

/// Tests every two boxes in each run of keys with the same cell.
///
/// Within a run the box indices ascend, so the first key's box is the lower index. A pair is
/// reported when the boxes overlap (closed, as `sweep_and_prune` tests them) and the run's
/// cell is their first shared cell, which happens in exactly one run.
pub(super) fn collect(keys: &[GridKey], grid: Grid<'_>, sink: &mut PairSink<'_>) {
    for run in keys.chunk_by(|a, b| a.cell() == b.cell()) {
        collect_run(run, grid, sink);
    }
}

/// Tests every two keys of one run, the lower box index first.
fn collect_run(run: &[GridKey], grid: Grid<'_>, sink: &mut PairSink<'_>) {
    let mut rest = run;
    while let Some((first, tail)) = rest.split_first() {
        for other in tail {
            consider(first, other, grid, sink);
        }
        rest = tail;
    }
}

fn consider(a: &GridKey, b: &GridKey, grid: Grid<'_>, sink: &mut PairSink<'_>) {
    sink.candidate_tests = sink.candidate_tests.saturating_add(1);
    let (Some(box_a), Some(box_b)) = (grid.bounds(a), grid.bounds(b)) else { return };
    if aabbs_overlap(box_a, box_b) && grid.first_shared_cell(box_a, box_b) == Some(a.cell()) {
        sink.push((a.object(), b.object()));
    }
}

/// Keeps the smallest pairs that fit the buffer and counts them all.
///
/// While the buffer has room, pairs are appended to a max-heap on its written prefix; once it
/// is full, a pair below the heap's largest replaces it. At the end the prefix is sorted, so
/// it holds the first `written` pairs of the pair order whatever order they were found in.
pub(super) struct PairSink<'a> {
    out: &'a mut [(u32, u32)],
    written: usize,
    total: u64,
    candidate_tests: u64,
}

impl<'a> PairSink<'a> {
    pub(super) const fn new(out: &'a mut [(u32, u32)]) -> Self {
        Self { out, written: 0, total: 0, candidate_tests: 0 }
    }

    fn push(&mut self, pair: (u32, u32)) {
        // Each pair is reported once, so the total stays below 2^63 (see `total`).
        self.total += 1;
        if self.written < self.out.len() {
            self.append(pair);
        } else {
            self.replace_largest(pair);
        }
    }

    fn append(&mut self, pair: (u32, u32)) {
        if let Some(slot) = self.out.get_mut(self.written) {
            *slot = pair;
            self.written += 1;
        }
        if let Some(heap) = self.out.get_mut(..self.written) {
            sift_up(heap, self.written.saturating_sub(1));
        }
    }

    /// The buffer is full and is a max-heap: `pair` replaces the largest when it is smaller.
    fn replace_largest(&mut self, pair: (u32, u32)) {
        if let Some(root) = self.out.first_mut().filter(|largest| pair < **largest) {
            *root = pair;
            sift_down(self.out, 0);
        }
    }

    pub(super) fn finish(self) -> GridPairReport {
        if let Some(written) = self.out.get_mut(..self.written) {
            written.sort_unstable();
        }
        let (written, total) = (self.written, self.total);
        GridPairReport { written, total, candidate_tests: self.candidate_tests }
    }
}

/// Moves the entry at `index` up until its parent is larger.
fn sift_up(heap: &mut [(u32, u32)], mut index: usize) {
    while index > 0 {
        let parent = (index - 1) / 2;
        if heap.get(index) <= heap.get(parent) {
            return;
        }
        heap.swap(index, parent);
        index = parent;
    }
}

/// Moves the entry at `index` down until both children are smaller. A missing child is
/// `None`, which compares below every entry.
fn sift_down(heap: &mut [(u32, u32)], mut index: usize) {
    loop {
        let left = 2 * index + 1;
        let mut largest = index;
        for child in [left, left + 1] {
            if heap.get(child) > heap.get(largest) {
                largest = child;
            }
        }
        if largest == index {
            return;
        }
        heap.swap(index, largest);
        index = largest;
    }
}
