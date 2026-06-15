use std::fmt;

use bunny_mesh::Triangle32;

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
    /// A face references a vertex outside the parsed vertex range.
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
            Self::IndexOutOfBounds => "OBJ face index is out of bounds",
            Self::UnsupportedStatement => "OBJ statement is unsupported",
        };
        f.write_str(message)
    }
}

impl std::error::Error for ObjError {}

/// A borrowed OBJ mesh view over the original text input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjMesh<'a> {
    source: &'a str,
    vertex_count: usize,
    face_count: usize,
}

impl<'a> ObjMesh<'a> {
    /// Returns the borrowed OBJ source text.
    #[must_use]
    pub const fn source(self) -> &'a str {
        self.source
    }

    /// Returns the number of parsed `v` records.
    #[must_use]
    pub const fn vertex_count(self) -> usize {
        self.vertex_count
    }

    /// Returns the number of parsed triangular `f` records.
    #[must_use]
    pub const fn face_count(self) -> usize {
        self.face_count
    }

    /// Reads the requested vertex from the borrowed source.
    ///
    /// # Errors
    /// Returns `ObjError::InvalidVertex` if the index is out of range.
    pub fn vertex(self, index: usize) -> Result<ObjVertex, ObjError> {
        find_record(self.source, "v", index)
            .ok_or(ObjError::InvalidVertex)
            .and_then(parse_vertex_line)
    }

    /// Reads the requested triangle from the borrowed source.
    ///
    /// # Errors
    /// Returns an `ObjError` if the index is out of range or the face record is invalid.
    pub fn triangle(self, index: usize) -> Result<Triangle32, ObjError> {
        find_record(self.source, "f", index)
            .ok_or(ObjError::NonTriangularFace)
            .and_then(parse_face_line)
    }
}

/// A text OBJ vertex decoded from a borrowed `v` record.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ObjVertex {
    /// X coordinate.
    pub x: f32,
    /// Y coordinate.
    pub y: f32,
    /// Z coordinate.
    pub z: f32,
}

/// Parses a supported OBJ text mesh as a zero-copy borrowed view.
///
/// The supported profile accepts `v x y z` vertices, triangular `f` faces, and
/// common metadata statements. Face indices are converted from OBJ one-based
/// indexing into zero-based `Triangle32` values.
///
/// # Errors
/// Returns `ObjError` when the source contains unsupported statements,
/// malformed vertices, malformed faces, or out-of-range face indices.
pub fn parse_obj_text(source: &str) -> Result<ObjMesh<'_>, ObjError> {
    let counts = count_records(source)?;
    if counts.vertex_count == 0 {
        return Err(ObjError::MissingVertices);
    }
    if counts.face_count == 0 {
        return Err(ObjError::MissingFaces);
    }
    validate_face_indices(source, counts.vertex_count)?;
    Ok(ObjMesh {
        source,
        vertex_count: counts.vertex_count,
        face_count: counts.face_count,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ObjCounts {
    vertex_count: usize,
    face_count: usize,
}

fn count_records(source: &str) -> Result<ObjCounts, ObjError> {
    let mut vertex_count = 0;
    let mut face_count = 0;
    for line in source.lines() {
        match statement_kind(line)? {
            Some("v") => {
                parse_vertex_line(line)?;
                vertex_count += 1;
            }
            Some("f") => {
                parse_face_line(line)?;
                face_count += 1;
            }
            Some(_) | None => {}
        }
    }
    Ok(ObjCounts {
        vertex_count,
        face_count,
    })
}

fn validate_face_indices(source: &str, vertex_count: usize) -> Result<(), ObjError> {
    for line in source.lines() {
        if statement_kind(line)? == Some("f") {
            let triangle = parse_face_line(line)?;
            if !face_indices_are_valid(triangle, vertex_count) {
                return Err(ObjError::IndexOutOfBounds);
            }
        }
    }
    Ok(())
}

fn face_indices_are_valid(face: Triangle32, vertex_count: usize) -> bool {
    u32_index_is_valid(face.v0, vertex_count)
        && u32_index_is_valid(face.v1, vertex_count)
        && u32_index_is_valid(face.v2, vertex_count)
}

fn u32_index_is_valid(index: u32, vertex_count: usize) -> bool {
    usize::try_from(index).is_ok_and(|index| index < vertex_count)
}

fn statement_kind(line: &str) -> Result<Option<&str>, ObjError> {
    let mut parts = record_body(line).split_whitespace();
    let Some(kind) = parts.next() else {
        return Ok(None);
    };
    if kind.starts_with('#') || harmless_statement(kind) {
        Ok(None)
    } else if matches!(kind, "v" | "f") {
        Ok(Some(kind))
    } else {
        Err(ObjError::UnsupportedStatement)
    }
}

fn harmless_statement(kind: &str) -> bool {
    matches!(kind, "o" | "g" | "s" | "usemtl" | "mtllib" | "vt" | "vn")
}

fn find_record<'a>(source: &'a str, kind: &str, index: usize) -> Option<&'a str> {
    let mut found = 0;
    for line in source.lines() {
        if statement_kind(line).ok().flatten() == Some(kind) {
            if found == index {
                return Some(line);
            }
            found += 1;
        }
    }
    None
}

fn record_body(line: &str) -> &str {
    line.split_once('#')
        .map_or(line, |(record, _comment)| record)
        .trim()
}

fn parse_vertex_line(line: &str) -> Result<ObjVertex, ObjError> {
    let mut parts = record_body(line).split_whitespace();
    if parts.next() != Some("v") {
        return Err(ObjError::InvalidVertex);
    }
    let vertex = ObjVertex {
        x: parse_coord(parts.next())?,
        y: parse_coord(parts.next())?,
        z: parse_coord(parts.next())?,
    };
    if !(vertex.x.is_finite() && vertex.y.is_finite() && vertex.z.is_finite()) {
        return Err(ObjError::NonFiniteVertex);
    }
    if parts.next().is_some() {
        Err(ObjError::InvalidVertex)
    } else {
        Ok(vertex)
    }
}

fn parse_coord(value: Option<&str>) -> Result<f32, ObjError> {
    value
        .ok_or(ObjError::InvalidVertex)?
        .parse::<f32>()
        .map_err(|_| ObjError::InvalidVertex)
}

fn parse_face_line(line: &str) -> Result<Triangle32, ObjError> {
    let mut parts = record_body(line).split_whitespace();
    if parts.next() != Some("f") {
        return Err(ObjError::NonTriangularFace);
    }
    let face = Triangle32::new(
        parse_index(parts.next())?,
        parse_index(parts.next())?,
        parse_index(parts.next())?,
    );
    if parts.next().is_some() {
        Err(ObjError::NonTriangularFace)
    } else {
        Ok(face)
    }
}

fn parse_index(token: Option<&str>) -> Result<u32, ObjError> {
    let token = token.ok_or(ObjError::NonTriangularFace)?;
    let mut fields = token.split('/');
    let vertex_index = parse_vertex_index(fields.next().ok_or(ObjError::InvalidIndex)?)?;
    match (fields.next(), fields.next(), fields.next()) {
        (None, None, None) => Ok(vertex_index),
        (Some(texture), None, None) => {
            parse_auxiliary_index(texture)?;
            Ok(vertex_index)
        }
        (Some(texture), Some(normal), None) => {
            if !texture.is_empty() {
                parse_auxiliary_index(texture)?;
            }
            parse_auxiliary_index(normal)?;
            Ok(vertex_index)
        }
        _ => Err(ObjError::InvalidIndex),
    }
}

fn parse_vertex_index(index_text: &str) -> Result<u32, ObjError> {
    let one_based = index_text
        .parse::<i64>()
        .map_err(|_| ObjError::InvalidIndex)?;
    let zero_based = one_based.checked_sub(1).ok_or(ObjError::InvalidIndex)?;
    u32::try_from(zero_based).map_err(|_| ObjError::InvalidIndex)
}

fn parse_auxiliary_index(index_text: &str) -> Result<(), ObjError> {
    let one_based = index_text
        .parse::<i64>()
        .map_err(|_| ObjError::InvalidIndex)?;
    let zero_based = one_based.checked_sub(1).ok_or(ObjError::InvalidIndex)?;
    u32::try_from(zero_based)
        .map(|_| ())
        .map_err(|_| ObjError::InvalidIndex)
}
