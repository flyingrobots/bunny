use bunny_num::FixedQ32_32;

/// Stable identity of the decimal-text to Q32.32 conversion rule.
pub const DECIMAL_Q32_32_ID: &str = "bunny-decimal-q32-32/1";

/// Most significant digits the conversion handles when the value has a fractional part.
pub const MAX_FRACTIONAL_SIGNIFICANT_DIGITS: usize = 28;

/// A value in `[10^(m - 1), 10^m)` with `m` at least this is at least 1e10, above 2^31.
const CERTAIN_OVERFLOW_MAGNITUDE: i128 = 11;
/// A value below `10^m` with `m` at most this is below 1e-10, which is below 2^-33
/// (half a raw unit), so it rounds to zero without a tie.
const CERTAIN_UNDERFLOW_MAGNITUDE: i128 = -10;
/// Saturation bound for exponent digits; far beyond any decision boundary.
const EXPONENT_CAP: i128 = 1 << 100;

/// Error returned when decimal text cannot be converted to Q32.32.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecimalError {
    /// The text is not `[+-] digits [. digits] [(e|E) [+-] digits]` with a digit in the mantissa.
    Invalid,
    /// The rounded value does not fit an `i64` raw value.
    Overflow,
    /// The value has more than 28 significant digits and cannot be decided exactly.
    TooLong,
}

struct Syntax<'a> {
    negative: bool,
    int: &'a [u8],
    frac: &'a [u8],
    exponent: i128,
}

struct Shape {
    leading_zeros: usize,
    significant: usize,
    exponent10: i128,
}

/// Converts decimal text to Q32.32 exactly, with integer arithmetic only.
///
/// The conversion rule, `bunny-decimal-q32-32/1`, is the exact rational value of
/// the text times 2^32, rounded to the nearest integer with ties to even.
///
/// # Errors
/// Returns `DecimalError::Invalid` for text outside the grammar,
/// `DecimalError::Overflow` when the rounded raw value is outside `i64`, and
/// `DecimalError::TooLong` when more than 28 significant digits must be read
/// to decide a value whose magnitude is in `[1e-10, 1e10)`.
pub fn parse_decimal_q32_32(text: &[u8]) -> Result<FixedQ32_32, DecimalError> {
    let syntax = Syntax::parse(text)?;
    let Some(shape) = syntax.shape() else {
        return Ok(FixedQ32_32::ZERO);
    };
    let magnitude = shape.magnitude();
    if magnitude >= CERTAIN_OVERFLOW_MAGNITUDE {
        return Err(DecimalError::Overflow);
    }
    if magnitude <= CERTAIN_UNDERFLOW_MAGNITUDE {
        return Ok(FixedQ32_32::ZERO);
    }
    if shape.significant > MAX_FRACTIONAL_SIGNIFICANT_DIGITS {
        return Err(DecimalError::TooLong);
    }
    let raw = scaled_magnitude(syntax.significand(&shape), shape.exponent10);
    signed_raw(raw, syntax.negative)
}

impl<'a> Syntax<'a> {
    fn parse(text: &'a [u8]) -> Result<Self, DecimalError> {
        let (negative, rest) = match text.split_first() {
            Some((b'-', rest)) => (true, rest),
            Some((b'+', rest)) => (false, rest),
            _ => (false, text),
        };
        let (mantissa, exponent_text) = split_at_byte(rest, |byte| matches!(byte, b'e' | b'E'));
        let (int, frac) = split_at_byte(mantissa, |byte| byte == b'.');
        let digits_ok = all_digits(int) && all_digits(frac.unwrap_or_default());
        if !digits_ok || (int.is_empty() && frac.is_none_or(<[u8]>::is_empty)) {
            return Err(DecimalError::Invalid);
        }
        let exponent = parse_exponent(exponent_text)?;
        Ok(Self { negative, int, frac: frac.unwrap_or_default(), exponent })
    }

    fn digits(&self) -> impl DoubleEndedIterator<Item = &u8> + Clone {
        self.int.iter().chain(self.frac)
    }

    fn shape(&self) -> Option<Shape> {
        let total = self.int.len() + self.frac.len();
        let leading_zeros = self.digits().take_while(|byte| **byte == b'0').count();
        if leading_zeros == total {
            return None;
        }
        let trailing_zeros = self.digits().rev().take_while(|byte| **byte == b'0').count();
        let fraction_digits = to_i128(self.frac.len()) - to_i128(trailing_zeros);
        Some(Shape {
            leading_zeros,
            significant: total - leading_zeros - trailing_zeros,
            exponent10: self.exponent - fraction_digits,
        })
    }

    fn significand(&self, shape: &Shape) -> u128 {
        self.digits()
            .skip(shape.leading_zeros)
            .take(shape.significant)
            .fold(0_u128, |acc, byte| acc * 10 + u128::from(byte - b'0'))
    }
}

impl Shape {
    /// The value lies in `[10^(magnitude - 1), 10^magnitude)`.
    fn magnitude(&self) -> i128 {
        to_i128(self.significant) + self.exponent10
    }
}

fn split_at_byte(input: &[u8], is_separator: impl Fn(u8) -> bool) -> (&[u8], Option<&[u8]>) {
    input.iter().position(|byte| is_separator(*byte)).map_or((input, None), |index| {
        (input.get(..index).unwrap_or_default(), input.get(index + 1..))
    })
}

fn all_digits(input: &[u8]) -> bool {
    input.iter().all(u8::is_ascii_digit)
}

fn parse_exponent(text: Option<&[u8]>) -> Result<i128, DecimalError> {
    let Some(text) = text else {
        return Ok(0);
    };
    let (negative, digits) = match text.split_first() {
        Some((b'-', rest)) => (true, rest),
        Some((b'+', rest)) => (false, rest),
        _ => (false, text),
    };
    if digits.is_empty() || !all_digits(digits) {
        return Err(DecimalError::Invalid);
    }
    let magnitude = digits.iter().fold(0_i128, |acc, byte| {
        acc.saturating_mul(10).saturating_add(i128::from(byte - b'0')).min(EXPONENT_CAP)
    });
    Ok(if negative { -magnitude } else { magnitude })
}

fn to_i128(value: usize) -> i128 {
    i128::try_from(value).unwrap_or(i128::MAX)
}

/// `round(significand * 10^exponent10 * 2^32)`, nearest with ties to even.
///
/// The caller guarantees `significand < 10^28` and `-37 <= exponent10 <= 9`, so
/// every intermediate value fits `u128`.
fn scaled_magnitude(significand: u128, exponent10: i128) -> u128 {
    let power = 10_u128.pow(exponent10.unsigned_abs().try_into().unwrap_or(0));
    if exponent10 >= 0 {
        (significand * power) << 32
    } else {
        round_half_even_div(significand << 32, power)
    }
}

fn round_half_even_div(numerator: u128, denominator: u128) -> u128 {
    let quotient = numerator / denominator;
    let doubled_remainder = (numerator % denominator) * 2;
    match doubled_remainder.cmp(&denominator) {
        std::cmp::Ordering::Greater => quotient + 1,
        std::cmp::Ordering::Equal => quotient + (quotient & 1),
        std::cmp::Ordering::Less => quotient,
    }
}

fn signed_raw(magnitude: u128, negative: bool) -> Result<FixedQ32_32, DecimalError> {
    let wide = i128::try_from(magnitude).map_err(|_| DecimalError::Overflow)?;
    let signed = if negative { -wide } else { wide };
    i64::try_from(signed).map(FixedQ32_32::from_raw).map_err(|_| DecimalError::Overflow)
}
