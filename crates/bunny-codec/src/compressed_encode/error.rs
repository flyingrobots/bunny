use std::fmt;

/// Error returned when a mesh cannot be encoded as a Bunny compressed mesh profile v1 stream.
///
/// Each variant is a mesh `decode_compressed_mesh` would refuse, or a buffer too short to hold
/// the stream; the encoder refuses it rather than write bytes the decoder rejects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompressedMeshEncodeError {
    /// The vertex count is 0 or above 1 000 000.
    InvalidVertexCount {
        /// The refused vertex count.
        count: usize,
    },
    /// The triangle count is 0 or above 1 000 000.
    InvalidTriangleCount {
        /// The refused triangle count.
        count: usize,
    },
    /// The triangles use 16-bit indices and there are more than 65 536 vertices.
    Width16VertexCapacity {
        /// The refused vertex count.
        vertex_count: usize,
    },
    /// The bounds' min is above its max on at least one axis.
    InvalidBounds,
    /// A triangle index is not below the vertex count.
    IndexOutOfBounds {
        /// The position of the first triangle with such an index.
        triangle: usize,
        /// Its first such index (`v0`, then `v1`, then `v2`).
        index: u32,
    },
    /// The output buffer is shorter than the stream.
    OutputTooSmall {
        /// The stream's length in bytes.
        required: usize,
        /// The buffer's length in bytes.
        provided: usize,
    },
    /// The stream's length does not fit `usize`. It is at most 18 000 076 bytes, so this needs a
    /// `usize` narrower than 25 bits, which no `std` target has.
    LengthOverflow,
}

impl fmt::Display for CompressedMeshEncodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::InvalidVertexCount { count } => {
                write!(formatter, "compressed mesh vertex count {count} is not 1 to 1000000")
            }
            Self::InvalidTriangleCount { count } => {
                write!(formatter, "compressed mesh triangle count {count} is not 1 to 1000000")
            }
            Self::Width16VertexCapacity { vertex_count } => write!(
                formatter,
                "compressed mesh width-16 indices cannot address {vertex_count} vertices (max 65536)"
            ),
            Self::InvalidBounds => formatter.write_str("compressed mesh bounds are inverted"),
            Self::IndexOutOfBounds { triangle, index } => write!(
                formatter,
                "compressed mesh triangle {triangle} has index {index} outside the vertices"
            ),
            Self::OutputTooSmall { required, provided } => write!(
                formatter,
                "compressed mesh needs {required} output bytes but {provided} were provided"
            ),
            Self::LengthOverflow => formatter.write_str("compressed mesh length overflows usize"),
        }
    }
}

impl std::error::Error for CompressedMeshEncodeError {}
