//! Shared helpers for fixed-point matrix primitives.

use bunny_num::FixedQ32_32;

/// A four-component row used by [`crate::FixedMat4`].
///
/// The tuple entries are columns 0 through 3 for a single row. `FixedMat4`
/// stores rows in row-major order and multiplies column vectors.
pub type FixedMat4Row = (FixedQ32_32, FixedQ32_32, FixedQ32_32, FixedQ32_32);
