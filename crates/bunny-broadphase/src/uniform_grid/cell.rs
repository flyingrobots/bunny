//! The cell size and the quantization of Q32.32 coordinates to integer cell coordinates.

use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;

use super::UniformGridError;

/// The edge length of a grid cell: a positive Q32.32 value, from one raw unit to `i64::MAX`.
///
/// Any positive raw value is a valid size, a power of two or not: the cell of a coordinate
/// is an exact floor division either way.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct GridCellSize {
    raw: i64,
}

impl GridCellSize {
    /// The cell size `size`.
    ///
    /// # Errors
    ///
    /// [`UniformGridError::NonPositiveCellSize`] when `size` is zero or negative.
    pub const fn try_new(size: FixedQ32_32) -> Result<Self, UniformGridError> {
        Self::try_from_raw(size.raw())
    }

    /// The cell size of `raw` raw units (`raw / 2^32`).
    ///
    /// # Errors
    ///
    /// [`UniformGridError::NonPositiveCellSize`] when `raw` is zero or negative.
    pub const fn try_from_raw(raw: i64) -> Result<Self, UniformGridError> {
        if raw > 0 {
            Ok(Self { raw })
        } else {
            Err(UniformGridError::NonPositiveCellSize { raw })
        }
    }

    /// The size in raw units: at least 1.
    #[must_use]
    pub const fn raw(self) -> i64 {
        self.raw
    }

    /// The size as a Q32.32 value.
    #[must_use]
    pub const fn size(self) -> FixedQ32_32 {
        FixedQ32_32::from_raw(self.raw)
    }

    /// The cell of `coordinate`: `floor(coordinate.raw() / self.raw())`, exactly.
    ///
    /// Cell `k` is the half-open interval `[k * size, (k + 1) * size)`, so a coordinate exactly
    /// on a boundary is in the upper cell and `-1` raw is in cell `-1`. For the whole `i64` raw
    /// range the cell lies in `floor(i64::MIN / size) ..= floor(i64::MAX / size)`, inside `i64`
    /// (all of it at one raw unit; `-2 ..= 1` at `i64::MAX`). The division cannot fail: the
    /// size is positive, so neither a zero divisor nor `i64::MIN / -1` can occur.
    #[must_use]
    pub const fn cell_of(self, coordinate: FixedQ32_32) -> i64 {
        coordinate.raw().div_euclid(self.raw)
    }

    const fn cell_of_point(self, point: FixedVec3) -> [i64; 3] {
        [self.cell_of(point.x), self.cell_of(point.y), self.cell_of(point.z)]
    }
}

/// The cells a closed box occupies: from the cell of its min to the cell of its max, inclusive,
/// on each axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CellRange {
    lo: [i64; 3],
    hi: [i64; 3],
}

impl CellRange {
    /// The cells of `bounds`, or `None` when it is inverted on some axis.
    pub(super) fn of(size: GridCellSize, bounds: &FixedAabb3) -> Option<Self> {
        let (min, max) = (bounds.min, bounds.max);
        let ordered = min.x <= max.x && min.y <= max.y && min.z <= max.z;
        ordered.then(|| Self { lo: size.cell_of_point(min), hi: size.cell_of_point(max) })
    }

    /// The lowest cell on each axis: the cell of the box's min.
    pub(super) const fn lo(&self) -> [i64; 3] {
        self.lo
    }

    /// The number of cells, or `None` past `usize`.
    ///
    /// Each axis spans `hi - lo + 1` cells, at most `2^64`, exact in `i128`; the product is
    /// checked in `u128`.
    pub(super) fn count(&self) -> Option<usize> {
        let mut spans = self.lo.iter().zip(&self.hi).map(|(&lo, &hi)| span(lo, hi));
        let cells = spans.try_fold(1_u128, u128::checked_mul)?;
        usize::try_from(cells).ok()
    }

    /// Every cell, `z` fastest, then `y`, then `x`.
    pub(super) const fn cells(self) -> Cells {
        Cells { range: self, next: Some(self.lo) }
    }
}

/// The cells of one axis, `hi - lo + 1` with `lo <= hi`: from 1 to `2^64`.
fn span(lo: i64, hi: i64) -> u128 {
    (i128::from(hi) - i128::from(lo) + 1).unsigned_abs()
}

/// The iterator of [`CellRange::cells`].
pub(super) struct Cells {
    range: CellRange,
    next: Option<[i64; 3]>,
}

impl Iterator for Cells {
    type Item = [i64; 3];

    fn next(&mut self) -> Option<[i64; 3]> {
        let cell = self.next?;
        self.next = self.after(cell);
        Some(cell)
    }
}

impl Cells {
    /// The cell after `cell`, or `None` after the last. Comparing before incrementing keeps
    /// every step inside `i64`, up to a range ending at `i64::MAX`.
    fn after(&self, mut cell: [i64; 3]) -> Option<[i64; 3]> {
        let axes = cell.iter_mut().zip(&self.range.lo).zip(&self.range.hi).rev();
        for ((value, &lo), &hi) in axes {
            if *value < hi {
                *value += 1;
                return Some(cell);
            }
            *value = lo;
        }
        None
    }
}
