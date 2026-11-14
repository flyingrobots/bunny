//! Integration tests for the decimal-text to Q32.32 conversion rule `bunny-decimal-q32-32/1`.
//!
//! Expected values are derived by hand in the comments or computed with exact
//! integer arithmetic. No floating point is used anywhere in this file.

use std::fmt::Write as _;

use bunny_codec::{parse_decimal_q32_32, DecimalError, DECIMAL_Q32_32_ID};
use wasm_bindgen_test::wasm_bindgen_test;

const ONE: i64 = 1 << 32;

fn convert(text: &str) -> Result<i64, DecimalError> {
    parse_decimal_q32_32(text.as_bytes()).map(bunny_num::FixedQ32_32::raw)
}

fn assert_all(cases: &[(&str, i64)]) {
    for (text, expected) in cases {
        assert_eq!(convert(text), Ok(*expected), "converting {text:?}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_020_rule_has_a_stable_identity() {
    assert_eq!(DECIMAL_Q32_32_ID, "bunny-decimal-q32-32/1");
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_021_whole_and_dyadic_values_are_exact() {
    // raw = value * 2^32, and these values are dyadic rationals, so no rounding occurs.
    assert_all(&[
        ("0", 0),
        ("1", ONE),
        ("-1", -ONE),
        ("0.5", ONE / 2),
        ("0.25", ONE / 4),
        ("1.5", ONE + ONE / 2),
        ("-1.5", -(ONE + ONE / 2)),
        ("125", 125 * ONE),
        ("65536", 65_536 * ONE),
    ]);
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_022_grammar_forms_agree_on_one_value() {
    // Every spelling below is exactly 1.2, and 1.2 * 2^32 = 5153960755.2 rounds to 5153960755.
    for text in
        ["1.2", "+1.2", "01.2", "1.20", "1.2e0", "1.2E+0", "12e-1", "0.12e1", "120e-2", "0.0012e3"]
    {
        assert_eq!(convert(text), Ok(5_153_960_755), "converting {text:?}");
    }
    // 0.1 * 2^32 = 429496729.6 rounds to 429496730; the sign mirrors it.
    assert_all(&[("0.1", 429_496_730), ("-0.1", -429_496_730), (".1", 429_496_730)]);
    // A digit is required in the mantissa, but either side of the point may be empty.
    assert_all(&[("5.", 5 * ONE), (".5", ONE / 2), ("-.5e1", -5 * ONE), ("5.e0", 5 * ONE)]);
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_023_exponents_scale_by_powers_of_ten() {
    // 1.25e2 = 125 and 125 * 2^32 = 536870912000.
    assert_all(&[("1.25e2", 536_870_912_000), ("1E1", 10 * ONE), ("100e-2", ONE), ("0.1e1", ONE)]);
    // 1e9 * 2^32 = 4294967296000000000, the largest power of ten in range.
    assert_all(&[("1e9", 4_294_967_296_000_000_000), ("1e+9", 4_294_967_296_000_000_000)]);
    // Zero stays zero for any exponent, however large, and so does -0.
    assert_all(&[("0", 0), ("-0", 0), ("0.0e5", 0), ("0e999999999999999999999999999999", 0)]);
    assert_all(&[("-0.000", 0), ("+0e-999999999999999999999999999999", 0)]);
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_024_rounding_is_to_nearest_with_ties_to_even() {
    // Constructed from raw units r = 2^-32: 1e-10 * 2^32 = 0.43 -> 0, 2e-10 -> 0.86 -> 1,
    // 4e-10 -> 1.72 -> 2, 5e-10 -> 2.15 -> 2, 6e-10 -> 2.58 -> 3, 1e-9 -> 4.29 -> 4.
    assert_all(&[
        ("0.0000000001", 0),
        ("0.0000000002", 1),
        ("0.0000000004", 2),
        ("0.0000000005", 2),
        ("6e-10", 3),
        ("1e-9", 4),
        ("-0.0000000001", 0),
        ("-0.0000000002", -1),
        ("-6e-10", -3),
    ]);
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_025_exact_ties_go_to_the_even_neighbour() {
    // (2n + 1) / 2^33 lies exactly halfway between n and n + 1 raw units. Since
    // 1 / 2^33 = 5^33 / 10^33 and 5^33 = 116415321826934814453125, the decimal
    // text of (2n + 1) / 2^33 is (2n + 1) * 5^33 with 33 fractional digits.
    assert_all(&[
        // n = 0: 1 * 5^33 -> tie between 0 and 1 -> 0 (even).
        ("0.000000000116415321826934814453125", 0),
        // n = 1: 3 * 5^33 = 349245965480804443359375 -> tie between 1 and 2 -> 2.
        ("0.000000000349245965480804443359375", 2),
        // n = 2: 5 * 5^33 = 582076609134674072265625 -> tie between 2 and 3 -> 2.
        ("0.000000000582076609134674072265625", 2),
        // n = 3: 7 * 5^33 = 814907252788543701171875 -> tie between 3 and 4 -> 4.
        ("0.000000000814907252788543701171875", 4),
        // The same ties with a negative sign mirror them.
        ("-0.000000000116415321826934814453125", 0),
        ("-0.000000000349245965480804443359375", -2),
        ("-0.000000000582076609134674072265625", -2),
        ("-0.000000000814907252788543701171875", -4),
    ]);
    // One digit either side of a tie decides the direction: 3 * 5^33 +- 1 in the last place.
    assert_all(&[
        ("0.000000000349245965480804443359374", 1),
        ("0.000000000349245965480804443359376", 2),
    ]);
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_026_range_edges_are_exact() {
    // 2^31 - 2^-32 is the largest value: raw 2^63 - 1. 2147483647.9999999998 is
    // below the tie (2^31 - 2^-33 = 2147483647.999999999883...) so it rounds to i64::MAX.
    assert_all(&[
        ("2147483647.9999999998", i64::MAX),
        ("-2147483648", i64::MIN),
        ("2147483647", 2_147_483_647 * ONE),
        // |v| = 2^31 + 1e-10 is 0.43 raw units above 2^63, which rounds back to 2^63 = -MIN.
        ("-2147483648.0000000001", i64::MIN),
        // The smallest nonzero magnitude is one raw unit, 2^-32 = 2.3283064365386962890625e-10.
        ("0.00000000023283064365386962890625", 1),
        ("-0.00000000023283064365386962890625", -1),
    ]);
    // Just above the tie, 2147483647.9999999999 is 2^63 - 0.57 -> rounds up to 2^63: overflow.
    assert_eq!(convert("2147483647.9999999999"), Err(DecimalError::Overflow));
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_027_values_outside_the_range_overflow() {
    for text in [
        "2147483648",
        "-2147483648.0000000002", // 2^63 + 0.86 -> rounds to 2^63 + 1, below i64::MIN
        "9999999999",
        "1e10",
        "-1e10",
        "1e400",
        "1e999999999999999999999999999999",
        "-1e999999999999999999999999999999",
        // Very long digit strings are decided by magnitude without being read.
        "123456789012345678901234567890123456789012345678901234567890",
        "123456789012345678901234567890.5",
    ] {
        assert_eq!(convert(text), Err(DecimalError::Overflow), "converting {text:?}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_028_tiny_values_round_to_zero_however_long() {
    // Below 1e-10 (less than half a raw unit, 2^-33 = 1.16e-10) the value is 0 and
    // the digits beyond are not read, so even 60 significant digits are fine.
    let long = format!("0.{}{}", "0".repeat(10), "1234567890".repeat(6));
    assert_eq!(convert(&long), Ok(0));
    assert_all(&[("1e-10", 0), ("9.99999999e-11", 0), ("1e-999999999999999999999999999999", 0)]);
    // 1.2e-10 is above 2^-33 = 1.164e-10, so it rounds to one raw unit.
    assert_all(&[("1.2e-10", 1), ("1.17e-10", 1)]);
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_029_twenty_eight_significant_digits_convert_and_more_are_refused() {
    // 28 significant digits: 1.234567890123456789012345678 * 2^32 = 5302428712.77..., which rounds to 5302428713.
    assert_eq!(convert("1.234567890123456789012345678"), Ok(5_302_428_713));
    // Trailing zeros are not significant digits, so this is the same 28-digit value.
    assert_eq!(convert("1.2345678901234567890123456780000000"), Ok(5_302_428_713));
    // 29 significant digits cannot be decided exactly.
    for text in [
        "1.2345678901234567890123456789",
        "-1.2345678901234567890123456789",
        "0.1234567890123456789012345678901",
        "1234567890.123456789012345678901",
        "0.000000000123456789012345678901234567890", // magnitude 1e-10 < v < 1e-9: still read
    ] {
        assert_eq!(convert(text), Err(DecimalError::TooLong), "converting {text:?}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_030_text_outside_the_grammar_is_invalid() {
    for text in [
        "", "+", "-", ".", "-.", "e5", ".e5", "1e", "1e+", "1e-", "1e5e3", "1.2.3", "--1", "+-1",
        "1,5", "0x10", "nan", "NaN", "inf", "-inf", "infinity", "1 ", " 1", "1_000", "１", "1e5.0",
        "1.5f", "1\r", "1e 5", "٣",
    ] {
        assert_eq!(convert(text), Err(DecimalError::Invalid), "converting {text:?}");
    }
}

/// Little-endian decimal digits, used only by the independent oracle below.
type Digits = Vec<u8>;

fn digits_of(text: &str) -> Digits {
    text.bytes().rev().map(|byte| byte - b'0').collect()
}

fn double_digits(digits: &Digits) -> Digits {
    let mut out = Vec::with_capacity(digits.len() + 1);
    let mut carry = 0;
    for digit in digits {
        let value = digit * 2 + carry;
        out.push(value % 10);
        carry = value / 10;
    }
    if carry > 0 {
        out.push(carry);
    }
    out
}

fn trim_high_zeros(mut digits: Digits) -> Digits {
    while digits.last() == Some(&0) {
        digits.pop();
    }
    digits
}

#[derive(Debug, PartialEq, Eq)]
enum Expected {
    Raw(i64),
    Overflow,
}

/// Independent oracle: decimal-string arithmetic with no integer division or shifting.
/// `digits` is the mantissa text without sign or point, and the value is `digits * 10^exp10`.
fn oracle(negative: bool, digits: &str, exp10: i64) -> Expected {
    let mut scaled = trim_high_zeros(digits_of(digits));
    for _ in 0..32 {
        scaled = double_digits(&scaled);
    }
    if scaled.is_empty() {
        return Expected::Raw(0);
    }
    let magnitude = match exp10 {
        e if e >= 0 => {
            if i64::try_from(scaled.len()).unwrap() + e > 40 {
                return Expected::Overflow;
            }
            let mut shifted = vec![0; usize::try_from(e).unwrap()];
            shifted.extend(&scaled);
            shifted
        }
        e => round_after_dropping(&scaled, usize::try_from(-e).unwrap_or(usize::MAX)),
    };
    to_expected(&trim_high_zeros(magnitude), negative)
}

/// Drops the lowest `k` digits and rounds half to even using the dropped digits.
fn round_after_dropping(scaled: &Digits, k: usize) -> Digits {
    if k > scaled.len() {
        return Vec::new(); // the value is below 10^-1 of a unit in the last place: rounds to zero
    }
    let (dropped, kept) = scaled.split_at(k);
    let mut kept = kept.to_vec();
    // Compare the dropped digits with 5 * 10^(k-1), most significant digit first.
    let half: Vec<u8> =
        std::iter::once(5).chain(std::iter::repeat_n(0, k.saturating_sub(1))).collect();
    let dropped_msb: Vec<u8> = dropped.iter().rev().copied().collect();
    let ordering = if k == 0 { std::cmp::Ordering::Less } else { dropped_msb.cmp(&half) };
    let odd = kept.first().is_some_and(|digit| digit % 2 == 1);
    if ordering.is_gt() || (ordering.is_eq() && odd) {
        add_one(&mut kept);
    }
    kept
}

fn add_one(digits: &mut Digits) {
    for digit in digits.iter_mut() {
        if *digit == 9 {
            *digit = 0;
        } else {
            *digit += 1;
            return;
        }
    }
    digits.push(1);
}

fn to_expected(magnitude: &Digits, negative: bool) -> Expected {
    // i64::MAX = 9223372036854775807 and |i64::MIN| = 9223372036854775808, as digit strings.
    let limit: Digits =
        digits_of(if negative { "9223372036854775808" } else { "9223372036854775807" });
    let within = match magnitude.len().cmp(&limit.len()) {
        std::cmp::Ordering::Less => true,
        std::cmp::Ordering::Greater => false,
        std::cmp::Ordering::Equal => {
            let high_first = |digits: &Digits| digits.iter().rev().copied().collect::<Vec<u8>>();
            high_first(magnitude) <= high_first(&limit)
        }
    };
    if !within {
        return Expected::Overflow;
    }
    let value = magnitude.iter().rev().fold(0_i128, |acc, digit| acc * 10 + i128::from(*digit));
    Expected::Raw(i64::try_from(if negative { -value } else { value }).unwrap())
}

struct Xorshift(u64);

impl Xorshift {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }

    fn digit_string(&mut self, len: u64) -> String {
        (0..len).map(|_| char::from(b'0' + u8::try_from(self.below(10)).unwrap())).collect()
    }
}

struct Generated {
    text: String,
    negative: bool,
    digits: String,
    exp10: i64,
    significant: usize,
}

fn generate(rng: &mut Xorshift) -> Generated {
    let int_len = rng.below(13);
    let frac_len = rng.below(31);
    // Sprinkle zeros so leading and trailing zero handling is exercised.
    let mut int = rng.digit_string(int_len);
    let mut frac = rng.digit_string(frac_len);
    if rng.below(4) == 0 {
        frac = format!("{frac}{}", "0".repeat(usize::try_from(rng.below(6)).unwrap()));
    }
    if rng.below(4) == 0 {
        int = format!("{}{int}", "0".repeat(usize::try_from(rng.below(4)).unwrap()));
    }
    if int.is_empty() && frac.is_empty() {
        int.push('0');
    }
    let negative = rng.below(2) == 0;
    let exponent =
        if rng.below(3) == 0 { Some(i64::try_from(rng.below(30)).unwrap() - 15) } else { None };
    let mut text = String::new();
    text.push_str(if negative { "-" } else { "" });
    text.push_str(&int);
    if !frac.is_empty() || rng.below(2) == 0 {
        text.push('.');
        text.push_str(&frac);
    }
    if let Some(exponent) = exponent {
        write!(text, "e{exponent}").unwrap();
    }
    let all = format!("{int}{frac}");
    let significant = all.trim_matches('0').len();
    let shift = i64::try_from(frac.len()).unwrap();
    Generated { text, negative, digits: all, exp10: exponent.unwrap_or(0) - shift, significant }
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_031_seeded_inputs_match_the_decimal_string_oracle() {
    let mut rng = Xorshift(0x9E37_79B9_7F4A_7C15);
    let (mut converted, mut overflowed, mut refused_long) = (0, 0, 0);
    for _ in 0..20_000 {
        let case = generate(&mut rng);
        let expected = oracle(case.negative, &case.digits, case.exp10);
        match (convert(&case.text), expected) {
            (Ok(raw), Expected::Raw(want)) => {
                assert_eq!(raw, want, "converting {:?}", case.text);
                converted += 1;
            }
            (Err(DecimalError::Overflow), Expected::Overflow) => overflowed += 1,
            (Err(DecimalError::TooLong), _) => {
                // Only more than 28 significant digits may be refused as too long.
                assert!(case.significant > 28, "{:?} is not too long", case.text);
                refused_long += 1;
            }
            (got, want) => panic!("converting {:?}: got {got:?}, oracle says {want:?}", case.text),
        }
    }
    // The generator must reach all three outcomes, or the test checks nothing.
    assert!(converted > 3_000 && overflowed > 100 && refused_long > 100);
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_032_every_tie_in_a_range_goes_to_even() {
    // (2n + 1) / 2^33 has the decimal digits of (2n + 1) * 5^33 with 33 fractional
    // digits, and sits exactly between raw n and n + 1: the even one must win.
    const FIVE_POW_33: u128 = 116_415_321_826_934_814_453_125;
    for n in 0_u128..2_000 {
        let digits = (FIVE_POW_33 * (2 * n + 1)).to_string();
        let text = format!("0.{digits:0>33}");
        let expected = i64::try_from(if n % 2 == 0 { n } else { n + 1 }).unwrap();
        assert_eq!(convert(&text), Ok(expected), "tie {n}: {text}");
        assert_eq!(oracle(false, &digits, -33), Expected::Raw(expected), "oracle tie {n}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_033_oracle_agrees_with_hand_derived_vectors() {
    // Guards the oracle itself against the hand-derived values above.
    assert_eq!(oracle(false, "1", -1), Expected::Raw(429_496_730));
    assert_eq!(oracle(true, "1", -1), Expected::Raw(-429_496_730));
    assert_eq!(oracle(false, "21474836479999999998", -10), Expected::Raw(i64::MAX));
    assert_eq!(oracle(false, "21474836479999999999", -10), Expected::Overflow);
    assert_eq!(oracle(true, "21474836480000000001", -10), Expected::Raw(i64::MIN));
    assert_eq!(oracle(true, "21474836480000000002", -10), Expected::Overflow);
    assert_eq!(oracle(false, "116415321826934814453125", -33), Expected::Raw(0));
    assert_eq!(oracle(false, "349245965480804443359375", -33), Expected::Raw(2));
}
