//! The (cell, object) keys the grid sorts, and how many a set of boxes needs.

use bunny_geom::FixedAabb3;

use super::cell::{CellRange, GridCellSize};
use super::UniformGridError;

/// One key-scratch slot: a cell and a box index.
///
/// The derived `Ord` is the sort order: cell `x`, then `y`, then `z`, then the box index.
/// Its contents on entry are never read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct GridKey {
    cell: [i64; 3],
    object: u32,
}

impl GridKey {
    /// A slot to fill scratch arrays with.
    pub const EMPTY: Self = Self { cell: [0; 3], object: 0 };

    /// The key of box `object` in `cell`.
    #[must_use]
    pub const fn new(cell: [i64; 3], object: u32) -> Self {
        Self { cell, object }
    }

    /// The integer cell coordinates `[x, y, z]`.
    #[must_use]
    pub const fn cell(&self) -> [i64; 3] {
        self.cell
    }

    /// The index of the box.
    #[must_use]
    pub const fn object(&self) -> u32 {
        self.object
    }
}

/// The exact number of key-scratch slots [`uniform_grid_pairs`](super::uniform_grid_pairs)
/// needs: the sum over the boxes of the cells each occupies.
///
/// A box occupies `(hi_x - lo_x + 1) (hi_y - lo_y + 1) (hi_z - lo_z + 1)` cells, where `lo` is
/// the cell of its min and `hi` the cell of its max. The count depends on the boxes and the
/// cell size, not only on how many boxes there are: a box many cells wide needs many keys.
///
/// # Errors
///
/// Checked in this order: [`UniformGridError::TooManyBoxes`] past `u32::MAX` boxes; then,
/// box by box in index order, [`UniformGridError::InvertedBox`] for a box whose minimum is
/// above its maximum on some axis and [`UniformGridError::KeyCountOverflow`] when the running
/// total passes `usize` at that box. The lowest failing index is reported.
pub fn grid_key_count(
    boxes: &[FixedAabb3],
    cell_size: GridCellSize,
) -> Result<usize, UniformGridError> {
    let count = boxes.len();
    u32::try_from(count).map_err(|_| UniformGridError::TooManyBoxes { count })?;
    boxes.iter().enumerate().try_fold(0_usize, |total, (index, bounds)| {
        let range =
            CellRange::of(cell_size, bounds).ok_or(UniformGridError::InvertedBox { index })?;
        let cells = range.count().and_then(|cells| total.checked_add(cells));
        cells.ok_or(UniformGridError::KeyCountOverflow { index })
    })
}

/// Writes the key of every (box, occupied cell) to `keys` and sorts them.
///
/// `keys` has exactly [`grid_key_count`] slots, checked by the caller, so every key has a slot
/// and every slot is written. Keys are distinct, so the unstable sort is deterministic.
pub(super) fn fill_and_sort(boxes: &[FixedAabb3], cell_size: GridCellSize, keys: &mut [GridKey]) {
    let all = (0_u32..).zip(boxes).flat_map(|(object, bounds)| {
        let cells = CellRange::of(cell_size, bounds).map(CellRange::cells);
        cells.into_iter().flatten().map(move |cell| GridKey::new(cell, object))
    });
    for (slot, key) in keys.iter_mut().zip(all) {
        *slot = key;
    }
    keys.sort_unstable();
}
