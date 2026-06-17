use std::str::Lines;

use bunny_mesh::Triangle32;

use super::{
    face_indices_are_valid, parse_face_line, parse_vertex_line, statement_kind, ObjError, ObjVertex,
};

/// Forward iterator over decoded OBJ vertex records.
#[derive(Clone, Debug)]
pub struct ObjVertices<'a> {
    lines: Lines<'a>,
}

impl<'a> ObjVertices<'a> {
    pub(super) fn new(source: &'a str) -> Self {
        Self { lines: source.lines() }
    }
}

impl Iterator for ObjVertices<'_> {
    type Item = Result<ObjVertex, ObjError>;

    fn next(&mut self) -> Option<Self::Item> {
        next_record(&mut self.lines, "v").map(|line| line.and_then(parse_vertex_line))
    }
}

/// Forward iterator over decoded OBJ triangle face records.
#[derive(Clone, Debug)]
pub struct ObjTriangles<'a> {
    lines: Lines<'a>,
    vertex_count: usize,
}

impl<'a> ObjTriangles<'a> {
    pub(super) fn new(source: &'a str, vertex_count: usize) -> Self {
        Self { lines: source.lines(), vertex_count }
    }
}

impl Iterator for ObjTriangles<'_> {
    type Item = Result<Triangle32, ObjError>;

    fn next(&mut self) -> Option<Self::Item> {
        next_record(&mut self.lines, "f").map(|line| {
            line.and_then(parse_face_line)
                .and_then(|triangle| validate_iter_triangle(triangle, self.vertex_count))
        })
    }
}

pub(super) fn find_record<'a>(source: &'a str, kind: &str, index: usize) -> Option<&'a str> {
    let mut lines = source.lines();
    let mut found = 0;
    while let Some(record) = next_record(&mut lines, kind) {
        let Ok(line) = record else {
            return None;
        };
        if found == index {
            return Some(line);
        }
        found += 1;
    }
    None
}

fn next_record<'a>(lines: &mut Lines<'a>, kind: &str) -> Option<Result<&'a str, ObjError>> {
    for line in lines {
        match statement_kind(line) {
            Ok(Some(record_kind)) if record_kind == kind => return Some(Ok(line)),
            Ok(Some(_) | None) => {}
            Err(error) => return Some(Err(error)),
        }
    }
    None
}

fn validate_iter_triangle(
    triangle: Triangle32,
    vertex_count: usize,
) -> Result<Triangle32, ObjError> {
    if face_indices_are_valid(triangle, vertex_count) {
        Ok(triangle)
    } else {
        Err(ObjError::IndexOutOfBounds)
    }
}
