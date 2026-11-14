//! Caller-chosen limits for the ASCII PLY reader.

use std::fmt;

use super::body::{is_blank, read_triangle, read_vertex, Lines};
use super::header::parse_header;
use super::{split_header, AsciiPlyError, AsciiPlyErrorKind, AsciiPlyMesh};

/// Which limit of [`AsciiPlyLimits`] an input went past.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AsciiPlyLimit {
    /// The declared vertex count (`AsciiPlyLimits::max_vertices`).
    Vertices,
    /// The declared face count (`AsciiPlyLimits::max_faces`).
    Faces,
    /// The length of a line in bytes (`AsciiPlyLimits::max_line_bytes`).
    LineBytes,
}

impl AsciiPlyLimit {
    const fn name(self) -> &'static str {
        match self {
            Self::Vertices => "vertex count",
            Self::Faces => "face count",
            Self::LineBytes => "line length in bytes",
        }
    }
}

/// Limits for [`parse_ascii_ply_with_limits`].
///
/// Each limit is inclusive: a value equal to it is accepted, one more is refused.
/// [`parse_ascii_ply`](super::parse_ascii_ply) applies no limit of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct AsciiPlyLimits {
    /// The most vertices the header may declare.
    pub max_vertices: usize,
    /// The most faces the header may declare.
    pub max_faces: usize,
    /// The longest line, in bytes without its `\n` or `\r\n` terminator, in the header or the
    /// body, blank lines included.
    pub max_line_bytes: usize,
}

impl AsciiPlyLimits {
    /// `2^24` vertices, `2^24` faces and 4096 bytes per line.
    pub const DEFAULT: Self = Self::new(1 << 24, 1 << 24, 1 << 12);

    /// No limit: every value is `usize::MAX`. Counts and lines are then bounded only by the
    /// input itself, which is what `parse_ascii_ply` accepts.
    pub const UNLIMITED: Self = Self::new(usize::MAX, usize::MAX, usize::MAX);

    /// Limits with the given maxima.
    #[must_use]
    pub const fn new(max_vertices: usize, max_faces: usize, max_line_bytes: usize) -> Self {
        Self { max_vertices, max_faces, max_line_bytes }
    }
}

impl Default for AsciiPlyLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// An input went past one of its [`AsciiPlyLimits`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AsciiPlyLimitExceeded {
    limit: AsciiPlyLimit,
    maximum: usize,
    observed: usize,
    line: usize,
}

impl AsciiPlyLimitExceeded {
    /// Which limit was exceeded.
    #[must_use]
    pub const fn limit(self) -> AsciiPlyLimit {
        self.limit
    }

    /// The configured maximum.
    #[must_use]
    pub const fn maximum(self) -> usize {
        self.maximum
    }

    /// The value the input has: the declared count, or the full length of the first line
    /// over the limit.
    #[must_use]
    pub const fn observed(self) -> usize {
        self.observed
    }

    /// The 1-based line: the `element` line for a count, the long line for a length.
    #[must_use]
    pub const fn line(self) -> usize {
        self.line
    }
}

impl fmt::Display for AsciiPlyLimitExceeded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "line {}: PLY {} is {}, over the limit of {}",
            self.line,
            self.limit.name(),
            self.observed,
            self.maximum
        )
    }
}

/// Why [`parse_ascii_ply_with_limits`] refused an input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AsciiPlyLimitedError {
    /// A limit was exceeded.
    Limit(AsciiPlyLimitExceeded),
    /// The input does not match the profile; the error `parse_ascii_ply` gives.
    Format(AsciiPlyError),
}

impl From<AsciiPlyError> for AsciiPlyLimitedError {
    fn from(error: AsciiPlyError) -> Self {
        Self::Format(error)
    }
}

impl From<AsciiPlyLimitExceeded> for AsciiPlyLimitedError {
    fn from(error: AsciiPlyLimitExceeded) -> Self {
        Self::Limit(error)
    }
}

impl fmt::Display for AsciiPlyLimitedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Limit(error) => error.fmt(f),
            Self::Format(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for AsciiPlyLimitedError {}

/// Parses an ASCII PLY triangle mesh like [`parse_ascii_ply`](super::parse_ascii_ply), and
/// refuses one that goes past `limits`.
///
/// The result for an input within the limits is the one `parse_ascii_ply` gives, and an input
/// that profile refuses gives the same [`AsciiPlyError`] unless a limit applies first. The
/// order of the checks, which decides the error when several apply:
///
/// 1. the header terminator and UTF-8 (`MissingHeaderEnd`, `HeaderUtf8`);
/// 2. the length of every header line, `end_header` included;
/// 3. the rest of the header;
/// 4. the declared vertex count, then the declared face count, before any body line is read;
/// 5. each body line in turn, blank lines included: its length, then its record.
///
/// # Errors
/// [`AsciiPlyLimitedError::Limit`] with the limit, the maximum, the observed value and the
/// line, or [`AsciiPlyLimitedError::Format`] with the profile error.
pub fn parse_ascii_ply_with_limits<'a>(
    input: &'a [u8],
    limits: &AsciiPlyLimits,
) -> Result<AsciiPlyMesh<'a>, AsciiPlyLimitedError> {
    let (header, end_line, body_start) = split_header(input)?;
    let mut header_lines = Lines::new(input.get(..body_start).unwrap_or_default(), 1);
    while next_checked(&mut header_lines, limits.max_line_bytes, false)?.is_some() {}
    let spec = parse_header(header, end_line)?;
    count_within(
        AsciiPlyLimit::Vertices,
        spec.vertex_count,
        limits.max_vertices,
        spec.vertex_line,
    )?;
    count_within(AsciiPlyLimit::Faces, spec.face_count, limits.max_faces, spec.face_line)?;
    let mut lines = Lines::new(input.get(body_start..).unwrap_or_default(), end_line + 1);
    let vertex_lines = lines;
    for _ in 0..spec.vertex_count {
        let (number, line) = next_record_or_short(&mut lines, limits.max_line_bytes)?;
        read_vertex(line, number, &spec)?;
    }
    let face_lines = lines;
    for _ in 0..spec.face_count {
        let (number, line) = next_record_or_short(&mut lines, limits.max_line_bytes)?;
        read_triangle(line, number, spec.vertex_count)?;
    }
    if let Some((number, _)) = next_checked(&mut lines, limits.max_line_bytes, true)? {
        return Err(AsciiPlyError::new(AsciiPlyErrorKind::TrailingData, number).into());
    }
    Ok(AsciiPlyMesh { spec, vertex_lines, face_lines })
}

const fn count_within(
    limit: AsciiPlyLimit,
    observed: usize,
    maximum: usize,
    line: usize,
) -> Result<(), AsciiPlyLimitExceeded> {
    if observed > maximum {
        Err(AsciiPlyLimitExceeded { limit, maximum, observed, line })
    } else {
        Ok(())
    }
}

/// The next line, checked against the length limit; with `records_only`, blank lines are
/// skipped (after checking them). `None` at the end of the input.
fn next_checked<'a>(
    lines: &mut Lines<'a>,
    max_line_bytes: usize,
    records_only: bool,
) -> Result<Option<(usize, &'a [u8])>, AsciiPlyLimitExceeded> {
    while let Some((number, line)) = lines.next_line() {
        count_within(AsciiPlyLimit::LineBytes, line.len(), max_line_bytes, number)?;
        if !records_only || !is_blank(line) {
            return Ok(Some((number, line)));
        }
    }
    Ok(None)
}

fn next_record_or_short<'a>(
    lines: &mut Lines<'a>,
    max_line_bytes: usize,
) -> Result<(usize, &'a [u8]), AsciiPlyLimitedError> {
    let record = next_checked(lines, max_line_bytes, true)?;
    record.ok_or_else(|| lines.missing_record(AsciiPlyErrorKind::PayloadTooShort).into())
}
