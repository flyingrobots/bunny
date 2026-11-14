//! Golden vectors and property oracles for the counter-based sample hash and the
//! Owen-scrambled Sobol pixel sampler: locked raw outputs, exact stratification of every
//! power-of-two sample block, range, evaluation-order independence, domain separation of the
//! key fields and an adjacent-key avalanche guard.

use bunny_linalg::FixedVec3;
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_optics::{
    fraction_from_bits, CameraSpec, PinholeCamera, PixelSample, PixelSampler, PixelStream,
    SAMPLER_ID, SAMPLE_HASH_ID,
};
use wasm_bindgen_test::wasm_bindgen_test;

/// The seed of the shuffles and seeded keys in this file; not a sampler seed of any contract.
const SAMPLER_TEST_SEED: u64 = 0xB002_0221_5A3F_0001;
const MIXED_SEED: u64 = 0x0123_4567_89AB_CDEF;
const MAX_K: u32 = 10;

/// A full key: (seed, x, y, sample, dimension or pair).
type Key = (u64, u32, u32, u32, u32);

struct Seeds(u64);

impl Seeds {
    fn next(&mut self) -> u64 {
        self.0 =
            self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    fn next_u32(&mut self) -> u32 {
        u32::try_from(self.next() >> 32).expect("high half fits u32")
    }

    /// Fisher-Yates shuffle with this generator.
    fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = usize::try_from(self.next() >> 33).expect("fits usize") % (i + 1);
            items.swap(i, j);
        }
    }
}

/// The raw 32-bit fraction of a coordinate, checked to be in `[0, 1)`.
fn raw_u32(value: FixedQ32_32) -> u32 {
    assert!((0..ONE_RAW).contains(&value.raw()), "coordinate {} outside [0, 1)", value.raw());
    u32::try_from(value.raw()).expect("below 2^32")
}

fn point_raw(sampler: &PixelSampler, sample: u32, pair: u32) -> (u32, u32) {
    let [u, v] = sampler.point(sample, pair);
    (raw_u32(u), raw_u32(v))
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_010_sample_hash_golden_vectors() {
    // (seed, x, y, sample, dimension) -> bits, from the reference model described in the test
    // plan. Zeros, one-unit steps in each field, all-ones, and adjacent keys.
    let cases: [(Key, u64); 10] = [
        ((0, 0, 0, 0, 0), 0x2382_75BC_38FC_BE91),
        ((0, 1, 0, 0, 0), 0x44E5_B981_00C6_7FB0),
        ((0, 0, 1, 0, 0), 0x19AE_FFC3_9B6F_4E53),
        ((0, 0, 0, 1, 0), 0x2F32_A784_96C6_7C60),
        ((0, 0, 0, 0, 1), 0x9E44_3F00_9112_736D),
        ((1, 0, 0, 0, 0), 0xB18A_02F4_6D8D_86C3),
        ((u64::MAX, u32::MAX, u32::MAX, u32::MAX, u32::MAX), 0x595B_17F4_87C0_E71B),
        ((MIXED_SEED, 1919, 1079, 15, 3), 0xE0C0_2E39_A879_A39C),
        ((MIXED_SEED, 1920, 1079, 15, 3), 0xAED7_0724_971D_1A6D),
        ((MIXED_SEED, 1919, 1079, 16, 3), 0x6B24_D34E_883C_8E7E),
    ];
    for (index, ((seed, x, y, sample, dimension), expected)) in cases.into_iter().enumerate() {
        let stream = PixelStream::new(seed, x, y);
        assert_eq!(stream.bits(sample, dimension), expected, "case {index}");
        let fraction = stream.fraction(sample, dimension);
        assert_eq!(fraction.raw(), i64::try_from(expected >> 32).expect("fits"), "case {index}");
    }
    assert_eq!(SAMPLE_HASH_ID, "bunny-sample-hash/1");
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_011_fractions_are_in_the_unit_interval() {
    assert_eq!(fraction_from_bits(0).raw(), 0);
    assert_eq!(fraction_from_bits(u64::MAX).raw(), ONE_RAW - 1);
    assert_eq!(fraction_from_bits((1 << 32) - 1).raw(), 0, "the low 32 bits are dropped");
    assert_eq!(fraction_from_bits(1 << 32).raw(), 1);
    assert_eq!(fraction_from_bits(1 << 63).raw(), ONE_RAW / 2);
    let mut seeds = Seeds(SAMPLER_TEST_SEED);
    for _ in 0..4096 {
        let stream = PixelStream::new(seeds.next(), seeds.next_u32(), seeds.next_u32());
        raw_u32(stream.fraction(seeds.next_u32(), seeds.next_u32()));
        let sampler = PixelSampler::new(seeds.next(), seeds.next_u32(), seeds.next_u32());
        point_raw(&sampler, seeds.next_u32(), seeds.next_u32());
    }
}

/// Whether each of the 2^k cells of the partition of the unit square into `2^p` columns and
/// `2^(k - p)` rows is hit exactly once: `false` if two points share a cell or a cell is empty.
fn cell_partition_is_exact(points: &[(u32, u32)], k: u32, p: u32) -> bool {
    let q = k - p;
    let mut seen = vec![false; 1 << k];
    for &(u, v) in points {
        let column = u64::from(u) >> (32 - p);
        let row = u64::from(v) >> (32 - q);
        let cell = usize::try_from((column << q) | row).expect("cell index fits usize");
        if std::mem::replace(&mut seen[cell], true) {
            return false;
        }
    }
    seen.iter().all(|&hit| hit)
}

/// Checks the (0, k, 2)-net property for every block size up to `2^MAX_K` and every elementary
/// partition, for samples `start .. start + 2^k` with `start` a multiple of 2^k.
fn check_stratified(sampler: &PixelSampler, pair: u32, block: u32) {
    for k in 0..=MAX_K {
        let start = block << k;
        let points: Vec<(u32, u32)> =
            (0..1_u32 << k).map(|i| point_raw(sampler, start + i, pair)).collect();
        for p in 0..=k {
            assert!(
                cell_partition_is_exact(&points, k, p),
                "pair {pair}, block {block}, N = 2^{k}: partition {p} x {} is not exact",
                k - p
            );
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_012_power_of_two_blocks_are_stratified() {
    let fixed_keys: [(u64, u32, u32, u32); 4] = [
        (0, 0, 0, 0),
        (MIXED_SEED, 1919, 1079, 1),
        (u64::MAX, u32::MAX, u32::MAX, u32::MAX),
        (7, 3, 9, 2),
    ];
    let mut seeds = Seeds(SAMPLER_TEST_SEED ^ 0x12);
    let seeded =
        (0..8).map(|_| (seeds.next(), seeds.next_u32(), seeds.next_u32(), seeds.next_u32() % 8));
    for (seed, x, y, pair) in fixed_keys.into_iter().chain(seeded) {
        let sampler = PixelSampler::new(seed, x, y);
        check_stratified(&sampler, pair, 0);
        check_stratified(&sampler, pair, 3);
    }
    // The largest block that fits the index range: samples 2^32 - 2^10 ..= 2^32 - 1.
    let sampler = PixelSampler::new(MIXED_SEED, 5, 6);
    let last = u32::MAX >> MAX_K;
    let points: Vec<(u32, u32)> =
        (0..1_u32 << MAX_K).map(|i| point_raw(&sampler, (last << MAX_K) + i, 4)).collect();
    for p in 0..=MAX_K {
        assert!(cell_partition_is_exact(&points, MAX_K, p), "last block, partition {p}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_013_sampler_golden_vectors() {
    // (seed, x, y, sample, pair) -> raw (u, v), from the reference model.
    let cases: [(Key, (u32, u32)); 10] = [
        ((0, 0, 0, 0, 0), (0x4353_0EFA, 0xC365_A181)),
        ((0, 0, 0, 1, 0), (0xB366_3062, 0x7162_20B7)),
        ((0, 0, 0, 2, 0), (0x12F3_1CF1, 0x109B_153C)),
        ((0, 0, 0, 3, 0), (0xC1BC_C77F, 0xB1D4_9202)),
        ((0, 1, 0, 0, 0), (0x907D_21AA, 0xEDE7_DBB3)),
        ((0, 0, 1, 0, 0), (0x5905_A167, 0xE2E3_693A)),
        ((0, 0, 0, 0, 1), (0x1BB2_D72B, 0x196A_BD10)),
        ((u64::MAX, u32::MAX, u32::MAX, u32::MAX, u32::MAX), (0x235A_8CA7, 0xB1E1_99C5)),
        ((MIXED_SEED, 1919, 1079, 15, 0), (0xCE19_1653, 0xC76C_191A)),
        ((MIXED_SEED, 1919, 1079, 15, 2), (0xD06D_C56C, 0x1947_15E4)),
    ];
    for (index, ((seed, x, y, sample, pair), expected)) in cases.into_iter().enumerate() {
        let sampler = PixelSampler::new(seed, x, y);
        assert_eq!(point_raw(&sampler, sample, pair), expected, "case {index}");
        if pair == 0 {
            let [u, v] = sampler.point(sample, 0);
            assert_eq!(sampler.camera_sample(sample), PixelSample::new(x, y, u, v), "case {index}");
        }
    }
    assert_eq!(SAMPLER_ID, "bunny-sobol-owen/1");
}

type Draw = (u32, u32, u32, u32, u64, (u32, u32));

/// Every value a small render would draw, keyed by (x, y, sample, dimension or pair), in the
/// given order of keys.
fn draws(order: &[(u32, u32, u32, u32)]) -> Vec<Draw> {
    let mut out: Vec<Draw> = order
        .iter()
        .map(|&(x, y, sample, dimension)| {
            let bits = PixelStream::new(MIXED_SEED, x, y).bits(sample, dimension);
            let point = point_raw(&PixelSampler::new(MIXED_SEED, x, y), sample, dimension);
            (x, y, sample, dimension, bits, point)
        })
        .collect();
    out.sort_unstable();
    out
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_014_values_do_not_depend_on_evaluation_order() {
    let mut keys = Vec::new();
    for y in 0..8 {
        for x in 0..8 {
            for sample in 0..16 {
                for dimension in 0..4 {
                    keys.push((x, y, sample, dimension));
                }
            }
        }
    }
    let forward = draws(&keys);
    keys.reverse();
    let reverse = draws(&keys);
    Seeds(SAMPLER_TEST_SEED).shuffle(&mut keys);
    let shuffled = draws(&keys);
    assert_eq!(forward, reverse);
    assert_eq!(forward, shuffled);
    // Reused handles give the same values as fresh ones.
    let stream = PixelStream::new(MIXED_SEED, 3, 4);
    let sampler = PixelSampler::new(MIXED_SEED, 3, 4);
    for &(x, y, sample, dimension, bits, point) in &forward {
        if (x, y) == (3, 4) {
            assert_eq!(stream.bits(sample, dimension), bits);
            assert_eq!(point_raw(&sampler, sample, dimension), point);
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_015_key_fields_are_domain_separated() {
    // Swapped coordinates and swapped sample/dimension never meet by construction.
    let stream = |x, y| PixelStream::new(0, x, y);
    assert_ne!(stream(1, 0).bits(0, 0), stream(0, 1).bits(0, 0));
    assert_ne!(stream(0, 0).bits(1, 0), stream(0, 0).bits(0, 1));
    // For a fixed seed and (sample, dimension) the map from pixel to bits is injective, and for a
    // fixed pixel the map from (sample, dimension) to bits is injective: check a 64 x 64 grid of
    // each, including the top of each field's range.
    let edge = |i: u32| if i < 32 { i } else { u32::MAX - (i - 32) };
    let mut pixel_bits = Vec::new();
    let mut sample_bits = Vec::new();
    let mut sampler_points = Vec::new();
    for a in 0..64 {
        for b in 0..64 {
            pixel_bits.push(PixelStream::new(MIXED_SEED, edge(a), edge(b)).bits(7, 1));
            sample_bits.push(PixelStream::new(MIXED_SEED, 7, 1).bits(edge(a), edge(b)));
            sampler_points.push(point_raw(&PixelSampler::new(MIXED_SEED, edge(a), edge(b)), 0, 0));
        }
    }
    for (name, count) in [
        ("pixel", distinct(&mut pixel_bits)),
        ("sample", distinct(&mut sample_bits)),
        ("sampler pixel", distinct(&mut sampler_points)),
    ] {
        assert_eq!(count, 64 * 64, "{name} keys collide");
    }
    // The sampler and the hash use separate domains: the sampler's first point is not the
    // hash of the same key.
    let point = point_raw(&PixelSampler::new(0, 0, 0), 0, 0);
    let bits = PixelStream::new(0, 0, 0).bits(0, 0);
    assert_ne!(u64::from(point.0) << 32 | u64::from(point.1), bits);
}

fn distinct<T: Ord>(values: &mut Vec<T>) -> usize {
    values.sort_unstable();
    values.dedup();
    values.len()
}

/// The 192 keys one bit away from `key`: each seed bit, then each bit of x, y, sample and
/// dimension.
fn flipped_keys((seed, x, y, s, d): Key) -> Vec<Key> {
    let mut keys: Vec<Key> = (0..64).map(|bit| (seed ^ (1 << bit), x, y, s, d)).collect();
    for bit in 0..32 {
        keys.push((seed, x ^ (1 << bit), y, s, d));
    }
    for bit in 0..32 {
        keys.push((seed, x, y ^ (1 << bit), s, d));
    }
    for bit in 0..32 {
        keys.push((seed, x, y, s ^ (1 << bit), d));
    }
    for bit in 0..32 {
        keys.push((seed, x, y, s, d ^ (1 << bit)));
    }
    keys
}

/// For each of the 192 key bits, the number of output bits that change when it flips, summed
/// over `keys`.
fn avalanche_counts(keys: &[Key]) -> Vec<u32> {
    let bits = |(seed, x, y, s, d): Key| PixelStream::new(seed, x, y).bits(s, d);
    let mut counts = vec![0_u32; 192];
    for &key in keys {
        let base = bits(key);
        for (count, flipped) in counts.iter_mut().zip(flipped_keys(key)) {
            *count += (base ^ bits(flipped)).count_ones();
        }
    }
    counts
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_016_adjacent_keys_avalanche() {
    // A regression guard on the mixing, not a statistical certification: for each of the 192
    // key bits, flipping it changes on average between 28 and 36 of the 64 output bits over
    // 256 seeded keys (an ideal mix gives 32, with a standard deviation of 0.25 for the mean).
    let mut seeds = Seeds(SAMPLER_TEST_SEED ^ 0x16);
    let keys: Vec<Key> = (0..256)
        .map(|_| {
            (seeds.next(), seeds.next_u32(), seeds.next_u32(), seeds.next_u32(), seeds.next_u32())
        })
        .collect();
    for (bit, count) in avalanche_counts(&keys).into_iter().enumerate() {
        let total = u32::try_from(keys.len()).expect("fits") * 64;
        assert!(
            (28 * total / 64..=36 * total / 64).contains(&count),
            "key bit {bit}: {count} of {total} output bits flipped"
        );
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_017_dimension_pairs_and_pixels_are_decorrelated() {
    // Sanity guard, not a proof of independence. Pair 0 against pair 1 over 1024 samples of one
    // pixel: each cell of a 4 x 4 grid of (u0, u1) holds between 32 and 96 of the expected 64.
    for (seed, x, y) in [(0, 0, 0), (MIXED_SEED, 1919, 1079), (u64::MAX, 1, 2)] {
        let sampler = PixelSampler::new(seed, x, y);
        let mut cells = [0_u32; 16];
        for sample in 0..1024 {
            let (u0, _) = point_raw(&sampler, sample, 0);
            let (u1, _) = point_raw(&sampler, sample, 1);
            cells[usize::try_from((u0 >> 30) << 2 | u1 >> 30).expect("fits")] += 1;
        }
        assert!(cells.iter().all(|c| (32..=96).contains(c)), "pixel ({x}, {y}): {cells:?}");
    }
    // Neighbouring pixels: no sample of a 16 x 16 block shares its first point with the same
    // sample of the pixel to its right or below.
    for y in 0..16 {
        for x in 0..16 {
            let here = PixelSampler::new(MIXED_SEED, x, y);
            for next in
                [PixelSampler::new(MIXED_SEED, x + 1, y), PixelSampler::new(MIXED_SEED, x, y + 1)]
            {
                for sample in 0..16 {
                    assert_ne!(
                        point_raw(&here, sample, 0),
                        point_raw(&next, sample, 0),
                        "({x}, {y})"
                    );
                }
            }
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_018_camera_samples_are_accepted_by_the_camera() {
    let camera = PinholeCamera::new(&CameraSpec {
        position: FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO),
        look: FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::from_raw(-ONE_RAW)),
        up: FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ONE, FixedQ32_32::ZERO),
        tan_half_fov: FixedQ32_32::ONE,
        width: 16,
        height: 9,
    })
    .expect("camera");
    for y in 0..9 {
        for x in 0..16 {
            let sampler = PixelSampler::new(MIXED_SEED, x, y);
            for sample in 0..16 {
                let pixel = sampler.camera_sample(sample);
                assert_eq!((pixel.x, pixel.y), (x, y));
                assert!(camera.ray(pixel).is_some(), "({x}, {y}) sample {sample}");
            }
        }
    }
}
