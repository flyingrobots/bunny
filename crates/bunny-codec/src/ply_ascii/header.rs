use super::{AsciiPlyError, AsciiPlyErrorKind as Kind};

/// Layout of the vertex element and the declared record counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct HeaderSpec {
    pub(super) vertex_count: usize,
    pub(super) face_count: usize,
    pub(super) vertex_properties: usize,
    pub(super) coordinate_slots: [usize; 3],
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Section {
    #[default]
    None,
    Vertex,
    Face,
    IgnoredEmpty,
}

#[derive(Default)]
struct State {
    format_seen: bool,
    section: Section,
    vertex_count: Option<usize>,
    face_count: Option<usize>,
    vertex_properties: usize,
    slots: [Option<usize>; 3],
    face_property_seen: bool,
}

const COORDINATE_TYPES: [&str; 4] = ["float", "float32", "double", "float64"];
const SCALAR_TYPES: [&str; 14] = [
    "char", "uchar", "short", "ushort", "int", "uint", "float", "double", "int8", "uint8", "int16",
    "uint16", "int32", "uint32",
];
const COUNT_TYPES: [&str; 2] = ["uchar", "uint8"];
const INDEX_TYPES: [&str; 4] = ["int", "uint", "int32", "uint32"];

/// Parses the header text, which ends before the `end_header` line. `end_line` is the
/// 1-based number of that line and carries the whole-header errors.
pub(super) fn parse_header(header: &str, end_line: usize) -> Result<HeaderSpec, AsciiPlyError> {
    let mut lines = header.split('\n').map(|line| line.strip_suffix('\r').unwrap_or(line));
    if lines.next().map(|line| line.trim_end_matches([' ', '\t'])) != Some("ply") {
        return Err(AsciiPlyError::new(Kind::InvalidMagic, 1));
    }
    let mut state = State::default();
    for (index, line) in lines.enumerate() {
        parse_line(line, &mut state).map_err(|kind| AsciiPlyError::new(kind, index + 2))?;
    }
    state.finish().map_err(|kind| AsciiPlyError::new(kind, end_line))
}

impl State {
    fn finish(self) -> Result<HeaderSpec, Kind> {
        if !self.format_seen {
            return Err(Kind::UnsupportedFormat);
        }
        let [Some(x), Some(y), Some(z)] = self.slots else {
            return Err(Kind::MissingVertexElement);
        };
        if !self.face_property_seen {
            return Err(Kind::MissingFaceElement);
        }
        Ok(HeaderSpec {
            vertex_count: self.vertex_count.ok_or(Kind::MissingVertexElement)?,
            face_count: self.face_count.ok_or(Kind::MissingFaceElement)?,
            vertex_properties: self.vertex_properties,
            coordinate_slots: [x, y, z],
        })
    }
}

fn parse_line(line: &str, state: &mut State) -> Result<(), Kind> {
    let mut words = line.split([' ', '\t']).filter(|word| !word.is_empty());
    match words.next() {
        None | Some("comment" | "obj_info") => Ok(()),
        Some("format") => parse_format(&mut words, state),
        Some("element") => parse_element(&mut words, state),
        Some("property") => parse_property(&mut words, state),
        Some(_) => Err(Kind::InvalidHeaderLine),
    }
}

fn parse_format<'a>(
    words: &mut impl Iterator<Item = &'a str>,
    state: &mut State,
) -> Result<(), Kind> {
    let valid = !state.format_seen
        && state.section == Section::None
        && words.next() == Some("ascii")
        && words.next() == Some("1.0")
        && words.next().is_none();
    state.format_seen = valid;
    if valid {
        Ok(())
    } else {
        Err(Kind::UnsupportedFormat)
    }
}

fn parse_element<'a>(
    words: &mut impl Iterator<Item = &'a str>,
    state: &mut State,
) -> Result<(), Kind> {
    if !state.format_seen {
        return Err(Kind::UnsupportedFormat);
    }
    let name = words.next().ok_or(Kind::InvalidHeaderLine)?;
    let count = parse_count(words.next().ok_or(Kind::InvalidCount)?)?;
    if words.next().is_some() {
        return Err(Kind::InvalidHeaderLine);
    }
    match name {
        "vertex" => set_vertex_element(state, count),
        "face" => set_face_element(state, count),
        _ if count == 0 => {
            state.section = Section::IgnoredEmpty;
            Ok(())
        }
        _ => Err(Kind::UnsupportedElement),
    }
}

fn parse_count(word: &str) -> Result<usize, Kind> {
    if word.is_empty() || !word.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(Kind::InvalidCount);
    }
    word.parse::<usize>().map_err(|_| Kind::IntegerOverflow)
}

const fn set_vertex_element(state: &mut State, count: usize) -> Result<(), Kind> {
    if state.vertex_count.is_some() || state.face_count.is_some() {
        return Err(Kind::UnsupportedElement);
    }
    state.vertex_count = Some(count);
    state.section = Section::Vertex;
    Ok(())
}

fn set_face_element(state: &mut State, count: usize) -> Result<(), Kind> {
    if state.vertex_count.is_none() || state.slots.iter().any(Option::is_none) {
        return Err(Kind::MissingVertexElement);
    }
    if state.face_count.is_some() {
        return Err(Kind::UnsupportedElement);
    }
    state.face_count = Some(count);
    state.section = Section::Face;
    Ok(())
}

fn parse_property<'a>(
    words: &mut impl Iterator<Item = &'a str>,
    state: &mut State,
) -> Result<(), Kind> {
    match state.section {
        Section::Vertex => parse_vertex_property(words, state),
        Section::Face => parse_face_property(words, state),
        Section::IgnoredEmpty => Ok(()),
        Section::None => Err(Kind::UnsupportedProperty),
    }
}

fn parse_vertex_property<'a>(
    words: &mut impl Iterator<Item = &'a str>,
    state: &mut State,
) -> Result<(), Kind> {
    let kind = words.next().ok_or(Kind::UnsupportedProperty)?;
    let name = words.next().ok_or(Kind::UnsupportedProperty)?;
    if words.next().is_some() || !SCALAR_TYPES.contains(&kind) {
        return Err(Kind::UnsupportedProperty);
    }
    if let Some(axis) = ["x", "y", "z"].iter().position(|axis| *axis == name) {
        let slot = state.slots.get_mut(axis).ok_or(Kind::UnsupportedProperty)?;
        if slot.is_some() || !COORDINATE_TYPES.contains(&kind) {
            return Err(Kind::UnsupportedProperty);
        }
        *slot = Some(state.vertex_properties);
    }
    state.vertex_properties += 1;
    Ok(())
}

fn parse_face_property<'a>(
    words: &mut impl Iterator<Item = &'a str>,
    state: &mut State,
) -> Result<(), Kind> {
    let valid = !state.face_property_seen
        && words.next() == Some("list")
        && words.next().is_some_and(|word| COUNT_TYPES.contains(&word))
        && words.next().is_some_and(|word| INDEX_TYPES.contains(&word))
        && words.next() == Some("vertex_indices")
        && words.next().is_none();
    state.face_property_seen = valid;
    if valid {
        Ok(())
    } else {
        Err(Kind::UnsupportedProperty)
    }
}
