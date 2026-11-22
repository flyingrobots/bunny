use std::fmt;

use super::decimal::DecimalError;

/// The reason an ASCII PLY mesh was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AsciiPlyErrorKind {
    /// The `end_header` line was not found.
    MissingHeaderEnd,
    /// The header is not valid UTF-8 text.
    HeaderUtf8,
    /// The first header line is not `ply`.
    InvalidMagic,
    /// The `format` line is absent, repeated, late, or is not `ascii 1.0`.
    UnsupportedFormat,
    /// A header line starts with an unknown keyword or has the wrong shape.
    InvalidHeaderLine,
    /// An element count is missing or not a decimal integer.
    InvalidCount,
    /// The header does not declare a vertex element with `x`, `y` and `z`.
    MissingVertexElement,
    /// The header does not declare the face element with its index list.
    MissingFaceElement,
    /// The header declares an unsupported non-empty element, or one twice.
    UnsupportedElement,
    /// The header declares an unsupported property, type or layout.
    UnsupportedProperty,
    /// The body has fewer records than the header declares.
    PayloadTooShort,
    /// The body has content after the declared records.
    TrailingData,
    /// A vertex line does not have exactly one value per declared property.
    VertexFieldCount,
    /// A coordinate is not a decimal number.
    InvalidNumber,
    /// A coordinate does not fit the Q32.32 range after rounding.
    NumberOverflow,
    /// A coordinate has too many significant digits to convert exactly.
    NumberTooLong,
    /// A face list entry is not a triangle.
    NonTriangularFace,
    /// A face line does not hold exactly the declared count and indices.
    FaceFieldCount,
    /// A face index is not a decimal integer.
    InvalidIndex,
    /// A face index has a leading minus sign.
    NegativeIndex,
    /// A face references a vertex outside the vertex range.
    IndexOutOfBounds,
    /// A declared count or offset overflowed `usize`.
    IntegerOverflow,
}

impl AsciiPlyErrorKind {
    const fn message(self) -> &'static str {
        match self {
            Self::MissingHeaderEnd => "PLY header terminator was not found",
            Self::HeaderUtf8 => "PLY header is not valid UTF-8",
            Self::InvalidMagic => "PLY header does not start with ply",
            Self::UnsupportedFormat => "PLY format must be ascii 1.0",
            Self::InvalidHeaderLine => "PLY header line is not recognised",
            Self::InvalidCount => "PLY element count is invalid",
            Self::MissingVertexElement => "PLY vertex element is missing or incomplete",
            Self::MissingFaceElement => "PLY face element is missing or incomplete",
            Self::UnsupportedElement => "PLY declares an unsupported element",
            Self::UnsupportedProperty => "PLY property layout is unsupported",
            Self::PayloadTooShort => "PLY body has fewer records than declared",
            Self::TrailingData => "PLY body has content after the declared records",
            Self::VertexFieldCount => "PLY vertex line has the wrong number of values",
            Self::InvalidNumber => "PLY coordinate is not a decimal number",
            Self::NumberOverflow => "PLY coordinate does not fit Q32.32",
            Self::NumberTooLong => "PLY coordinate has too many digits to convert exactly",
            Self::NonTriangularFace => "PLY face list entry is not a triangle",
            Self::FaceFieldCount => "PLY face line has the wrong number of values",
            Self::InvalidIndex => "PLY face index is not a decimal integer",
            Self::NegativeIndex => "PLY face index is negative",
            Self::IndexOutOfBounds => "PLY face index is out of bounds",
            Self::IntegerOverflow => "PLY count or offset overflowed usize",
        }
    }
}

impl From<DecimalError> for AsciiPlyErrorKind {
    fn from(error: DecimalError) -> Self {
        match error {
            DecimalError::Invalid => Self::InvalidNumber,
            DecimalError::Overflow => Self::NumberOverflow,
            DecimalError::TooLong => Self::NumberTooLong,
        }
    }
}

/// Error returned when an ASCII PLY mesh cannot be read, with its line number.
///
/// `line` is 1-based and counts `\n` characters from the start of the input. A
/// problem that is a missing line (a missing `end_header`, too few records) is
/// reported at the line after the last line of the input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AsciiPlyError {
    kind: AsciiPlyErrorKind,
    line: usize,
}

impl AsciiPlyError {
    pub(super) const fn new(kind: AsciiPlyErrorKind, line: usize) -> Self {
        Self { kind, line }
    }

    /// Returns the reason for the refusal.
    #[must_use]
    pub const fn kind(self) -> AsciiPlyErrorKind {
        self.kind
    }

    /// Returns the 1-based line number where the problem was found.
    #[must_use]
    pub const fn line(self) -> usize {
        self.line
    }
}

impl fmt::Display for AsciiPlyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.kind.message())
    }
}

impl std::error::Error for AsciiPlyError {}
