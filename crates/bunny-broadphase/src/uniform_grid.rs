//! A deterministic uniform grid broadphase (`bunny-uniform-grid/1`).
//!
//! [`uniform_grid_pairs`] reports every pair of overlapping boxes, as `sweep_and_prune` does,
//! from a uniform grid of cubic cells. A cell is identified by its exact integer coordinates
//! ([`GridCellSize::cell_of`] on each axis), not by a hash: candidates come from sorting one
//! (cell, box) [`GridKey`] per occupied cell in caller-provided scratch. Nothing allocates.

mod cell;
mod error;
mod key;
mod pairs;

pub use cell::GridCellSize;
pub use error::UniformGridError;
pub use key::{grid_key_count, GridKey};
pub use pairs::GridPairReport;

use bunny_geom::FixedAabb3;
use pairs::{Grid, PairSink};

/// The contract identity of the cell rule, the pairs and their order.
pub const UNIFORM_GRID_CONTRACT_ID: &str = "bunny-uniform-grid/1";

/// Writes every pair of overlapping boxes, in pair order, to the front of `pairs`.
///
/// The cell of a coordinate is `floor(raw / cell_size.raw())` ([`GridCellSize::cell_of`]),
/// and a box occupies every cell from the cell of its min to the cell of its max, inclusive,
/// on each axis. Boxes are closed: two boxes that touch overlap, and they share the cell of the
/// point they touch at, so no overlapping pair is missed. Candidates sharing a cell are
/// filtered by the exact closed overlap test `sweep_and_prune` uses, and a pair is reported
/// only in its lowest shared cell, so the pairs are exactly the overlapping pairs, each once.
///
/// Pair order: each pair is `(lower index, higher index)`, and the pairs ascend
/// lexicographically, the order of `sweep_and_prune`. It depends only on the boxes and their
/// indices: not on the cell size, the scratch contents on entry, or the target.
///
/// Storage: `keys` needs at least [`grid_key_count`] slots; its first that many are
/// overwritten and left holding the sorted keys, and the rest are not touched. `pairs` may have
/// any length: the first `min(pairs.len(), total)` pairs of the pair order are written, the
/// rest is not touched, and the [`GridPairReport`] counts every pair, so
/// [`GridPairReport::is_complete`] is `false` when the buffer was short.
///
/// Cost: one key per occupied cell, and one candidate test per two boxes per shared cell
/// ([`GridPairReport::candidate_tests`]). Boxes all in one cell cost `n (n - 1) / 2` tests,
/// as an all-pairs scan does; a cell size near the typical box size keeps both small.
///
/// # Errors
///
/// The refusals of [`grid_key_count`], then [`UniformGridError::KeyScratchTooSmall`] when
/// `keys` is shorter than that count. A refusal writes nothing.
pub fn uniform_grid_pairs(
    boxes: &[FixedAabb3],
    cell_size: GridCellSize,
    keys: &mut [GridKey],
    pairs: &mut [(u32, u32)],
) -> Result<GridPairReport, UniformGridError> {
    let required = grid_key_count(boxes, cell_size)?;
    let provided = keys.len();
    let too_small = UniformGridError::KeyScratchTooSmall { required, provided };
    let keys = keys.get_mut(..required).ok_or(too_small)?;
    key::fill_and_sort(boxes, cell_size, keys);
    let mut sink = PairSink::new(pairs);
    pairs::collect(keys, Grid { boxes, cell_size }, &mut sink);
    Ok(sink.finish())
}
