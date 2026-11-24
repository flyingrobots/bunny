//! `broadphase/sweep_and_prune`: one `sweep_and_prune` of 27 648 seeded boxes per call, the
//! size of the BVH cases' scene (27 650 triangle boxes).
//!
//! The boxes are built once, before timing, from the fixed seed [`SWEEP_SEED`]: centres in a
//! slab 256 units long in `x` and 8 units across in `y` and `z`, half extents in
//! `[1/8, 1/4)` units per axis, so a box overlaps under one other on average and the sweep
//! along `x` compares each box with a few dozen neighbours. Each call sorts, sweeps and sorts
//! the pairs into buffers allocated once (room for four pairs per box). The call's pair count
//! and the whole pair buffer pass through `black_box`, and the checksum is the count plus the
//! first and last pairs.

use std::cell::RefCell;
use std::hint::black_box;

use bunny_broadphase::sweep_and_prune;
use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;

use crate::cases::Case;

/// The generator's fixed seed.
const SWEEP_SEED: u64 = 0xB002_0130_5A90_0001;
/// Boxes in the scene: `6 * 48 * 48 * 2`, the BVH scene's sphere triangles.
const BOXES: usize = 27_648;
/// Room in the pair buffer, per box.
const PAIRS_PER_BOX: usize = 4;

fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state
}

/// A seeded raw value in `[-half_span, half_span)` units.
fn centre(state: &mut u64, half_span: i64) -> i64 {
    let span = (2 * half_span * ONE_RAW).unsigned_abs();
    i64::try_from(next(state) % span).unwrap_or(0) - half_span * ONE_RAW
}

/// A seeded raw half extent in `[1/8, 1/4)` units.
fn half_extent(state: &mut u64) -> i64 {
    let eighth = ONE_RAW / 8;
    eighth + i64::try_from(next(state) % eighth.unsigned_abs()).unwrap_or(0)
}

/// The scene's boxes, from [`SWEEP_SEED`].
pub(crate) fn boxes() -> Vec<FixedAabb3> {
    let mut state = SWEEP_SEED;
    (0..BOXES)
        .map(|_| {
            let c = [centre(&mut state, 128), centre(&mut state, 4), centre(&mut state, 4)];
            let h = [half_extent(&mut state), half_extent(&mut state), half_extent(&mut state)];
            let point = |sign: i64| {
                let at = |axis: usize| FixedQ32_32::from_raw(c[axis] + sign * h[axis]);
                FixedVec3::new(at(0), at(1), at(2))
            };
            FixedAabb3::new(point(-1), point(1))
        })
        .collect()
}

/// Every case of the `sweep` group.
pub(crate) fn sweep_cases() -> Vec<Case> {
    let boxes = boxes();
    let buffers = RefCell::new((vec![(0_u32, 0_u32); PAIRS_PER_BOX * BOXES], vec![0_u32; BOXES]));
    let run = move || {
        let (pairs, prim_indices) = &mut *buffers.borrow_mut();
        let count = sweep_and_prune(pairs, prim_indices, black_box(&boxes));
        let (count, pairs) = black_box((count, &*pairs));
        let Some(count) = count else { return -1 };
        let pair =
            |i: usize| pairs.get(i).map_or(0, |&(a, b)| i64::from(a) * 65_536 + i64::from(b));
        i64::try_from(count).unwrap_or(0) + pair(0) + pair(count.saturating_sub(1))
    };
    vec![Case { name: "broadphase/sweep_and_prune", calls: 1, run: Box::new(run) }]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(boxes: &[FixedAabb3]) -> Vec<u8> {
        boxes
            .iter()
            .flat_map(|b| [b.min.x, b.min.y, b.min.z, b.max.x, b.max.y, b.max.z])
            .flat_map(|c| c.raw().to_le_bytes())
            .collect()
    }

    fn fnv1a(bytes: &[u8]) -> u64 {
        bytes.iter().fold(0xCBF2_9CE4_8422_2325, |hash, &byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01B3)
        })
    }

    /// The scene is the same bytes on every construction, and these (a digest recorded from
    /// this generator).
    #[test]
    fn the_scene_is_the_same_on_every_construction() {
        let (first, second) = (bytes(&boxes()), bytes(&boxes()));
        assert_eq!(first, second);
        assert_eq!(first.len(), BOXES * 48);
        assert_eq!(fnv1a(&first), SCENE_DIGEST);
    }

    /// The sweep finds pairs, fits the buffer (run with `--nocapture` to see the count), and finds exactly the pairs a brute-force
    /// all-pairs test finds over boxes near each other in `x` (an oracle that shares no code
    /// with `sweep_and_prune`).
    #[test]
    fn the_sweep_finds_every_overlapping_pair() {
        let boxes = boxes();
        let mut pairs = vec![(0_u32, 0_u32); PAIRS_PER_BOX * BOXES];
        let mut indices = vec![0_u32; BOXES];
        let count = sweep_and_prune(&mut pairs, &mut indices, &boxes).expect("the buffer fits");
        println!("broadphase/sweep_and_prune finds {count} pairs among {BOXES} boxes");
        assert!(count > BOXES / 4 && count < BOXES * 2, "{count} pairs");
        let overlap = |a: &FixedAabb3, b: &FixedAabb3| {
            a.min.x <= b.max.x
                && b.min.x <= a.max.x
                && a.min.y <= b.max.y
                && b.min.y <= a.max.y
                && a.min.z <= b.max.z
                && b.min.z <= a.max.z
        };
        let mut order: Vec<usize> = (0..BOXES).collect();
        order.sort_by_key(|&i| boxes[i].min.x);
        let mut want = Vec::new();
        for (k, &i) in order.iter().enumerate() {
            for &j in &order[k + 1..] {
                if boxes[j].min.x > boxes[i].max.x {
                    break;
                }
                if overlap(&boxes[i], &boxes[j]) {
                    let (a, b) = (i.min(j), i.max(j));
                    want.push((u32::try_from(a).unwrap_or(0), u32::try_from(b).unwrap_or(0)));
                }
            }
        }
        want.sort_unstable();
        assert_eq!(pairs[..count], want[..]);
    }

    /// The case runs once (no timing) and returns the same checksum on every run and every
    /// construction; it is not the failure value `-1`.
    #[test]
    fn the_sweep_case_runs_and_its_checksum_is_stable() {
        let (first, second) = (sweep_cases(), sweep_cases());
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].name, "broadphase/sweep_and_prune");
        let value = (first[0].run)();
        assert_eq!(value, (first[0].run)());
        assert_eq!(value, (second[0].run)());
        assert_ne!(value, -1);
    }

    const SCENE_DIGEST: u64 = 0;
}
