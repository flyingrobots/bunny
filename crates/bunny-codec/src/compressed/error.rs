use std::fmt;

/// Error returned when a Bunny compressed mesh byte stream is invalid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompressedMeshError {
    /// The magic bytes are not `BUNNYQZ!`.
    InvalidMagic,
    /// The profile version is not supported.
    UnsupportedVersion,
    /// Reserved header flags are non-zero.
    UnsupportedFlags,
    /// The triangle index width field is not canonical.
    InvalidIndexWidth,
    /// A vertex or triangle count is zero or exceeds the profile limit.
    InvalidCount,
    /// The encoded quantization bounds are inverted.
    InvalidBounds,
    /// The declared payload length does not match the canonical layout.
    InvalidPayloadLength,
    /// The input ends before the declared byte range.
    PayloadTooShort,
    /// The input has bytes after the declared payload.
    TrailingData,
    /// A requested record or triangle vertex index is out of bounds.
    IndexOutOfBounds,
    /// A checked integer conversion or offset calculation overflowed.
    IntegerOverflow,
}

impl fmt::Display for CompressedMeshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidMagic => "compressed mesh magic bytes are invalid",
            Self::UnsupportedVersion => "compressed mesh version is unsupported",
            Self::UnsupportedFlags => "compressed mesh reserved flags are non-zero",
            Self::InvalidIndexWidth => "compressed mesh index width is invalid",
            Self::InvalidCount => "compressed mesh count is invalid",
            Self::InvalidBounds => "compressed mesh quantization bounds are invalid",
            Self::InvalidPayloadLength => "compressed mesh payload length is invalid",
            Self::PayloadTooShort => "compressed mesh payload is shorter than declared",
            Self::TrailingData => "compressed mesh payload has trailing bytes",
            Self::IndexOutOfBounds => "compressed mesh index is out of bounds",
            Self::IntegerOverflow => "compressed mesh offset calculation overflowed",
        };
        f.write_str(message)
    }
}

impl std::error::Error for CompressedMeshError {}
