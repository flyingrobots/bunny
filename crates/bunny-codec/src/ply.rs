use std::fmt;
use std::str;

use bunny_mesh::Triangle32;

mod header;

use header::parse_header;

const VERTEX_STRIDE: usize = 12;
const TRIANGLE_FACE_STRIDE: usize = 13;

/// Error returned when a binary PLY mesh cannot be parsed as the Bunny mesh profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlyError {
    /// The `end_header` marker was not found.
    MissingHeaderEnd,
    /// The header is not valid UTF-8 text.
    HeaderUtf8,
    /// The first header line is not `ply`.
    InvalidMagic,
    /// The `format` line is absent or is not `binary_little_endian 1.0`.
    UnsupportedFormat,
    /// An element count is missing or not a non-negative decimal integer.
    InvalidCount,
    /// The header does not declare the canonical vertex element.
    MissingVertexElement,
    /// The header does not declare the canonical face element.
    MissingFaceElement,
    /// The header declares an unsupported non-empty element.
    UnsupportedElement,
    /// The header declares an unsupported property layout.
    UnsupportedProperty,
    /// The binary payload is shorter than the declared mesh layout.
    PayloadTooShort,
    /// The binary payload contains bytes after the declared mesh layout.
    TrailingData,
    /// A face list entry is not a triangle.
    NonTriangularFace,
    /// A face index is negative and cannot be represented as `u32`.
    NegativeIndex,
    /// A declared count or offset overflowed `usize`.
    IntegerOverflow,
}

impl fmt::Display for PlyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::MissingHeaderEnd => "PLY header terminator was not found",
            Self::HeaderUtf8 => "PLY header is not valid UTF-8",
            Self::InvalidMagic => "PLY header does not start with ply",
            Self::UnsupportedFormat => "PLY format must be binary_little_endian 1.0",
            Self::InvalidCount => "PLY element count is invalid",
            Self::MissingVertexElement => "PLY vertex element is missing or incomplete",
            Self::MissingFaceElement => "PLY face element is missing or incomplete",
            Self::UnsupportedElement => "PLY declares an unsupported non-empty element",
            Self::UnsupportedProperty => "PLY property layout is unsupported",
            Self::PayloadTooShort => "PLY binary payload is shorter than declared",
            Self::TrailingData => "PLY binary payload has trailing bytes",
            Self::NonTriangularFace => "PLY face list entry is not a triangle",
            Self::NegativeIndex => "PLY face index is negative",
            Self::IntegerOverflow => "PLY count or offset overflowed usize",
        };
        f.write_str(message)
    }
}

impl std::error::Error for PlyError {}

/// A borrowed binary PLY mesh with canonical `float x/y/z` vertices and triangle faces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlyBinaryMesh<'a> {
    vertex_bytes: &'a [u8],
    face_bytes: &'a [u8],
    vertex_count: usize,
    face_count: usize,
}

impl<'a> PlyBinaryMesh<'a> {
    /// Returns the number of vertices declared by the PLY file.
    #[must_use]
    pub const fn vertex_count(self) -> usize {
        self.vertex_count
    }

    /// Returns the number of triangle faces declared by the PLY file.
    #[must_use]
    pub const fn face_count(self) -> usize {
        self.face_count
    }

    /// Returns the borrowed binary vertex payload.
    #[must_use]
    pub const fn vertex_bytes(self) -> &'a [u8] {
        self.vertex_bytes
    }

    /// Returns the borrowed binary face payload.
    #[must_use]
    pub const fn face_bytes(self) -> &'a [u8] {
        self.face_bytes
    }

    /// Reads a vertex from the borrowed payload.
    ///
    /// # Errors
    /// Returns `PlyError::PayloadTooShort` if the requested vertex is out of range.
    pub fn vertex(self, index: usize) -> Result<PlyVertex, PlyError> {
        let start = checked_offset(index, VERTEX_STRIDE)?;
        let bytes = take(self.vertex_bytes, start, VERTEX_STRIDE)?;
        Ok(PlyVertex {
            x: f32::from_le_bytes(take_array(bytes, 0)?),
            y: f32::from_le_bytes(take_array(bytes, 4)?),
            z: f32::from_le_bytes(take_array(bytes, 8)?),
        })
    }

    /// Reads a triangle face from the borrowed payload.
    ///
    /// # Errors
    /// Returns a `PlyError` if the face is out of range, is not triangular, or
    /// contains negative signed PLY indices.
    pub fn triangle(self, index: usize) -> Result<Triangle32, PlyError> {
        let start = checked_offset(index, TRIANGLE_FACE_STRIDE)?;
        let bytes = take(self.face_bytes, start, TRIANGLE_FACE_STRIDE)?;
        read_triangle(bytes)
    }
}

/// A PLY vertex decoded from borrowed little-endian float bytes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlyVertex {
    /// X coordinate.
    pub x: f32,
    /// Y coordinate.
    pub y: f32,
    /// Z coordinate.
    pub z: f32,
}

/// Parses a canonical binary little-endian PLY mesh as a zero-copy borrowed view.
///
/// The accepted profile is `float x`, `float y`, `float z` vertices and
/// `property list uchar int vertex_indices` triangle faces.
///
/// # Errors
/// Returns `PlyError` when the header or binary payload does not match the
/// canonical Bunny mesh PLY profile.
pub fn parse_binary_ply(input: &[u8]) -> Result<PlyBinaryMesh<'_>, PlyError> {
    let (header, payload_start) = split_header(input)?;
    let spec = parse_header(header)?;
    let vertex_len = checked_offset(spec.vertex_count, VERTEX_STRIDE)?;
    let face_len = checked_offset(spec.face_count, TRIANGLE_FACE_STRIDE)?;
    let face_start = payload_start
        .checked_add(vertex_len)
        .ok_or(PlyError::IntegerOverflow)?;
    let payload_end = face_start
        .checked_add(face_len)
        .ok_or(PlyError::IntegerOverflow)?;
    if input.len() < payload_end {
        return Err(PlyError::PayloadTooShort);
    }
    if input.len() != payload_end {
        return Err(PlyError::TrailingData);
    }

    let vertex_bytes = take(input, payload_start, vertex_len)?;
    let face_bytes = take(input, face_start, face_len)?;
    validate_faces(face_bytes, spec.face_count)?;

    Ok(PlyBinaryMesh {
        vertex_bytes,
        face_bytes,
        vertex_count: spec.vertex_count,
        face_count: spec.face_count,
    })
}

fn split_header(input: &[u8]) -> Result<(&str, usize), PlyError> {
    let (header_end, payload_start) = header_bounds(input)?;
    let header = input.get(..header_end).ok_or(PlyError::MissingHeaderEnd)?;
    str::from_utf8(header)
        .map(|header| (header, payload_start))
        .map_err(|_| PlyError::HeaderUtf8)
}

fn header_bounds(input: &[u8]) -> Result<(usize, usize), PlyError> {
    let lf = find_bytes(input, b"end_header\n").map(|pos| (pos, pos + 11));
    let crlf = find_bytes(input, b"end_header\r\n").map(|pos| (pos, pos + 12));
    match (lf, crlf) {
        (Some(a), Some(b)) => Ok(if a.0 <= b.0 { a } else { b }),
        (Some(bounds), None) | (None, Some(bounds)) => Ok(bounds),
        (None, None) => Err(PlyError::MissingHeaderEnd),
    }
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn checked_offset(index: usize, stride: usize) -> Result<usize, PlyError> {
    index.checked_mul(stride).ok_or(PlyError::IntegerOverflow)
}

fn take(input: &[u8], start: usize, len: usize) -> Result<&[u8], PlyError> {
    let end = start.checked_add(len).ok_or(PlyError::IntegerOverflow)?;
    input.get(start..end).ok_or(PlyError::PayloadTooShort)
}

fn take_array<const N: usize>(input: &[u8], start: usize) -> Result<[u8; N], PlyError> {
    let slice = take(input, start, N)?;
    let mut bytes = [0_u8; N];
    bytes.copy_from_slice(slice);
    Ok(bytes)
}

fn validate_faces(face_bytes: &[u8], count: usize) -> Result<(), PlyError> {
    for index in 0..count {
        let start = checked_offset(index, TRIANGLE_FACE_STRIDE)?;
        read_triangle(take(face_bytes, start, TRIANGLE_FACE_STRIDE)?)?;
    }
    Ok(())
}

fn read_triangle(bytes: &[u8]) -> Result<Triangle32, PlyError> {
    if take(bytes, 0, 1)?.first().copied() != Some(3) {
        return Err(PlyError::NonTriangularFace);
    }
    Ok(Triangle32::new(
        read_index(bytes, 1)?,
        read_index(bytes, 5)?,
        read_index(bytes, 9)?,
    ))
}

fn read_index(bytes: &[u8], start: usize) -> Result<u32, PlyError> {
    let value = i32::from_le_bytes(take_array(bytes, start)?);
    u32::try_from(value).map_err(|_| PlyError::NegativeIndex)
}
