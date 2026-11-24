use super::StlError;

/// Bytes before the first triangle record: the 80-byte header and the `u32` count.
pub(super) const PREFIX_LEN: usize = 84;

/// Bytes per triangle record: a normal and three vertices of three binary32 each, and a
/// `u16` attribute byte count.
pub(super) const RECORD_LEN: usize = 50;

/// The largest triangle count `parse_binary_stl` accepts: `u32::MAX / 3`, so that every
/// vertex index `3 i + k` and the vertex count `3 n` fit `u32`.
pub const BINARY_STL_MAX_TRIANGLES: u32 = u32::MAX / 3;

/// The length rule, then the triangle limit. `actual` is the input length; the expected
/// length is formed in `u64`, where its largest value, `84 + 50 * u32::MAX`, is below
/// `2^38`, so nothing overflows on any target.
pub(super) fn check_layout(triangles: u32, actual: u64) -> Result<(), StlError> {
    let expected = 84 + 50 * u64::from(triangles);
    if actual != expected {
        return Err(StlError::LengthMismatch { triangles, expected, actual });
    }
    if triangles > BINARY_STL_MAX_TRIANGLES {
        return Err(StlError::TooManyTriangles { triangles });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
