use bunny_mesh::Triangle32;
use bunny_num::{FixedQ32_32, FloatConversionError};

mod error;
mod layout;

pub use error::StlError;
pub use layout::BINARY_STL_MAX_TRIANGLES;
use layout::{check_layout, PREFIX_LEN, RECORD_LEN};

/// Stable identity of the binary STL triangle-mesh profile read by [`parse_binary_stl`].
pub const BINARY_STL_PROFILE_ID: &str = "bunny-binary-stl-triangle-mesh/1";

/// One 50-byte triangle record, borrowed from the input.
type Record = [u8; RECORD_LEN];

/// An STL vertex position in Q32.32, converted from binary32 by
/// [`FixedQ32_32::try_from_f32`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StlVertex {
    /// X coordinate.
    pub x: FixedQ32_32,
    /// Y coordinate.
    pub y: FixedQ32_32,
    /// Z coordinate.
    pub z: FixedQ32_32,
}

/// A borrowed, fully validated binary STL triangle mesh.
///
/// `parse_binary_stl` converts every coordinate once to validate it. The view keeps the
/// borrowed triangle records and re-reads them on iteration, so it holds no allocation.
/// Vertex `3 i + k` is vertex `k` of triangle `i`; nothing is welded.
#[derive(Clone, Copy, Debug)]
pub struct StlMesh<'a> {
    records: &'a [Record],
}

impl<'a> StlMesh<'a> {
    /// Returns the number of triangles, as declared by the file and checked by its length.
    #[must_use]
    pub const fn triangle_count(&self) -> usize {
        self.records.len()
    }

    /// Returns the number of vertices: three per triangle.
    #[must_use]
    pub const fn vertex_count(&self) -> usize {
        // No overflow: the records occupy 50 bytes per triangle of one borrowed slice.
        self.records.len() * 3
    }

    /// Returns a forward iterator over the vertex positions: the three vertices of each
    /// triangle in stored order, triangle by triangle.
    ///
    /// The iterator does not allocate. Each item is `Ok` because the mesh was validated when
    /// it was parsed.
    #[must_use]
    pub const fn vertices(&self) -> StlVertices<'a> {
        StlVertices { records: self.records, triangle: 0, vertex: 0 }
    }

    /// Returns a forward iterator over the triangles: triangle `i` is
    /// `(3 i, 3 i + 1, 3 i + 2)`.
    ///
    /// The iterator does not allocate. Each item is `Ok` because the mesh was validated when
    /// it was parsed.
    #[must_use]
    pub const fn triangles(&self) -> StlTriangles<'a> {
        StlTriangles { records: self.records, next: 0 }
    }
}

/// Forward iterator over the vertices of an [`StlMesh`].
#[derive(Clone, Debug)]
pub struct StlVertices<'a> {
    records: &'a [Record],
    triangle: u32,
    vertex: u8,
}

impl Iterator for StlVertices<'_> {
    type Item = Result<StlVertex, StlError>;

    fn next(&mut self) -> Option<Self::Item> {
        let (record, rest) = self.records.split_first()?;
        let bytes = vertex_bytes(record).nth(usize::from(self.vertex))?;
        let item = read_vertex(bytes, self.triangle, self.vertex);
        if self.vertex == 2 {
            self.records = rest;
            self.triangle = self.triangle.wrapping_add(1);
            self.vertex = 0;
        } else {
            self.vertex += 1;
        }
        Some(item)
    }
}

/// Forward iterator over the triangles of an [`StlMesh`].
#[derive(Clone, Debug)]
pub struct StlTriangles<'a> {
    records: &'a [Record],
    next: u32,
}

impl Iterator for StlTriangles<'_> {
    type Item = Result<Triangle32, StlError>;

    fn next(&mut self) -> Option<Self::Item> {
        let (_, rest) = self.records.split_first()?;
        self.records = rest;
        // At most `BINARY_STL_MAX_TRIANGLES` records, so `3 i + 2 <= u32::MAX - 1`.
        let first = self.next.checked_mul(3)?;
        self.next = self.next.wrapping_add(1);
        Some(Ok(Triangle32::new(first, first.checked_add(1)?, first.checked_add(2)?)))
    }
}

/// Parses a binary STL triangle mesh (`bunny-binary-stl-triangle-mesh/1`).
///
/// The input is an 80-byte header, which is not interpreted, a little-endian `u32` triangle
/// count `n`, and `n` records of 50 bytes. The length must be exactly `84 + 50 n`. Each
/// vertex coordinate is converted by [`FixedQ32_32::try_from_f32`] unchanged: the raw value
/// is the float's exact value times `2^32`, rounded to nearest with ties to even. The stored
/// vertex order is kept; the stored normal and the attribute byte count are never read.
/// The profile is in `docs/topics/binary-stl/README.md`.
///
/// # Errors
/// Returns, in this order of checks: [`StlError::TooShort`] below 84 bytes;
/// [`StlError::LengthMismatch`] when the length is not `84 + 50 n`;
/// [`StlError::TooManyTriangles`] when `n` exceeds [`BINARY_STL_MAX_TRIANGLES`]; then, for
/// the first bad coordinate in file order, [`StlError::NonFiniteVertex`] for a NaN or an
/// infinity and [`StlError::VertexOutOfRange`] for a finite value outside Q32.32.
pub fn parse_binary_stl(input: &[u8]) -> Result<StlMesh<'_>, StlError> {
    let (prefix, body) = input
        .split_first_chunk::<PREFIX_LEN>()
        .ok_or(StlError::TooShort { length: input.len() })?;
    let [.., c0, c1, c2, c3] = *prefix;
    let count = u32::from_le_bytes([c0, c1, c2, c3]);
    check_layout(count, u64::try_from(input.len()).unwrap_or(u64::MAX))?;
    let (records, _) = body.as_chunks::<RECORD_LEN>();
    for (triangle, record) in (0_u32..).zip(records) {
        for (vertex, bytes) in (0_u8..).zip(vertex_bytes(record)) {
            read_vertex(bytes, triangle, vertex)?;
        }
    }
    Ok(StlMesh { records })
}

/// The three 12-byte vertices of a record, after its 12-byte stored normal, which is not read.
/// The 2-byte attribute byte count is the remainder of the split and is not read either.
fn vertex_bytes(record: &Record) -> impl Iterator<Item = &[u8; 12]> {
    record.as_chunks::<12>().0.iter().skip(1)
}

/// Converts one vertex, `x`, `y` and `z` binary32 little-endian.
fn read_vertex(bytes: &[u8; 12], triangle: u32, vertex: u8) -> Result<StlVertex, StlError> {
    let [x0, x1, x2, x3, y0, y1, y2, y3, z0, z1, z2, z3] = *bytes;
    let coordinate = |word: [u8; 4], axis: u8| {
        FixedQ32_32::try_from_f32(f32::from_le_bytes(word)).map_err(|error| match error {
            FloatConversionError::NonFinite => StlError::NonFiniteVertex { triangle, vertex, axis },
            FloatConversionError::OutOfRange => {
                StlError::VertexOutOfRange { triangle, vertex, axis }
            }
        })
    };
    Ok(StlVertex {
        x: coordinate([x0, x1, x2, x3], 0)?,
        y: coordinate([y0, y1, y2, y3], 1)?,
        z: coordinate([z0, z1, z2, z3], 2)?,
    })
}
