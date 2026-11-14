use bunny_mesh::Triangle32;
use bunny_num::FixedQ32_32;

use super::decimal::parse_decimal_q32_32;
use super::header::HeaderSpec;
use super::{AsciiPlyError, AsciiPlyErrorKind as Kind, AsciiPlyVertex};

/// Cursor over the lines of the input. A line ends at `\n`, and one `\r` directly
/// before it belongs to the terminator. The last line may lack a terminator.
#[derive(Clone, Copy, Debug)]
pub(super) struct Lines<'a> {
    rest: &'a [u8],
    next_line: usize,
}

impl<'a> Lines<'a> {
    pub(super) const fn new(rest: &'a [u8], first_line: usize) -> Self {
        Self { rest, next_line: first_line }
    }

    pub(super) const fn remaining(&self) -> usize {
        self.rest.len()
    }

    /// Returns the next line's number and content, or `None` at the end of the input.
    pub(super) fn next_line(&mut self) -> Option<(usize, &'a [u8])> {
        if self.rest.is_empty() {
            return None;
        }
        let (content, rest) = match self.rest.iter().position(|byte| *byte == b'\n') {
            Some(end) => (self.rest.get(..end)?, self.rest.get(end + 1..)?),
            None => (self.rest, <&[u8]>::default()),
        };
        let number = self.next_line;
        self.next_line += 1;
        self.rest = rest;
        Some((number, content.strip_suffix(b"\r").unwrap_or(content)))
    }

    /// Returns the next line that is not empty or only spaces and tabs.
    pub(super) fn next_record(&mut self) -> Option<(usize, &'a [u8])> {
        loop {
            let (number, line) = self.next_line()?;
            if !is_blank(line) {
                return Some((number, line));
            }
        }
    }

    pub(super) fn only_blank_lines_remain(mut self) -> Result<(), AsciiPlyError> {
        match self.next_record() {
            None => Ok(()),
            Some((number, _)) => Err(AsciiPlyError::new(Kind::TrailingData, number)),
        }
    }

    pub(super) fn missing_record(&self, kind: Kind) -> AsciiPlyError {
        AsciiPlyError::new(kind, self.next_line)
    }
}

/// Removes leading and trailing spaces and tabs.
pub(super) fn trim_blank(line: &[u8]) -> &[u8] {
    let start = line.iter().position(|byte| !matches!(byte, b' ' | b'\t')).unwrap_or(line.len());
    let end = line.iter().rposition(|byte| !matches!(byte, b' ' | b'\t')).map_or(start, |i| i + 1);
    line.get(start..end).unwrap_or_default()
}

fn is_blank(line: &[u8]) -> bool {
    line.iter().all(|byte| matches!(byte, b' ' | b'\t'))
}

fn tokens(line: &[u8]) -> impl Iterator<Item = &[u8]> + Clone {
    line.split(|byte| matches!(byte, b' ' | b'\t')).filter(|token| !token.is_empty())
}

pub(super) fn read_vertex(
    line: &[u8],
    number: usize,
    spec: &HeaderSpec,
) -> Result<AsciiPlyVertex, AsciiPlyError> {
    let fail = |kind: Kind| AsciiPlyError::new(kind, number);
    if tokens(line).count() != spec.vertex_properties {
        return Err(fail(Kind::VertexFieldCount));
    }
    let coordinate = |slot: usize| -> Result<FixedQ32_32, AsciiPlyError> {
        let token = tokens(line).nth(slot).ok_or_else(|| fail(Kind::VertexFieldCount))?;
        parse_decimal_q32_32(token).map_err(|error| fail(error.into()))
    };
    let [x, y, z] = spec.coordinate_slots;
    Ok(AsciiPlyVertex { x: coordinate(x)?, y: coordinate(y)?, z: coordinate(z)? })
}

pub(super) fn read_triangle(
    line: &[u8],
    number: usize,
    vertex_count: usize,
) -> Result<Triangle32, AsciiPlyError> {
    let fail = |kind: Kind| AsciiPlyError::new(kind, number);
    let mut words = tokens(line);
    let count = words.next().ok_or_else(|| fail(Kind::FaceFieldCount))?;
    match parse_unsigned(count) {
        Some(3) => {}
        Some(_) => return Err(fail(Kind::NonTriangularFace)),
        None => return Err(fail(Kind::InvalidIndex)),
    }
    let (Some(a), Some(b), Some(c), None) = (words.next(), words.next(), words.next(), words.next())
    else {
        return Err(fail(Kind::FaceFieldCount));
    };
    let index = |token: &[u8]| read_index(token, vertex_count).map_err(&fail);
    Ok(Triangle32::new(index(a)?, index(b)?, index(c)?))
}

fn parse_unsigned(token: &[u8]) -> Option<u64> {
    if token.is_empty() || !token.iter().all(u8::is_ascii_digit) {
        return None;
    }
    let saturating = |acc: u64, byte: &u8| acc.saturating_mul(10).saturating_add(u64::from(byte - b'0'));
    Some(token.iter().fold(0_u64, saturating))
}

fn read_index(token: &[u8], vertex_count: usize) -> Result<u32, Kind> {
    if let Some(digits) = token.strip_prefix(b"-") {
        let valid = !digits.is_empty() && digits.iter().all(u8::is_ascii_digit);
        return Err(if valid { Kind::NegativeIndex } else { Kind::InvalidIndex });
    }
    let value = parse_unsigned(token).ok_or(Kind::InvalidIndex)?;
    let index = u32::try_from(value).map_err(|_| Kind::IndexOutOfBounds)?;
    if usize::try_from(index).is_ok_and(|index| index < vertex_count) {
        Ok(index)
    } else {
        Err(Kind::IndexOutOfBounds)
    }
}
