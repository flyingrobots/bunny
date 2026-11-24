//! The refusals of `validate_topology`.

use core::fmt;

/// Why [`validate_topology`](super::validate_topology) refused its input.
///
/// A refusal writes no record. Topology findings are never refusals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TopologyError {
    /// A corner index is not below the vertex count: `IndexBufferLayout::is_valid` is `false`.
    IndexOutOfRange,
    /// More than `u32::MAX` triangles, so a triangle index would not fit a record.
    TooManyTriangles {
        /// The triangle count of the index buffer.
        triangle_count: usize,
    },
    /// The face scratch is shorter than `face_scratch_len(triangle_count)`.
    FaceScratchTooSmall {
        /// The slots needed.
        required: usize,
        /// The slots given.
        provided: usize,
    },
    /// The edge scratch is shorter than `edge_scratch_len(triangle_count)`.
    EdgeScratchTooSmall {
        /// The slots needed.
        required: usize,
        /// The slots given.
        provided: usize,
    },
}

impl fmt::Display for TopologyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::IndexOutOfRange => f.write_str("a triangle index is out of range"),
            Self::TooManyTriangles { triangle_count } => {
                write!(f, "{triangle_count} triangles is more than u32::MAX")
            }
            Self::FaceScratchTooSmall { required, provided } => {
                write!(f, "face scratch has {provided} slots; {required} are required")
            }
            Self::EdgeScratchTooSmall { required, provided } => {
                write!(f, "edge scratch has {provided} slots; {required} are required")
            }
        }
    }
}

impl core::error::Error for TopologyError {}
