//! `bunny-srgb8-decode/1` contract tests (`docs/topics/optics/test-plan.md`, OPTICS-TP-120
//! to 122): every table entry against an exact integer decision of "the sRGB decode rounded to
//! nearest", the `bunny-tone-srgb/1` round trip, and the table's golden and boundary values.
//!
//! The oracle uses no floating point and shares no code with the table's generator.

use core::cmp::Ordering;

use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_optics::{srgb8_to_linear, ToneCurve, SRGB8_DECODE_IDENTITY};
use wasm_bindgen_test::wasm_bindgen_test;

/// The largest code on the linear toe: `10 / 255 <= 0.04045 < 11 / 255`.
const LAST_TOE_CODE: u32 = 10;

/// A non-negative integer of any size, as little-endian 32-bit limbs; only what the oracle
/// needs: multiplication by a `u64` and comparison.
#[derive(Clone, Debug)]
struct Big(Vec<u32>);

impl Big {
    fn from_u64(value: u64) -> Self {
        let mut big = Self(vec![u32::try_from(value & 0xFFFF_FFFF).unwrap_or(0)]);
        big.0.push(u32::try_from(value >> 32).unwrap_or(0));
        big.trim();
        big
    }

    fn trim(&mut self) {
        while self.0.len() > 1 && self.0.last() == Some(&0) {
            self.0.pop();
        }
    }

    fn mul_u64(&mut self, factor: u64) {
        let mut carry: u128 = 0;
        for limb in &mut self.0 {
            let product = u128::from(*limb) * u128::from(factor) + carry;
            *limb = u32::try_from(product & 0xFFFF_FFFF).unwrap_or(0);
            carry = product >> 32;
        }
        while carry > 0 {
            self.0.push(u32::try_from(carry & 0xFFFF_FFFF).unwrap_or(0));
            carry >>= 32;
        }
    }

    fn mul_pow(&mut self, base: u64, exponent: u32) {
        for _ in 0..exponent {
            self.mul_u64(base);
        }
    }

    fn mul_pow2(&mut self, bits: u32) {
        for _ in 0..bits {
            self.mul_u64(2);
        }
    }

    fn cmp_big(&self, other: &Self) -> Ordering {
        let (mut a, mut b) = (self.clone(), other.clone());
        a.trim();
        b.trim();
        a.0.len().cmp(&b.0.len()).then_with(|| a.0.iter().rev().cmp(b.0.iter().rev()))
    }
}

/// Where `x = 2^32 * decode(code)` lies against the half-integer `odd / 2`, decided exactly.
///
/// - On the toe, `x = code * 100 * 2^32 / 329 460` (`255 * 12.92 = 3 294.6`, scaled by 100 and
///   10), so `x` against `odd / 2` is `200 * code * 2^32` against `odd * 329 460`.
/// - Above it, `x = 2^32 * u^(12/5)` with `u = N / D`, `N = 1000 * code + 14 025` and
///   `D = 269 025` (`255 * 1.055 = 269.025`). Raising both sides to the fifth power (monotone
///   on the positives), `x` against `odd / 2` is `2^165 * N^12` against `odd^5 * D^12`.
fn compare_to_half(code: u32, odd: u64) -> Ordering {
    if code <= LAST_TOE_CODE {
        let mut x = Big::from_u64(200 * u64::from(code));
        x.mul_pow2(32);
        let mut half = Big::from_u64(odd);
        half.mul_u64(329_460);
        x.cmp_big(&half)
    } else {
        let numerator = 1000 * u64::from(code) + 14_025;
        let mut x = Big::from_u64(1);
        x.mul_pow(numerator, 12);
        x.mul_pow2(165);
        let mut half = Big::from_u64(1);
        half.mul_pow(odd, 5);
        half.mul_pow(269_025, 12);
        x.cmp_big(&half)
    }
}

/// Whether `n` is `x` rounded to nearest: `n - 1/2 < x < n + 1/2`. `x` is never exactly a
/// half-integer (the oracle asserts it), so the ties-to-even rule is never needed.
fn is_nearest(code: u32, n: u64) -> bool {
    let above_lower = n == 0 || compare_to_half(code, 2 * n - 1) == Ordering::Greater;
    let below_upper = compare_to_half(code, 2 * n + 1) == Ordering::Less;
    above_lower && below_upper
}

fn entry(code: u8) -> i64 {
    srgb8_to_linear(code).raw()
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_120_every_entry_is_the_exact_decode_rounded_to_nearest() {
    for code in 0..=255u8 {
        let raw = entry(code);
        let n = raw.unsigned_abs();
        let c = u32::from(code);
        assert!(raw >= 0 && is_nearest(c, n), "entry {code} = {raw} is not the nearest raw value");
        // The oracle can fail: both neighbours of the right answer are rejected.
        assert!(!is_nearest(c, n + 1), "entry {code}: the oracle accepted {}", n + 1);
        assert!(n == 0 || !is_nearest(c, n - 1), "entry {code}: the oracle accepted {}", n - 1);
        // No tie: x is on neither side's half-integer boundary.
        assert_ne!(compare_to_half(c, 2 * n + 1), Ordering::Equal, "entry {code} is a tie");
        assert_ne!(compare_to_half(c, 2 * n.max(1) - 1), Ordering::Equal, "entry {code} is a tie");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_121_tone_srgb_encodes_every_entry_back_to_its_code() {
    for code in 0..=255u8 {
        let linear = srgb8_to_linear(code);
        assert_eq!(ToneCurve::SrgbV1.encode_8(linear), code, "code {code} (raw {})", linear.raw());
    }
}

/// `(code, raw)` from the generator's 80-digit decimal run, at the ends and around the toe's
/// end (code 10 is the last linear entry).
const GOLDEN: &[(u8, i64)] =
    &[(0, 0), (1, 1_303_638), (10, 13_036_385), (11, 14_373_262), (255, 4_294_967_296)];

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_122_identity_ends_order_and_golden_entries() {
    assert_eq!(SRGB8_DECODE_IDENTITY, "bunny-srgb8-decode/1");
    assert_eq!(entry(0), 0);
    assert_eq!(entry(255), ONE_RAW);
    assert_eq!(srgb8_to_linear(255), FixedQ32_32::ONE);
    for code in 0..255u8 {
        assert!(entry(code) < entry(code + 1), "entries increase at {code}");
    }
    for &(code, raw) in GOLDEN {
        assert_eq!(entry(code), raw, "golden entry {code}");
    }
}
