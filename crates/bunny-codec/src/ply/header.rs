use super::PlyError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct HeaderSpec {
    pub(super) vertex_count: usize,
    pub(super) face_count: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum HeaderSection {
    #[default]
    None,
    Vertex,
    Face,
    IgnoredEmptyElement,
}

#[derive(Default)]
struct HeaderParseState {
    format_seen: bool,
    section: HeaderSection,
    vertex_count: Option<usize>,
    face_count: Option<usize>,
    vertex_props: u8,
    face_property_seen: bool,
}

pub(super) fn parse_header(header: &str) -> Result<HeaderSpec, PlyError> {
    let mut lines = header.lines();
    if lines.next() != Some("ply") {
        return Err(PlyError::InvalidMagic);
    }

    let mut state = HeaderParseState::default();
    for line in lines {
        parse_header_line(line.trim_end_matches('\r'), &mut state)?;
    }
    state.finish()
}

impl HeaderParseState {
    fn finish(self) -> Result<HeaderSpec, PlyError> {
        if !self.format_seen {
            return Err(PlyError::UnsupportedFormat);
        }
        if self.vertex_props != 3 {
            return Err(PlyError::MissingVertexElement);
        }
        if !self.face_property_seen {
            return Err(PlyError::MissingFaceElement);
        }
        Ok(HeaderSpec {
            vertex_count: self.vertex_count.ok_or(PlyError::MissingVertexElement)?,
            face_count: self.face_count.ok_or(PlyError::MissingFaceElement)?,
        })
    }
}

fn parse_header_line(line: &str, state: &mut HeaderParseState) -> Result<(), PlyError> {
    if line.starts_with("comment ") || line.starts_with("obj_info ") || line.is_empty() {
        return Ok(());
    }
    let mut parts = line.split_whitespace();
    match parts.next() {
        Some("format") => parse_format(&mut parts, state),
        Some("element") => parse_element(&mut parts, state),
        Some("property") => parse_property(&mut parts, state),
        Some(_) | None => Err(PlyError::UnsupportedProperty),
    }
}

fn parse_format<'a>(
    parts: &mut impl Iterator<Item = &'a str>,
    state: &mut HeaderParseState,
) -> Result<(), PlyError> {
    let valid = parts.next() == Some("binary_little_endian")
        && parts.next() == Some("1.0")
        && parts.next().is_none();
    if valid {
        state.format_seen = true;
        Ok(())
    } else {
        Err(PlyError::UnsupportedFormat)
    }
}

fn parse_element<'a>(
    parts: &mut impl Iterator<Item = &'a str>,
    state: &mut HeaderParseState,
) -> Result<(), PlyError> {
    let name = parts.next().ok_or(PlyError::UnsupportedElement)?;
    let count = parts
        .next()
        .ok_or(PlyError::InvalidCount)?
        .parse::<usize>()
        .map_err(|_| PlyError::InvalidCount)?;
    if parts.next().is_some() {
        return Err(PlyError::UnsupportedElement);
    }
    match name {
        "vertex" => set_vertex_element(state, count),
        "face" => set_face_element(state, count),
        _ if count == 0 => {
            state.section = HeaderSection::IgnoredEmptyElement;
            Ok(())
        }
        _ => Err(PlyError::UnsupportedElement),
    }
}

const fn set_vertex_element(state: &mut HeaderParseState, count: usize) -> Result<(), PlyError> {
    if state.vertex_count.is_some() || state.face_count.is_some() {
        return Err(PlyError::UnsupportedElement);
    }
    state.vertex_count = Some(count);
    state.vertex_props = 0;
    state.section = HeaderSection::Vertex;
    Ok(())
}

const fn set_face_element(state: &mut HeaderParseState, count: usize) -> Result<(), PlyError> {
    if state.vertex_count.is_none() || state.vertex_props != 3 {
        return Err(PlyError::MissingVertexElement);
    }
    if state.face_count.is_some() {
        return Err(PlyError::UnsupportedElement);
    }
    state.face_count = Some(count);
    state.face_property_seen = false;
    state.section = HeaderSection::Face;
    Ok(())
}

fn parse_property<'a>(
    parts: &mut impl Iterator<Item = &'a str>,
    state: &mut HeaderParseState,
) -> Result<(), PlyError> {
    match state.section {
        HeaderSection::Vertex => parse_vertex_property(parts, state),
        HeaderSection::Face => parse_face_property(parts, state),
        HeaderSection::IgnoredEmptyElement => Ok(()),
        HeaderSection::None => Err(PlyError::UnsupportedProperty),
    }
}

fn parse_vertex_property<'a>(
    parts: &mut impl Iterator<Item = &'a str>,
    state: &mut HeaderParseState,
) -> Result<(), PlyError> {
    let expected = match state.vertex_props {
        0 => Some("x"),
        1 => Some("y"),
        2 => Some("z"),
        _ => None,
    };
    if parts.next() == Some("float") && parts.next() == expected && parts.next().is_none() {
        state.vertex_props += 1;
        Ok(())
    } else {
        Err(PlyError::UnsupportedProperty)
    }
}

fn parse_face_property<'a>(
    parts: &mut impl Iterator<Item = &'a str>,
    state: &mut HeaderParseState,
) -> Result<(), PlyError> {
    let valid = parts.next() == Some("list")
        && parts.next() == Some("uchar")
        && parts.next() == Some("int")
        && parts.next() == Some("vertex_indices")
        && parts.next().is_none();
    if valid && !state.face_property_seen {
        state.face_property_seen = true;
        Ok(())
    } else {
        Err(PlyError::UnsupportedProperty)
    }
}
