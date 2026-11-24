# Uniform Grid

This chapter defines Bunny's current contract for the uniform grid broadphase
in `bunny-broadphase`, `uniform_grid_pairs`, under the contract identity
`bunny-uniform-grid/1` (`UNIFORM_GRID_CONTRACT_ID`). It reports every pair of
overlapping boxes, in the same form and order as `sweep_and_prune`, so a
caller can use either one.

## Status

This is a living topic chapter, not a proposal.

The grid is implemented in `crates/bunny-broadphase/src/uniform_grid/` and
exported from `bunny_broadphase` (and `bunny_broadphase::uniform_grid`) as
`uniform_grid_pairs`, `grid_key_count`, `GridCellSize`, `GridKey`,
`GridPairReport` and `UniformGridError`. Evidence is listed in the
[test plan](test-plan.md).

All arithmetic is integer arithmetic on raw Q32.32 values. There is no
floating point, no hashing, no `unsafe` code, no allocation and no
target-specific path, so every supported target gives the same output from
the same input.

## Grid, Not Hash

A cell is identified by its exact integer coordinates, not by a hash of them.
The grid sorts one key per (box, occupied cell) in caller-provided scratch and
reads candidate pairs from runs of equal cells. A spatial hash would add a hash
function and collision handling whose only benefit is an unbounded extent of
cells; integer cell coordinates already cover the whole Q32.32 range (below),
so the grid has no table size, no collisions and nothing for the output to
depend on but the boxes and the cell size.

## Cell Size

`GridCellSize::try_new(size)` (or `try_from_raw(raw)`) accepts any positive
size: from one raw unit (`2^-32`) to `i64::MAX` raw. A power of two is not
required, because the cell of a coordinate is an exact floor division whatever
the divisor. Zero and negative sizes are refused with
`UniformGridError::NonPositiveCellSize { raw }`, naming the refused raw value.

Choose a cell size near the typical box size: see [Cost](#cost).

## Cell Coordinates

The cell of a coordinate `x` is

```text
cell_of(x) = floor(x.raw() / size.raw())
```

computed exactly on raw values (`i64::div_euclid` by the positive size), an
`i64`. Floor, not truncation toward zero: cell `k` is the half-open interval
`[k * size, (k + 1) * size)`, so

- a coordinate exactly on a boundary `k * size` is in the upper cell `k`;
- a negative coordinate maps to the cell below: at size 1.0, `-1` raw and
  `-1.0` are both in cell `-1`, and `-1.0 - 1` raw is in cell `-2`.

For the whole `i64` raw range the cells lie in
`floor(i64::MIN / size) ..= floor(i64::MAX / size)`, which fits `i64`: all of
`i64` at one raw unit, `-2^31 ..= 2^31 - 1` at 1.0, and `-2 ..= 1` at
`i64::MAX`. The division cannot fail, since the size is positive (no zero
divisor, no `i64::MIN / -1`).

## Cells Of A Box And The Key Count

Boxes are closed, as `sweep_and_prune` tests them: two boxes overlap when, on
every axis, `a.min <= b.max` and `a.max >= b.min`, so boxes that touch overlap.

A box occupies every cell from `cell_of(min)` to `cell_of(max)` inclusive on
each axis. A box whose `max` lies exactly on a boundary therefore also occupies
the cell that starts there, which is what makes touching boxes candidates: two
closed boxes that share a point `p` both occupy the cell of `p`, because
`cell_of` is monotone.

A box occupies `(hi_x - lo_x + 1) (hi_y - lo_y + 1) (hi_z - lo_z + 1)` cells,
where `lo` is the cell of its min and `hi` the cell of its max. Each factor is
between 1 and `2^64`, exact in `i128`; the product is checked in `u128` and
then against `usize`. `grid_key_count(boxes, cell_size)` returns the exact sum
over the boxes, the number of key-scratch slots a call needs. It depends on
the boxes and the cell size, not only on the box count.

## Refusals

Checked in this order, before anything is written (neither the key scratch nor
the pair buffer is touched by a refusal):

| Error | When |
| --- | --- |
| `TooManyBoxes { count }` | More than `u32::MAX` boxes, so an index would not fit a pair. |
| `InvertedBox { index }` | A box whose minimum is above its maximum on some axis. |
| `KeyCountOverflow { index }` | The running key count passes `usize` at this box. |
| `KeyScratchTooSmall { required, provided }` | The key scratch is shorter than `grid_key_count`. |

`InvertedBox` and `KeyCountOverflow` are checked box by box in index order, so
the lowest failing index is reported, of either kind. `grid_key_count` makes
the first three checks. An inverted box is refused, as the `bunny-bounds/1`
helpers refuse one; `FixedAabb3::try_new` makes boxes that pass.

## Pairs

The pairs are exactly the pairs of boxes that overlap, each reported once:

1. The keys `(cell, box)` are sorted (cell `x`, `y`, `z`, then box index:
   the derived `Ord` of `GridKey`). Keys are distinct, so the order is total.
2. In each run of keys with the same cell, every two boxes are a candidate,
   the lower index first.
3. A candidate is reported when the boxes overlap by the closed test above (the
   one `sweep_and_prune` uses) and the run's cell is their **first shared
   cell**: on each axis the larger of the two `cell_of(min)` values. Two
   overlapping boxes share every cell from that one to the smaller of their
   `cell_of(max)` values, so the pair is reported in exactly one run.

No overlapping pair is missed (every overlapping pair shares a cell) and none
is invented (the exact test filters every candidate), so the output equals an
all-pairs scan, bit for bit. It does not depend on the cell size.

## Pair Order

Each pair is `(lower index, higher index)`, and the pairs are in ascending
lexicographic order: the order `sweep_and_prune` documents. On the same input
the two give identical output. The order depends only on the boxes and their
indices: not on the cell size, the contents of either buffer on entry, the
order keys were found in, or the target.

## Storage Contract

The caller supplies everything; nothing is allocated.

```rust,ignore
fn uniform_grid_pairs(
    boxes: &[FixedAabb3],
    cell_size: GridCellSize,
    keys: &mut [GridKey],
    pairs: &mut [(u32, u32)],
) -> Result<GridPairReport, UniformGridError>;
```

- `keys` needs at least `grid_key_count(boxes, cell_size)` slots; fill it with
  `GridKey::EMPTY` or anything else, since its contents on entry are never
  read. The first `grid_key_count` slots are overwritten and left holding the
  sorted keys; the rest are not touched.
- `pairs` may have any length. The first `min(pairs.len(), total)` pairs of the
  pair order are written to its front; the rest are not touched.
- The `GridPairReport` counts every pair: `written()` (pairs written),
  `total()` (all overlapping pairs, at most `n (n - 1) / 2 < 2^63`) and
  `is_complete()` (`written == total`). A short pair buffer is never silently
  truncated: the report says how many pairs there are, and the written ones are
  the first of them in order. While the buffer has room pairs go into a
  max-heap on its written prefix; once it is full a pair below the largest
  replaces it; the prefix is sorted at the end.

## Cost

One key per occupied cell, and one candidate test per two boxes per cell they
share, counted in `GridPairReport::candidate_tests()` (saturating at
`u64::MAX`). The cost depends on the cell size:

- a box many cells wide needs many keys: `grid_key_count` says exactly how many
  before any work is done;
- boxes crowded into one cell are all tested against each other: `n` boxes in
  one cell cost `n (n - 1) / 2` tests, as an all-pairs scan does, and two boxes
  sharing `c` cells cost `c` tests.

A cell size near the typical box size keeps both small. There is no cap on the
cells of one box; the caller's scratch is the limit, and too little is refused
with the size required. This chapter makes no timing claim.
