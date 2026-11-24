//! Caller-provided scratch storage for `validate_topology`.

/// One face-scratch slot. Its contents on entry are never read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FaceSlot {
    /// The three corner indices, ascending.
    pub(super) set: [u32; 3],
    /// Whether the face's cyclic order is the reverse of `set`'s.
    pub(super) odd: bool,
    pub(super) triangle: u32,
    /// The lowest-indexed face with the same `set`.
    pub(super) first: u32,
    pub(super) mark: FaceMark,
}

/// What a face is relative to the first face of its index set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum FaceMark {
    #[default]
    Unique,
    Exact,
    Flipped,
}

impl FaceSlot {
    /// A slot to fill scratch arrays with.
    pub const EMPTY: Self =
        Self { set: [0; 3], odd: false, triangle: 0, first: 0, mark: FaceMark::Unique };
}

/// One edge-scratch slot. Its contents on entry are never read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EdgeSlot {
    pub(super) lo: u32,
    pub(super) hi: u32,
    pub(super) triangle: u32,
    /// Whether the face runs from `lo` to `hi`.
    pub(super) forward: bool,
}

impl EdgeSlot {
    /// A slot to fill scratch arrays with.
    pub const EMPTY: Self = Self { lo: 0, hi: 0, triangle: 0, forward: false };
}

/// The scratch storage of [`validate_topology`](super::validate_topology).
///
/// `faces` needs at least [`face_scratch_len`] slots and `edges` at least
/// [`edge_scratch_len`] for the triangle count. Longer slices are fine; only the prefix is
/// used. Whatever the slices hold on entry, the findings are the same.
#[derive(Debug)]
pub struct TopologyScratch<'a> {
    faces: &'a mut [FaceSlot],
    edges: &'a mut [EdgeSlot],
}

impl<'a> TopologyScratch<'a> {
    /// Wraps the two scratch slices.
    #[must_use]
    pub const fn new(faces: &'a mut [FaceSlot], edges: &'a mut [EdgeSlot]) -> Self {
        Self { faces, edges }
    }

    pub(super) const fn into_parts(self) -> (&'a mut [FaceSlot], &'a mut [EdgeSlot]) {
        (self.faces, self.edges)
    }
}

/// The face-scratch slots needed for `triangle_count` triangles: one per triangle.
#[must_use]
pub const fn face_scratch_len(triangle_count: usize) -> usize {
    triangle_count
}

/// The edge-scratch slots needed for `triangle_count` triangles: three per triangle, or
/// `None` when that overflows `usize`.
#[must_use]
pub const fn edge_scratch_len(triangle_count: usize) -> Option<usize> {
    triangle_count.checked_mul(3)
}
