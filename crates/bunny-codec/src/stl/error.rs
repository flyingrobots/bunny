use core::fmt;

/// Error returned when a binary STL file cannot be read as
/// `bunny-binary-stl-triangle-mesh/1`.
///
/// A coordinate refusal names the 0-based `triangle` in file order, the `vertex` (0, 1 or 2,
/// in stored order) and the `axis` (0, 1 or 2 for `x`, `y`, `z`) of the first bad coordinate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum StlError {
    /// The input is shorter than the 80-byte header and the 4-byte triangle count.
    TooShort {
        /// The input length in bytes.
        length: usize,
    },
    /// The input length is not exactly `84 + 50 * triangles`: truncated, a count that claims
    /// more or fewer triangles than are present, or trailing bytes.
    LengthMismatch {
        /// The triangle count read from bytes 80 to 83, little-endian.
        triangles: u32,
        /// `84 + 50 * triangles`, computed in `u64`.
        expected: u64,
        /// The input length.
        actual: u64,
    },
    /// The length matches, but the count is above [`super::BINARY_STL_MAX_TRIANGLES`], so
    /// its vertex indices would not fit `u32`.
    TooManyTriangles {
        /// The declared triangle count.
        triangles: u32,
    },
    /// A vertex coordinate is NaN or an infinity.
    NonFiniteVertex {
        /// 0-based triangle index in file order.
        triangle: u32,
        /// 0, 1 or 2: the vertex within the triangle, in stored order.
        vertex: u8,
        /// 0, 1 or 2: `x`, `y` or `z`.
        axis: u8,
    },
    /// A vertex coordinate is finite but outside the Q32.32 range after rounding.
    VertexOutOfRange {
        /// 0-based triangle index in file order.
        triangle: u32,
        /// 0, 1 or 2: the vertex within the triangle, in stored order.
        vertex: u8,
        /// 0, 1 or 2: `x`, `y` or `z`.
        axis: u8,
    },
}

const fn axis_name(axis: u8) -> &'static str {
    match axis {
        0 => "x",
        1 => "y",
        _ => "z",
    }
}

impl fmt::Display for StlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::TooShort { length } => write!(
                f,
                "STL input is {length} bytes, shorter than the 84-byte header and triangle count"
            ),
            Self::LengthMismatch { triangles, expected, actual } => write!(
                f,
                "STL declares {triangles} triangles, which needs exactly {expected} bytes; \
                 the input is {actual} bytes"
            ),
            Self::TooManyTriangles { triangles } => write!(
                f,
                "STL declares {triangles} triangles; at most {} have vertex indices that fit u32",
                super::BINARY_STL_MAX_TRIANGLES
            ),
            Self::NonFiniteVertex { triangle, vertex, axis } => write!(
                f,
                "STL triangle {triangle} vertex {vertex} coordinate {} is not finite",
                axis_name(axis)
            ),
            Self::VertexOutOfRange { triangle, vertex, axis } => write!(
                f,
                "STL triangle {triangle} vertex {vertex} coordinate {} is outside the Q32.32 range",
                axis_name(axis)
            ),
        }
    }
}

impl core::error::Error for StlError {}
