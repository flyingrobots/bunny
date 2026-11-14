//! Golden vectors for `Xoroshiro128Plus`, the sequential generator kept bit for bit compatible
//! with Echo's `warp-math` `Prng`: Echo's own golden values, the published `SplitMix64`
//! reference outputs behind its single-seed constructor, and its zero-seed and range rules.

use bunny_num::FixedQ32_32;
use bunny_optics::Xoroshiro128Plus;
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_019_echo_golden_vectors_replay() {
    // Echo's `prng_golden_regression.rs`: three draws over the whole i32 range.
    let mut prng = Xoroshiro128Plus::from_seed(0xDEAD_BEEF, 0xFACE_FEED);
    let values: Vec<Option<i32>> = (0..3).map(|_| prng.next_int(i32::MIN, i32::MAX)).collect();
    assert_eq!(values, [Some(1_501_347_292), Some(1_946_982_111), Some(-117_316_573)]);

    // Echo's `math-fixtures.json` prng fixture. Echo's `next_f32` returns
    // `(next_u64 >> 41) * 2^-23`, here exactly as Q32.32: 0.000014305115, 0.49138594 and
    // 0.75950694 as f32 are 120, 4_122_044 and 6_371_206 times 2^-23.
    let seed = (173_476_091_849_832, 91_255_900_676_441);
    let mut prng = Xoroshiro128Plus::from_seed(seed.0, seed.1);
    let units: Vec<FixedQ32_32> = (0..3).map(|_| prng.next_unit()).collect();
    let expected = [120_i64, 4_122_044, 6_371_206].map(|k| FixedQ32_32::from_raw(k << 9));
    assert_eq!(units, expected);
    let mut prng = Xoroshiro128Plus::from_seed(seed.0, seed.1);
    let ints: Vec<Option<i32>> = (0..3).map(|_| prng.next_int(5, 10)).collect();
    assert_eq!(ints, [Some(10), Some(10), Some(6)]);
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_020_seeding_and_range_rules() {
    // `from_seed_u64` expands the seed with SplitMix64; these are its published first two
    // outputs for the seed 1234567.
    let prng = Xoroshiro128Plus::from_seed_u64(1_234_567);
    assert_eq!(prng.state(), [6_457_827_717_110_365_317, 3_203_168_211_198_807_973]);
    let mut prng = Xoroshiro128Plus::from_seed_u64(1_234_567);
    let draws: Vec<u64> = (0..3).map(|_| prng.next_u64()).collect();
    assert_eq!(
        draws,
        [9_660_995_928_309_173_290, 13_057_124_114_940_788_888, 1_483_754_980_695_055_771]
    );
    let mut prng = Xoroshiro128Plus::from_seed_u64(0);
    assert_eq!([prng.next_u64(), prng.next_u64()], [0x5099_46A4_1CD7_33A3, 0x0088_5667_B193_4BFA]);

    // An all-zero seed is replaced, so the state is never the all-zero sink.
    let mut prng = Xoroshiro128Plus::from_seed(0, 0);
    assert_eq!(prng.state(), [0x9E37_79B9_7F4A_7C15, 0]);
    let draws: Vec<u64> = (0..3).map(|_| prng.next_u64()).collect();
    assert_eq!(
        draws,
        [11_400_714_819_323_198_485, 4_557_078_984_430_990_530, 5_559_460_909_137_937_373]
    );

    // A one-value range returns it without drawing; an empty range is refused, not a panic.
    let mut prng = Xoroshiro128Plus::from_seed(1, 2);
    assert_eq!(prng.next_int(0, 0), Some(0));
    assert_eq!(prng.state(), [1, 2]);
    assert_eq!(prng.next_int(1, 0), None);
    assert_eq!(prng.state(), [1, 2]);
    let values: Vec<Option<i32>> =
        [(-3, 3), (-3, 3), (0, 1 << 20)].iter().map(|&(a, b)| prng.next_int(a, b)).collect();
    assert_eq!(values, [Some(0), Some(3), Some(727_076)]);
}
