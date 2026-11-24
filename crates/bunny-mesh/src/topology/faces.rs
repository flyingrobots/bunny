//! Exact and flipped duplicate faces, found by sorting face keys in caller scratch.

use super::finding::Sink;
use super::scratch::{FaceMark, FaceSlot};
use super::triangles::distinct_triangles;
use super::TopologyFinding;
use crate::IndexBufferLayout;

/// Rotates three distinct indices so the smallest is first, keeping the cyclic order.
const fn rotate_smallest_first([a, b, c]: [u32; 3]) -> [u32; 3] {
    if a < b && a < c {
        [a, b, c]
    } else if b < c {
        [b, c, a]
    } else {
        [c, a, b]
    }
}

/// The face key: the rotated key `[m, x, y]` as its ascending set `[m, min, max]` and
/// whether `x > y`. Two faces have equal rotated keys exactly when their sets and parities
/// are equal, and opposite cyclic orders exactly when only the parities differ.
const fn slot(triangle: u32, corners: [u32; 3]) -> FaceSlot {
    let [m, x, y] = rotate_smallest_first(corners);
    let odd = x > y;
    let set = if odd { [m, y, x] } else { [m, x, y] };
    FaceSlot { set, odd, triangle, first: triangle, mark: FaceMark::Unique }
}

fn fill(indices: IndexBufferLayout<'_>, slots: &mut [FaceSlot]) -> usize {
    let mut used = 0;
    for (target, (triangle, corners)) in slots.iter_mut().zip(distinct_triangles(indices)) {
        *target = slot(triangle, corners);
        used += 1;
    }
    used
}

/// Marks every face of one index-set group against the group's lowest-indexed face.
fn mark(group: &mut [FaceSlot]) {
    let Some((first, rest)) = group.split_first_mut() else { return };
    for later in rest {
        later.first = first.triangle;
        later.mark = if later.odd == first.odd { FaceMark::Exact } else { FaceMark::Flipped };
    }
}

fn emit(slots: &[FaceSlot], mark: FaceMark, sink: &mut Sink<'_>) {
    for face in slots.iter().filter(|face| face.mark == mark) {
        let (triangle, first) = (face.triangle, face.first);
        sink.push(match mark {
            FaceMark::Flipped => TopologyFinding::FlippedDuplicateFace { triangle, first },
            _ => TopologyFinding::DuplicateFace { triangle, first },
        });
    }
}

/// Sorts the keys by (set, triangle), marks each group, re-sorts by triangle and emits the
/// exact duplicates, then the flipped ones. Keys are unique (the triangle index is in them),
/// so the unstable sort has one result. Cost: two sorts of `n` slots, `O(n log n)`.
pub(super) fn duplicates(
    indices: IndexBufferLayout<'_>,
    slots: &mut [FaceSlot],
    sink: &mut Sink<'_>,
) {
    let used = fill(indices, slots);
    let Some(slots) = slots.get_mut(..used) else { return };
    slots.sort_unstable_by_key(|face| (face.set, face.triangle));
    for group in slots.chunk_by_mut(|a, b| a.set == b.set) {
        mark(group);
    }
    slots.sort_unstable_by_key(|face| face.triangle);
    emit(slots, FaceMark::Exact, sink);
    emit(slots, FaceMark::Flipped, sink);
}
