use std::fmt;

/// Error returned when an OBJ mesh cannot be parsed as the Bunny text profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjError {
    /// The OBJ source contains no vertex records.
    MissingVertices,
    /// The OBJ source contains no face records.
    MissingFaces,
    /// A numeric vertex coordinate could not be parsed.
    InvalidVertex,
    /// A vertex coordinate is NaN or infinity.
    NonFiniteVertex,
    /// A face is not a triangle.
    NonTriangularFace,
    /// A face index is zero, negative, relative, or not a valid integer.
    InvalidIndex,
    /// An accessor index or face vertex reference is outside the parsed range.
    IndexOutOfBounds,
    /// The OBJ statement is not part of the supported Bunny mesh profile.
    UnsupportedStatement,
}

impl fmt::Display for ObjError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::MissingVertices => "OBJ source contains no vertex records",
            Self::MissingFaces => "OBJ source contains no face records",
            Self::InvalidVertex => "OBJ vertex coordinate is invalid",
            Self::NonFiniteVertex => "OBJ vertex coordinate is not finite",
            Self::NonTriangularFace => "OBJ face is not triangular",
            Self::InvalidIndex => "OBJ face index is invalid",
            Self::IndexOutOfBounds => "OBJ index is out of bounds",
            Self::UnsupportedStatement => "OBJ statement is unsupported",
        };
        f.write_str(message)
    }
}

impl std::error::Error for ObjError {}
