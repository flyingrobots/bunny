//! `uniform_grid_pairs` (`bunny-uniform-grid/1`) against an independent brute-force oracle.
//!
//! The oracle shares nothing with the kernel: floor division in `i128` with an explicit
//! remainder correction, a closed-interval overlap written as `max(mins) <= min(maxes)`, an
//! all-pairs scan, and key counts as exact `u128` products. Seeds are named constants and
//! printed; every seeded scene pins its pair count.

use bunny_broadphase::sweep_and_prune;
use bunny_broadphase::uniform_grid::{
    grid_key_count, uniform_grid_pairs, GridCellSize, GridKey, GridPairReport, UniformGridError,
    UNIFORM_GRID_CONTRACT_ID,
};
use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const ONE: i64 = bunny_num::fixed_q32_32::ONE_RAW;
const MIN: i64 = i64::MIN;
const MAX: i64 = i64::MAX;
const UNTOUCHED: (u32, u32) = (u32::MAX, u32::MAX);

// ---- Independent oracle -------------------------------------------------------------------

/// `floor(x / s)` for `s > 0`, from truncating `i128` division and a remainder correction.
fn oracle_floor(x: i64, s: i64) -> i64 {
    assert!(s > 0);
    let (x, s) = (i128::from(x), i128::from(s));
    let quotient = x / s;
    let corrected = if x % s < 0 { quotient - 1 } else { quotient };
    i64::try_from(corrected).expect("a floor quotient by s >= 1 fits i64")
}

fn axes(v: &FixedVec3) -> [i64; 3] {
    [v.x.raw(), v.y.raw(), v.z.raw()]
}

/// Closed boxes overlap when, on every axis, the larger minimum is at most the smaller maximum.
fn oracle_overlap(a: &FixedAabb3, b: &FixedAabb3) -> bool {
    let (a_min, a_max, b_min, b_max) = (axes(&a.min), axes(&a.max), axes(&b.min), axes(&b.max));
    (0..3).all(|k| a_min[k].max(b_min[k]) <= a_max[k].min(b_max[k]))
}

/// Every overlapping pair `(i, j)`, `i < j`, in lexicographic order: the all-pairs scan.
fn oracle_pairs(boxes: &[FixedAabb3]) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    for (i, a) in boxes.iter().enumerate() {
        for (j, b) in boxes.iter().enumerate().skip(i + 1) {
            if oracle_overlap(a, b) {
                out.push((u32::try_from(i).unwrap(), u32::try_from(j).unwrap()));
            }
        }
    }
    out
}

/// The exact number of (object, cell) keys, or `None` past `u128`.
fn oracle_key_count(boxes: &[FixedAabb3], s: i64) -> Option<u128> {
    let mut total: u128 = 0;
    for b in boxes {
        let (lo, hi) = (axes(&b.min), axes(&b.max));
        let mut cells: u128 = 1;
        for (l, h) in lo.into_iter().zip(hi) {
            let span = i128::from(oracle_floor(h, s)) - i128::from(oracle_floor(l, s)) + 1;
            cells = cells.checked_mul(u128::try_from(span).unwrap())?;
        }
        total = total.checked_add(cells)?;
    }
    Some(total)
}

/// Every (cell, object) key, sorted: cell x, y, z, then object.
fn oracle_keys(boxes: &[FixedAabb3], s: i64) -> Vec<([i64; 3], u32)> {
    let mut out = Vec::new();
    for (object, b) in boxes.iter().enumerate() {
        let (lo, hi) = (axes(&b.min), axes(&b.max));
        let lo = lo.map(|v| oracle_floor(v, s));
        let hi = hi.map(|v| oracle_floor(v, s));
        for x in lo[0]..=hi[0] {
            for y in lo[1]..=hi[1] {
                for z in lo[2]..=hi[2] {
                    out.push(([x, y, z], u32::try_from(object).unwrap()));
                }
            }
        }
    }
    out.sort_unstable();
    out
}

// ---- Helpers ------------------------------------------------------------------------------

fn vec3(raw: [i64; 3]) -> FixedVec3 {
    FixedVec3::new(
        FixedQ32_32::from_raw(raw[0]),
        FixedQ32_32::from_raw(raw[1]),
        FixedQ32_32::from_raw(raw[2]),
    )
}

fn bx(min: [i64; 3], max: [i64; 3]) -> FixedAabb3 {
    FixedAabb3::new(vec3(min), vec3(max))
}

/// A box on x only: y and z are the zero-extent interval at 0.
fn bx_x(min: i64, max: i64) -> FixedAabb3 {
    bx([min, 0, 0], [max, 0, 0])
}

fn size(raw: i64) -> GridCellSize {
    GridCellSize::try_from_raw(raw).expect("a positive cell size")
}

struct Run {
    report: GridPairReport,
    pairs: Vec<(u32, u32)>,
    keys: Vec<GridKey>,
}

/// Runs the kernel with exactly the required scratch and a pair buffer of `pair_len`.
fn run_with(boxes: &[FixedAabb3], cell: GridCellSize, pair_len: usize) -> Run {
    let required = grid_key_count(boxes, cell).expect("the key count is defined");
    let mut keys = vec![GridKey::EMPTY; required];
    let mut pairs = vec![UNTOUCHED; pair_len];
    let report = uniform_grid_pairs(boxes, cell, &mut keys, &mut pairs).expect("the grid runs");
    Run { report, pairs, keys }
}

/// Runs the kernel with a pair buffer large enough for every pair and returns the pairs.
fn grid_pairs(boxes: &[FixedAabb3], cell: GridCellSize) -> Vec<(u32, u32)> {
    let expected = oracle_pairs(boxes).len();
    let run = run_with(boxes, cell, expected + 3);
    assert!(run.report.is_complete());
    assert_eq!(run.report.written(), expected);
    assert_eq!(run.report.total(), u64::try_from(expected).unwrap());
    assert!(
        run.pairs[expected..].iter().all(|p| *p == UNTOUCHED),
        "entries past written are untouched"
    );
    run.pairs[..expected].to_vec()
}

fn key_tuples(keys: &[GridKey]) -> Vec<([i64; 3], u32)> {
    keys.iter().map(|k| (k.cell(), k.object())).collect()
}

/// Asserts the full contract on one input: keys, pairs and report equal the oracle.
fn assert_matches_oracle(boxes: &[FixedAabb3], cell: GridCellSize) -> usize {
    let expected = oracle_pairs(boxes);
    let count = oracle_key_count(boxes, cell.raw()).expect("small inputs have a key count");
    assert_eq!(grid_key_count(boxes, cell), Ok(usize::try_from(count).unwrap()));
    let run = run_with(boxes, cell, expected.len());
    assert_eq!(
        key_tuples(&run.keys),
        oracle_keys(boxes, cell.raw()),
        "keys are the sorted (cell, object) set"
    );
    assert_eq!(run.pairs, expected, "pairs equal the all-pairs scan");
    assert!(run.report.is_complete());
    expected.len()
}

/// A 64-bit linear congruential generator (Knuth's MMIX constants); every test names its seed.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 =
            self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 16
    }

    /// Uniform in `[-bound, bound]`.
    fn signed(&mut self, bound: i64) -> i64 {
        let span = u64::try_from(2 * bound + 1).unwrap();
        i64::try_from(self.next() % span).unwrap() - bound
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }
}

/// A coordinate near the grid: a multiple of `cell` moved by -1, 0 or +1 raw unit half the
/// time, and an arbitrary raw value otherwise.
fn near_grid(rng: &mut Lcg, cells: i64, cell: i64) -> i64 {
    let k = rng.signed(cells);
    if rng.below(2) == 0 {
        k * cell + rng.signed(1)
    } else {
        k * cell + rng.signed(cell)
    }
}

fn seeded_box(rng: &mut Lcg, cells: i64, cell: i64, max_extent: i64) -> FixedAabb3 {
    let mut min = [0; 3];
    let mut max = [0; 3];
    for (lo, hi) in min.iter_mut().zip(max.iter_mut()) {
        *lo = near_grid(rng, cells, cell);
        let extent = match rng.below(4) {
            0 => 0,
            1 => cell * (1 + rng.signed(max_extent).abs()),
            _ => rng.signed(max_extent * cell).abs(),
        };
        *hi = *lo + extent;
    }
    bx(min, max)
}

/// `count` seeded boxes on a grid of 1.0 units, corners within `cells` units of the origin
/// and extents up to `max_extent` units. Coordinates sit on a unit boundary and one raw unit
/// either side about half the time; some boxes are zero-extent; some are duplicated.
fn seeded_scene(seed: u64, count: usize, cells: i64, max_extent: i64) -> Vec<FixedAabb3> {
    let mut rng = Lcg(seed);
    let mut boxes = Vec::with_capacity(count);
    while boxes.len() < count {
        let b = seeded_box(&mut rng, cells, ONE, max_extent);
        boxes.push(b);
        if rng.below(8) == 0 && boxes.len() < count {
            boxes.push(b);
        }
    }
    boxes
}

// ---- GRID-TP-001: cell size domain --------------------------------------------------------

#[wasm_bindgen_test(unsupported = test)]
fn grid_tp_001_cell_size_must_be_positive() {
    assert_eq!(UNIFORM_GRID_CONTRACT_ID, "bunny-uniform-grid/1");
    for raw in [0, -1, -ONE, MIN] {
        assert_eq!(
            GridCellSize::try_from_raw(raw),
            Err(UniformGridError::NonPositiveCellSize { raw })
        );
        assert_eq!(
            GridCellSize::try_new(FixedQ32_32::from_raw(raw)),
            Err(UniformGridError::NonPositiveCellSize { raw })
        );
    }
    for raw in [1, 2, 3, ONE - 1, ONE, 3 * ONE + 7, MAX] {
        let cell = GridCellSize::try_from_raw(raw).expect("positive sizes are accepted");
        assert_eq!(cell.raw(), raw);
        assert_eq!(cell.size(), FixedQ32_32::from_raw(raw));
        assert_eq!(GridCellSize::try_new(FixedQ32_32::from_raw(raw)), Ok(cell));
    }
}

// ---- GRID-TP-002: floor quantization ------------------------------------------------------

#[wasm_bindgen_test(unsupported = test)]
fn grid_tp_002_cell_of_is_floor_division_on_raw_values() {
    // Each size: on every boundary k * s for k in -3..=3, and one raw unit either side.
    let sizes = [1, 2, 3, ONE - 1, ONE, ONE + 1, 3 * ONE + 7, 1 << 62, MAX];
    let mut checked = 0;
    for &s in &sizes {
        let cell = size(s);
        for k in -3_i128..=3 {
            for d in -1_i128..=1 {
                let Ok(x) = i64::try_from(k * i128::from(s) + d) else { continue };
                assert_eq!(
                    cell.cell_of(FixedQ32_32::from_raw(x)),
                    oracle_floor(x, s),
                    "x {x} s {s}"
                );
                checked += 1;
            }
        }
        for x in [MIN, MIN + 1, -1, 0, 1, MAX - 1, MAX] {
            assert_eq!(cell.cell_of(FixedQ32_32::from_raw(x)), oracle_floor(x, s), "x {x} s {s}");
            checked += 1;
        }
    }
    println!("grid_tp_002: {checked} coordinates checked");
    assert_eq!(checked, 230, "every size and boundary offset was checked (recorded count)");

    // Hand-derived values: the half-open cell [k s, (k + 1) s), so a boundary belongs to the
    // upper cell and a negative value to the cell below zero.
    let one = size(ONE);
    let at = |c: GridCellSize, x: i64| c.cell_of(FixedQ32_32::from_raw(x));
    assert_eq!([at(one, -ONE - 1), at(one, -ONE), at(one, -1), at(one, 0)], [-2, -1, -1, 0]);
    assert_eq!([at(one, ONE - 1), at(one, ONE), at(one, ONE + 1)], [0, 1, 1]);
    assert_eq!([at(one, MIN), at(one, MAX)], [-(1 << 31), (1 << 31) - 1]);
    let unit = size(1);
    assert_eq!([at(unit, MIN), at(unit, -1), at(unit, MAX)], [MIN, -1, MAX]);
    let widest = size(MAX);
    assert_eq!(
        [at(widest, MIN), at(widest, MIN + 1), at(widest, -1), at(widest, 0)],
        [-2, -1, -1, 0]
    );
    assert_eq!([at(widest, MAX - 1), at(widest, MAX)], [0, 1]);
    let three = size(3);
    assert_eq!([at(three, -4), at(three, -3), at(three, -2), at(three, -1)], [-2, -1, -1, -1]);
}

// ---- GRID-TP-003: occupied cells and the exact key count ----------------------------------

#[wasm_bindgen_test(unsupported = test)]
fn grid_tp_003_a_box_occupies_the_cells_of_its_min_to_its_max_inclusive() {
    let s = ONE;
    let one = size(s);
    // On x only (y and z occupy cell 0): faces on, and one raw unit inside, a boundary.
    let count = |b: FixedAabb3| grid_key_count(&[b], one).unwrap();
    assert_eq!(count(bx_x(0, s)), 2, "a max exactly on a boundary also occupies the next cell");
    assert_eq!(count(bx_x(0, s - 1)), 1);
    assert_eq!(count(bx_x(1, s)), 2);
    assert_eq!(count(bx_x(-1, 0)), 2, "-1 raw is in cell -1, 0 in cell 0");
    assert_eq!(count(bx_x(-s, -1)), 1);
    assert_eq!(count(bx_x(-s - 1, -s)), 2, "cells -2 and -1");
    assert_eq!(
        count(bx_x(-2 * s, -s - 1)),
        1,
        "cell -2 only (truncation would give cells -2 and -1)"
    );
    assert_eq!(count(bx_x(-s + 1, -1)), 1, "cell -1 only");
    assert_eq!(count(bx_x(5, 5)), 1, "a point occupies one cell");
    // Three axes multiply, and boxes add.
    assert_eq!(count(bx([0, -1, -s - 1], [2 * s, 0, -s])), 3 * 2 * 2);
    assert_eq!(grid_key_count(&[bx_x(0, s), bx_x(-1, 0), bx_x(5, 5)], one), Ok(5));
    assert_eq!(grid_key_count(&[], one), Ok(0));
    // The widest cell: the whole raw range is cells -2..=1 on each axis.
    assert_eq!(grid_key_count(&[bx([MIN; 3], [MAX; 3])], size(MAX)), Ok(64));

    let keys = run_with(&[bx_x(0, s)], one, 0).keys;
    assert_eq!(key_tuples(&keys), vec![([0, 0, 0], 0), ([1, 0, 0], 0)]);
    let keys = run_with(&[bx_x(-s + 1, -1)], one, 0).keys;
    assert_eq!(key_tuples(&keys), vec![([-1, 0, 0], 0)], "truncation would give cell 0");
}

#[wasm_bindgen_test(unsupported = test)]
fn grid_tp_004_key_count_overflow_is_refused_with_the_box_index() {
    // 2^64 cells on one axis and 2^64 on another overflow u128: refused at box 0.
    let all = bx([MIN; 3], [MAX; 3]);
    assert_eq!(
        grid_key_count(&[all], size(1)),
        Err(UniformGridError::KeyCountOverflow { index: 0 })
    );
    // 2^32 cells per axis: 2^96 keys, past usize on every target.
    assert_eq!(
        grid_key_count(&[all], size(ONE)),
        Err(UniformGridError::KeyCountOverflow { index: 0 })
    );
    // 2^21 cells per axis is 2^63 keys: it fits a 64-bit usize, and two of them do not.
    let big = bx([0; 3], [(1 << 21) - 1; 3]);
    let first = usize::from(usize::BITS == 64);
    if usize::BITS == 64 {
        assert_eq!(grid_key_count(&[big], size(1)), Ok(1 << 63));
    }
    assert_eq!(
        grid_key_count(&[big, big], size(1)),
        Err(UniformGridError::KeyCountOverflow { index: first })
    );
    assert_eq!(oracle_key_count(&[big, big], 1), Some(1 << 64), "the oracle's exact count");
    // The same refusal from the kernel, which then writes nothing.
    let mut keys = [GridKey::EMPTY; 4];
    let mut pairs = [UNTOUCHED; 4];
    assert_eq!(
        uniform_grid_pairs(&[big, big], size(1), &mut keys, &mut pairs),
        Err(UniformGridError::KeyCountOverflow { index: first })
    );
    assert_eq!(keys, [GridKey::EMPTY; 4]);
    assert_eq!(pairs, [UNTOUCHED; 4]);
}

// ---- GRID-TP-005: refusals and their order ------------------------------------------------

#[wasm_bindgen_test(unsupported = test)]
fn grid_tp_005_inverted_boxes_and_short_key_scratch_are_refused_before_anything_is_written() {
    let one = size(ONE);
    let ok = bx([0; 3], [ONE; 3]);
    for axis in 0..3 {
        let mut max = [ONE; 3];
        max[axis] = -1;
        let inverted = bx([0; 3], max);
        let boxes = [ok, ok, inverted, ok, inverted];
        assert_eq!(grid_key_count(&boxes, one), Err(UniformGridError::InvertedBox { index: 2 }));
        let mut keys = [GridKey::EMPTY; 64];
        let mut pairs = [UNTOUCHED; 8];
        assert_eq!(
            uniform_grid_pairs(&boxes, one, &mut keys, &mut pairs),
            Err(UniformGridError::InvertedBox { index: 2 })
        );
        assert_eq!(pairs, [UNTOUCHED; 8]);
        assert_eq!(keys, [GridKey::EMPTY; 64]);
    }
    // A box is checked whole before the next one: the lowest failing index wins, of either kind.
    let all = bx([MIN; 3], [MAX; 3]);
    let inverted = bx([1, 0, 0], [0, 0, 0]);
    let small = bx([0; 3], [1; 3]);
    assert_eq!(
        grid_key_count(&[small, inverted, all], size(1)),
        Err(UniformGridError::InvertedBox { index: 1 })
    );
    assert_eq!(
        grid_key_count(&[small, all, inverted], size(1)),
        Err(UniformGridError::KeyCountOverflow { index: 1 })
    );

    // Short key scratch: required - 1 is refused naming both sizes; exactly required and more run.
    let boxes = [ok, bx([ONE; 3], [2 * ONE; 3]), bx_x(-1, 0)];
    let required = grid_key_count(&boxes, one).unwrap();
    assert_eq!(required, 8 + 8 + 2);
    let mut keys = vec![GridKey::EMPTY; required - 1];
    let mut pairs = [UNTOUCHED; 4];
    assert_eq!(
        uniform_grid_pairs(&boxes, one, &mut keys, &mut pairs),
        Err(UniformGridError::KeyScratchTooSmall { required, provided: required - 1 })
    );
    assert_eq!(pairs, [UNTOUCHED; 4]);
    assert!(keys.iter().all(|k| *k == GridKey::EMPTY));
    for extra in [0, 1, 50] {
        let mut keys = vec![GridKey::new([7, 7, 7], 9); required + extra];
        let mut pairs = [UNTOUCHED; 4];
        let report = uniform_grid_pairs(&boxes, one, &mut keys, &mut pairs).unwrap();
        assert_eq!(&pairs[..report.written()], &oracle_pairs(&boxes)[..]);
        assert!(
            keys[required..].iter().all(|k| *k == GridKey::new([7, 7, 7], 9)),
            "keys past required untouched"
        );
    }
    // No boxes: nothing needed, nothing written.
    let report = uniform_grid_pairs(&[], one, &mut [], &mut []).unwrap();
    assert_eq!((report.written(), report.total(), report.candidate_tests()), (0, 0, 0));
    assert!(report.is_complete());
}

// ---- GRID-TP-006: boundary on a cell edge -------------------------------------------------

#[wasm_bindgen_test(unsupported = test)]
fn grid_tp_006_boxes_with_faces_on_cell_boundaries() {
    let s = ONE;
    let one = size(s);
    let pairs = |boxes: &[FixedAabb3]| {
        assert_matches_oracle(boxes, one);
        grid_pairs(boxes, one)
    };
    // Two boxes touching exactly at the boundary x = s are a pair (closed boxes); one raw unit
    // apart they are not; one raw unit overlapping they are.
    assert_eq!(pairs(&[bx_x(0, s), bx_x(s, 2 * s)]), vec![(0, 1)]);
    assert_eq!(pairs(&[bx_x(0, s), bx_x(s + 1, 2 * s)]), vec![]);
    assert_eq!(pairs(&[bx_x(0, s), bx_x(s - 1, 2 * s)]), vec![(0, 1)]);
    assert_eq!(pairs(&[bx_x(0, s - 1), bx_x(s, 2 * s)]), vec![]);
    // At zero.
    assert_eq!(pairs(&[bx_x(-s, 0), bx_x(0, s)]), vec![(0, 1)]);
    assert_eq!(pairs(&[bx_x(-s, -1), bx_x(0, s)]), vec![]);
    assert_eq!(pairs(&[bx_x(-s, 1), bx_x(0, s)]), vec![(0, 1)]);
    // At negative boundaries, where truncation would give the wrong cell.
    assert_eq!(pairs(&[bx_x(-2 * s, -s), bx_x(-s, 0)]), vec![(0, 1)]);
    assert_eq!(pairs(&[bx_x(-2 * s, -s - 1), bx_x(-s, 0)]), vec![]);
    assert_eq!(pairs(&[bx_x(-2 * s, -s + 1), bx_x(-s, 0)]), vec![(0, 1)]);
    assert_eq!(pairs(&[bx_x(-s - 1, -s - 1), bx_x(-s, -s)]), vec![]);
    // A zero-extent box exactly on a boundary meets both neighbours.
    assert_eq!(pairs(&[bx_x(-s, -s), bx_x(-2 * s, -s), bx_x(-s, 0)]), vec![(0, 1), (0, 2), (1, 2)]);
    // Touching at a corner on a boundary in all three axes.
    assert_eq!(
        pairs(&[bx([-s; 3], [0; 3]), bx([0; 3], [s; 3]), bx([1; 3], [s; 3])]),
        vec![(0, 1), (1, 2)]
    );
    // The keys of the touching pair: the shared boundary cell holds both.
    let run = run_with(&[bx_x(0, s), bx_x(s, 2 * s)], one, 1);
    assert_eq!(
        key_tuples(&run.keys),
        vec![([0, 0, 0], 0), ([1, 0, 0], 0), ([1, 0, 0], 1), ([2, 0, 0], 1)]
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn grid_tp_007_boxes_at_the_extremes_of_the_raw_range() {
    // Cell size one raw unit: the cell of a coordinate is the coordinate.
    let unit = size(1);
    assert_eq!(
        assert_matches_oracle(&[bx([MIN; 3], [MIN; 3]), bx([MIN; 3], [MIN + 1; 3])], unit),
        1
    );
    assert_eq!(
        assert_matches_oracle(&[bx([MAX; 3], [MAX; 3]), bx([MAX - 1; 3], [MAX - 1; 3])], unit),
        0
    );
    assert_eq!(
        assert_matches_oracle(&[bx([MAX - 1; 3], [MAX; 3]), bx([MAX; 3], [MAX; 3])], unit),
        1
    );
    // Cell size 1.0: the lowest cell is -2^31 and the highest 2^31 - 1.
    let one = size(ONE);
    let low = [bx_x(MIN, MIN + ONE - 1), bx_x(MIN + ONE, MIN + ONE), bx_x(MIN, MIN + ONE)];
    assert_eq!(assert_matches_oracle(&low, one), 2);
    assert_eq!(key_tuples(&run_with(&low[..1], one, 0).keys), vec![([-(1 << 31), 0, 0], 0)]);
    let high = [bx_x(MAX - 1, MAX), bx_x(MAX, MAX), bx_x(MAX - ONE, MAX - ONE)];
    assert_eq!(assert_matches_oracle(&high, one), 1);
    assert_eq!(key_tuples(&run_with(&high[1..2], one, 0).keys), vec![([(1 << 31) - 1, 0, 0], 0)]);
    // The widest cells: boxes spanning the whole range, the ends, and zero.
    let widest = size(MAX);
    let wide = [
        bx([MIN; 3], [MAX; 3]),
        bx([MIN; 3], [MIN; 3]),
        bx([MAX; 3], [MAX; 3]),
        bx([0; 3], [0; 3]),
        bx([-1; 3], [-1; 3]),
    ];
    assert_eq!(assert_matches_oracle(&wide, widest), 4);
    let half = size(1 << 62);
    assert_eq!(assert_matches_oracle(&wide, half), 4);
}

// ---- GRID-TP-008: equality with the all-pairs scan over seeded scenes ---------------------

struct Scene {
    name: &'static str,
    seed: u64,
    boxes: Vec<FixedAabb3>,
    cells: Vec<i64>,
}

fn scenes() -> Vec<Scene> {
    let mut out = Vec::new();
    // Scattered over 100 cells a side, negative and positive, sizes up to 3 cells.
    out.push(Scene {
        name: "scattered",
        seed: 0x5CA7_7E2D,
        boxes: seeded_scene(0x5CA7_7E2D, 400, 20, 3),
        cells: vec![ONE, 3 * ONE + 7, 5 * ONE],
    });
    // A dense cluster: 250 boxes in a few cells.
    out.push(Scene {
        name: "dense",
        seed: 0xDE45_E001,
        boxes: seeded_scene(0xDE45_E001, 250, 2, 1),
        cells: vec![ONE, ONE / 2 + 3, 4 * ONE],
    });
    // Boxes spanning up to 8 cells a side.
    out.push(Scene {
        name: "many-cells",
        seed: 0x3A11_CE11,
        boxes: seeded_scene(0x3A11_CE11, 80, 10, 7),
        cells: vec![ONE, 2 * ONE + 1],
    });
    // One box larger than the whole occupied region among small boxes.
    let mut huge = seeded_scene(0x4B16_B0C5, 150, 8, 1);
    huge.insert(37, bx([-20 * ONE; 3], [20 * ONE; 3]));
    out.push(Scene {
        name: "huge",
        seed: 0x4B16_B0C5,
        boxes: huge,
        cells: vec![4 * ONE, 13 * ONE + 5],
    });
    // Identical boxes and points.
    let mut same = vec![bx([ONE; 3], [2 * ONE; 3]); 12];
    same.extend(vec![bx([2 * ONE; 3], [2 * ONE; 3]); 5]);
    same.extend(seeded_scene(0x1DE7_1CA1, 40, 3, 1));
    out.push(Scene {
        name: "identical",
        seed: 0x1DE7_1CA1,
        boxes: same,
        cells: vec![ONE, ONE / 3],
    });
    out
}

/// The recorded pair count of each seeded scene (equal for every cell size).
const PINNED: [(&str, usize); 5] =
    [("scattered", 70), ("dense", 326), ("many-cells", 50), ("huge", 170), ("identical", 158)];

#[wasm_bindgen_test(unsupported = test)]
fn grid_tp_008_seeded_scenes_equal_the_all_pairs_scan() {
    for (scene, (name, pinned)) in scenes().into_iter().zip(PINNED) {
        assert_eq!(scene.name, name);
        let expected = oracle_pairs(&scene.boxes);
        println!(
            "grid_tp_008: scene {} seed {:#x}: {} boxes, {} pairs",
            scene.name,
            scene.seed,
            scene.boxes.len(),
            expected.len()
        );
        assert_eq!(expected.len(), pinned, "scene {} pair count", scene.name);
        for &cell in &scene.cells {
            assert_eq!(
                assert_matches_oracle(&scene.boxes, size(cell)),
                expected.len(),
                "scene {} cell {cell}",
                scene.name
            );
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn grid_tp_009_seeded_scenes_at_the_extremes_of_the_raw_range() {
    const SEED: u64 = 0xE87E_3E55;
    let mut rng = Lcg(SEED);
    let mut boxes = Vec::new();
    let anchors = [MIN, MIN + 1, -(1 << 62), -1, 0, 1, 1 << 62, MAX - 1, MAX];
    for _ in 0..120 {
        let mut min = [0; 3];
        let mut max = [0; 3];
        for (lo, hi) in min.iter_mut().zip(max.iter_mut()) {
            let a = anchors[usize::try_from(rng.below(9)).unwrap()];
            let b = anchors[usize::try_from(rng.below(9)).unwrap()];
            (*lo, *hi) = (a.min(b), a.max(b));
        }
        boxes.push(bx(min, max));
    }
    let expected = oracle_pairs(&boxes);
    println!("grid_tp_009: seed {SEED:#x}: {} boxes, {} pairs", boxes.len(), expected.len());
    assert_eq!(expected.len(), 3180, "recorded pair count");
    for cell in [1 << 62, MAX, (1 << 61) + 3] {
        assert_eq!(assert_matches_oracle(&boxes, size(cell)), expected.len());
    }
}

// ---- GRID-TP-010: the same pairs and order as sweep_and_prune -----------------------------

#[wasm_bindgen_test(unsupported = test)]
fn grid_tp_010_pairs_equal_sweep_and_prune_on_the_same_input() {
    for scene in scenes() {
        let expected = oracle_pairs(&scene.boxes);
        let mut sweep = vec![(0, 0); expected.len()];
        let mut indices = vec![0; scene.boxes.len()];
        let count =
            sweep_and_prune(&mut sweep, &mut indices, &scene.boxes).expect("the pair buffer fits");
        assert_eq!(count, expected.len());
        for &cell in &scene.cells {
            assert_eq!(
                grid_pairs(&scene.boxes, size(cell)),
                sweep,
                "scene {} cell {cell}",
                scene.name
            );
        }
    }
}

// ---- GRID-TP-011: independence from scratch contents and input order ----------------------

#[wasm_bindgen_test(unsupported = test)]
fn grid_tp_011_output_does_not_depend_on_scratch_contents_or_input_order() {
    const SEED: u64 = 0x0DE7_E2A1;
    let boxes = seeded_scene(SEED, 300, 12, 3);
    let cell = size(2 * ONE + 1);
    let expected = oracle_pairs(&boxes);
    let required = grid_key_count(&boxes, cell).unwrap();
    println!(
        "grid_tp_011: seed {SEED:#x}: {} boxes, {} keys, {} pairs",
        boxes.len(),
        required,
        expected.len()
    );

    let mut rng = Lcg(SEED ^ 0xFFFF);
    let garbage: Vec<GridKey> = (0..required)
        .map(|_| {
            GridKey::new(
                [rng.signed(1 << 60), rng.signed(3), MIN],
                u32::try_from(rng.below(1 << 32)).unwrap(),
            )
        })
        .collect();
    let mut previous = run_with(&boxes, cell, expected.len()).keys;
    previous.reverse();
    let fills = [vec![GridKey::EMPTY; required], garbage, previous];
    let mut reports = Vec::new();
    for fill in fills {
        let mut keys = fill;
        let mut pairs: Vec<(u32, u32)> =
            (0..expected.len()).map(|i| (u32::try_from(i).unwrap(), 0)).collect();
        let report = uniform_grid_pairs(&boxes, cell, &mut keys, &mut pairs).unwrap();
        assert_eq!(pairs, expected);
        assert_eq!(key_tuples(&keys), oracle_keys(&boxes, cell.raw()));
        reports.push(report);
    }
    assert!(reports.windows(2).all(|w| w[0] == w[1]), "identical reports");

    // Permuting the input permutes the indices and nothing else.
    let mut order: Vec<usize> = (0..boxes.len()).collect();
    for i in (1..order.len()).rev() {
        let j = usize::try_from(rng.below(u64::try_from(i + 1).unwrap())).unwrap();
        order.swap(i, j);
    }
    let permuted: Vec<FixedAabb3> = order.iter().map(|&i| boxes[i]).collect();
    let mut mapped: Vec<(u32, u32)> = grid_pairs(&permuted, cell)
        .into_iter()
        .map(|(a, b)| {
            let index = |i: u32| u32::try_from(order[usize::try_from(i).unwrap()]).unwrap();
            let (a, b) = (index(a), index(b));
            (a.min(b), a.max(b))
        })
        .collect();
    mapped.sort_unstable();
    assert_eq!(mapped, expected);
}

// ---- GRID-TP-012: a short pair buffer -----------------------------------------------------

#[wasm_bindgen_test(unsupported = test)]
fn grid_tp_012_a_short_pair_buffer_holds_the_first_pairs_and_the_total_counts_all() {
    const SEED: u64 = 0x5807_7B0F;
    let boxes = seeded_scene(SEED, 120, 4, 2);
    let cell = size(ONE);
    let expected = oracle_pairs(&boxes);
    println!("grid_tp_012: seed {SEED:#x}: {} boxes, {} pairs", boxes.len(), expected.len());
    assert!(expected.len() > 20, "enough pairs to cut short");
    for len in 0..=expected.len() + 2 {
        let run = run_with(&boxes, cell, len);
        let written = len.min(expected.len());
        assert_eq!(run.report.written(), written);
        assert_eq!(run.report.total(), u64::try_from(expected.len()).unwrap());
        assert_eq!(run.report.is_complete(), len >= expected.len());
        assert_eq!(
            &run.pairs[..written],
            &expected[..written],
            "the first {written} pairs in order"
        );
        assert!(run.pairs[written..].iter().all(|p| *p == UNTOUCHED));
    }
}

// ---- GRID-TP-013: each pair once, and the candidate-test count ----------------------------

#[wasm_bindgen_test(unsupported = test)]
fn grid_tp_013_pairs_sharing_many_cells_are_reported_once() {
    let one = size(ONE);
    // Two identical boxes sharing 27 cells: one pair, 27 candidate tests.
    let run = run_with(&[bx([0; 3], [2 * ONE; 3]); 2], one, 4);
    assert_eq!(
        (run.report.written(), run.report.total(), run.report.candidate_tests()),
        (1, 1, 27)
    );
    assert_eq!(run.pairs[0], (0, 1));
    // Ten points in one cell: every pair, tested once each.
    let points: Vec<FixedAabb3> = (0..10).map(|i| bx([i * 1000; 3], [i * 1000 + 1; 3])).collect();
    let run = run_with(&points, one, 0);
    assert_eq!((run.report.total(), run.report.candidate_tests()), (0, 45));
    let nested: Vec<FixedAabb3> = (0..10).map(|i| bx([i; 3], [ONE - 1 - i; 3])).collect();
    let run = run_with(&nested, one, 45);
    assert_eq!((run.report.total(), run.report.candidate_tests()), (45, 45));
    // Boxes sharing cells but not overlapping are tested and not reported.
    let run = run_with(&[bx_x(0, 10), bx_x(11, 20)], one, 1);
    assert_eq!((run.report.total(), run.report.candidate_tests()), (0, 1));
}
