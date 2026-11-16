use bunny_mesh::Triangle32;
use bunny_num::FixedQ32_32;

mod body;
mod decimal;
mod error;
mod header;
mod limits;

use body::{read_texture_coordinate, read_triangle, read_vertex, trim_blank, Lines};
pub use decimal::{
    parse_decimal_q32_32, DecimalError, DECIMAL_Q32_32_ID, MAX_FRACTIONAL_SIGNIFICANT_DIGITS,
};
pub use error::{AsciiPlyError, AsciiPlyErrorKind};
use header::{parse_header, HeaderSpec, Profile};
pub use limits::{
    parse_ascii_ply_v2_with_limits, parse_ascii_ply_with_limits, AsciiPlyLimit,
    AsciiPlyLimitExceeded, AsciiPlyLimitedError, AsciiPlyLimits,
};

/// Stable identity of the ASCII PLY triangle-mesh profile read by [`parse_ascii_ply`].
pub const ASCII_PLY_PROFILE_ID: &str = "bunny-ascii-ply-triangle-mesh/1";

/// Stable identity of the ASCII PLY triangle-mesh profile with texture coordinates read
/// by [`parse_ascii_ply_v2`].
pub const ASCII_PLY_PROFILE_V2_ID: &str = "bunny-ascii-ply-triangle-mesh/2";

/// A PLY vertex position in Q32.32, converted exactly from decimal text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AsciiPlyVertex {
    /// X coordinate.
    pub x: FixedQ32_32,
    /// Y coordinate.
    pub y: FixedQ32_32,
    /// Z coordinate.
    pub z: FixedQ32_32,
}

/// A PLY texture coordinate in Q32.32, converted exactly from decimal text.
///
/// `u` is read from the vertex property `s` or `u`, and `v` from `t` or `v`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AsciiPlyTextureCoordinate {
    /// First texture coordinate (`s` or `u`).
    pub u: FixedQ32_32,
    /// Second texture coordinate (`t` or `v`).
    pub v: FixedQ32_32,
}

/// A borrowed, fully validated ASCII PLY triangle mesh.
///
/// `parse_ascii_ply` reads every record once to validate it. The view keeps the
/// borrowed text and re-reads it on iteration, so it holds no allocation.
#[derive(Clone, Copy, Debug)]
pub struct AsciiPlyMesh<'a> {
    spec: HeaderSpec,
    vertex_lines: Lines<'a>,
    face_lines: Lines<'a>,
}

impl<'a> AsciiPlyMesh<'a> {
    /// Returns the number of vertices declared by the PLY file.
    #[must_use]
    pub const fn vertex_count(&self) -> usize {
        self.spec.vertex_count
    }

    /// Returns the number of triangle faces declared by the PLY file.
    #[must_use]
    pub const fn face_count(&self) -> usize {
        self.spec.face_count
    }

    /// Returns a forward iterator over the vertex positions.
    ///
    /// The iterator does not allocate. Each item is `Ok` because the mesh was
    /// validated when it was parsed.
    #[must_use]
    pub const fn vertices(&self) -> AsciiPlyVertices<'a> {
        AsciiPlyVertices {
            lines: self.vertex_lines,
            spec: self.spec,
            remaining: self.spec.vertex_count,
        }
    }

    /// Returns whether the mesh has texture coordinates: read by [`parse_ascii_ply_v2`]
    /// from a vertex element that declares `s` and `t`, or `u` and `v`. Always false
    /// under [`parse_ascii_ply`].
    #[must_use]
    pub const fn has_texture_coordinates(&self) -> bool {
        self.spec.texture_slots.is_some()
    }

    /// Returns a forward iterator over the per-vertex texture coordinates, in vertex
    /// order, or `None` when the file declares none.
    ///
    /// The iterator does not allocate. Each item is `Ok` because the mesh was
    /// validated when it was parsed.
    #[must_use]
    pub const fn texture_coordinates(&self) -> Option<AsciiPlyTextureCoordinates<'a>> {
        if self.spec.texture_slots.is_none() {
            return None;
        }
        Some(AsciiPlyTextureCoordinates {
            lines: self.vertex_lines,
            spec: self.spec,
            remaining: self.spec.vertex_count,
        })
    }

    /// Returns a forward iterator over the triangles.
    ///
    /// The iterator does not allocate. Each item is `Ok` because the mesh was
    /// validated when it was parsed.
    #[must_use]
    pub const fn triangles(&self) -> AsciiPlyTriangles<'a> {
        AsciiPlyTriangles {
            lines: self.face_lines,
            vertex_count: self.spec.vertex_count,
            remaining: self.spec.face_count,
        }
    }
}

/// Forward iterator over the vertices of an [`AsciiPlyMesh`].
#[derive(Clone, Debug)]
pub struct AsciiPlyVertices<'a> {
    lines: Lines<'a>,
    spec: HeaderSpec,
    remaining: usize,
}

impl Iterator for AsciiPlyVertices<'_> {
    type Item = Result<AsciiPlyVertex, AsciiPlyError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        let (number, line) = self.lines.next_record()?;
        Some(read_vertex(line, number, &self.spec))
    }
}

/// Forward iterator over the texture coordinates of an [`AsciiPlyMesh`].
#[derive(Clone, Debug)]
pub struct AsciiPlyTextureCoordinates<'a> {
    lines: Lines<'a>,
    spec: HeaderSpec,
    remaining: usize,
}

impl Iterator for AsciiPlyTextureCoordinates<'_> {
    type Item = Result<AsciiPlyTextureCoordinate, AsciiPlyError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        let (number, line) = self.lines.next_record()?;
        read_texture_coordinate(line, number, &self.spec).transpose()
    }
}

/// Forward iterator over the triangles of an [`AsciiPlyMesh`].
#[derive(Clone, Debug)]
pub struct AsciiPlyTriangles<'a> {
    lines: Lines<'a>,
    vertex_count: usize,
    remaining: usize,
}

impl Iterator for AsciiPlyTriangles<'_> {
    type Item = Result<Triangle32, AsciiPlyError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        let (number, line) = self.lines.next_record()?;
        Some(read_triangle(line, number, self.vertex_count))
    }
}

/// Parses an ASCII PLY triangle mesh (`bunny-ascii-ply-triangle-mesh/1`).
///
/// Coordinates are converted from decimal text to Q32.32 with the exact rule
/// `bunny-decimal-q32-32/1` (see [`parse_decimal_q32_32`]). The profile, the
/// line, whitespace and comment rules, and the error contract are in
/// `docs/topics/ascii-ply/README.md`.
///
/// # Errors
/// Returns an [`AsciiPlyError`], with the line number, when the header or body
/// does not match the profile.
pub fn parse_ascii_ply(input: &[u8]) -> Result<AsciiPlyMesh<'_>, AsciiPlyError> {
    parse_with(input, Profile::V1)
}

/// Parses an ASCII PLY triangle mesh with texture coordinates
/// (`bunny-ascii-ply-triangle-mesh/2`).
///
/// The profile is that of [`parse_ascii_ply`], except that a vertex element may declare
/// one pair of texture coordinates, `s` and `t` or `u` and `v`, of a float type. They are
/// converted by `bunny-decimal-q32-32/1` and returned by
/// [`AsciiPlyMesh::texture_coordinates`]. A file with none of the four names reads to the
/// same mesh as under [`parse_ascii_ply`].
///
/// # Errors
/// Returns what [`parse_ascii_ply`] returns, and [`AsciiPlyErrorKind::UnsupportedProperty`]
/// at the line of the last texture name declared for a lone name of a pair, names from
/// both pairs, a repeated name or a non-float type.
pub fn parse_ascii_ply_v2(input: &[u8]) -> Result<AsciiPlyMesh<'_>, AsciiPlyError> {
    parse_with(input, Profile::V2)
}

fn parse_with(input: &[u8], profile: Profile) -> Result<AsciiPlyMesh<'_>, AsciiPlyError> {
    let (header, end_line, body_start) = split_header(input)?;
    let spec = parse_header(header, end_line, profile)?;
    let mut lines = Lines::new(input.get(body_start..).unwrap_or_default(), end_line + 1);
    let vertex_lines = lines;
    for _ in 0..spec.vertex_count {
        let (number, line) = next_or_short(&mut lines)?;
        read_vertex(line, number, &spec)?;
    }
    let face_lines = lines;
    for _ in 0..spec.face_count {
        let (number, line) = next_or_short(&mut lines)?;
        read_triangle(line, number, spec.vertex_count)?;
    }
    lines.only_blank_lines_remain()?;
    Ok(AsciiPlyMesh { spec, vertex_lines, face_lines })
}

fn next_or_short<'a>(lines: &mut Lines<'a>) -> Result<(usize, &'a [u8]), AsciiPlyError> {
    lines.next_record().ok_or_else(|| lines.missing_record(AsciiPlyErrorKind::PayloadTooShort))
}

/// Splits off the header text, which ends before the `end_header` line. Returns the
/// text, the 1-based number of the `end_header` line, and the offset of the body.
/// The `end_header` line must end with `\n`.
fn split_header(input: &[u8]) -> Result<(&str, usize, usize), AsciiPlyError> {
    let mut lines = Lines::new(input, 1);
    while let Some((number, line)) = peek_line(&mut lines, input) {
        if trim_blank(line.1) == b"end_header" {
            let body_start = input.len() - lines.remaining();
            if input.get(body_start.wrapping_sub(1)) != Some(&b'\n') {
                break;
            }
            return header_parts(input, line.0, number, body_start);
        }
    }
    Err(lines.missing_record(AsciiPlyErrorKind::MissingHeaderEnd))
}

/// Advances one line and returns its number with its start offset and content.
fn peek_line<'a>(lines: &mut Lines<'a>, input: &[u8]) -> Option<(usize, (usize, &'a [u8]))> {
    let start = input.len() - lines.remaining();
    lines.next_line().map(|(number, content)| (number, (start, content)))
}

fn header_parts(
    input: &[u8],
    header_end: usize,
    end_line: usize,
    body_start: usize,
) -> Result<(&str, usize, usize), AsciiPlyError> {
    let text = input.get(..header_end).unwrap_or_default();
    let text = std::str::from_utf8(text).map_err(|error| {
        let valid = text.get(..error.valid_up_to()).unwrap_or_default();
        let line = valid.split(|byte| *byte == b'\n').count();
        AsciiPlyError::new(AsciiPlyErrorKind::HeaderUtf8, line)
    })?;
    Ok((text, end_line, body_start))
}
