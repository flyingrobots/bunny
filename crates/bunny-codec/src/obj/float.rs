const MAX_MANTISSA_DIGITS: i32 = 19;

#[allow(clippy::cast_possible_truncation)]
pub(super) fn parse_ascii_float(text: &str) -> Option<f32> {
    if text.eq_ignore_ascii_case("nan") {
        return Some(f32::NAN);
    }
    if text.eq_ignore_ascii_case("inf")
        || text.eq_ignore_ascii_case("+inf")
        || text.eq_ignore_ascii_case("infinity")
        || text.eq_ignore_ascii_case("+infinity")
    {
        return Some(f32::INFINITY);
    }
    if text.eq_ignore_ascii_case("-inf") || text.eq_ignore_ascii_case("-infinity") {
        return Some(f32::NEG_INFINITY);
    }

    let bytes = text.as_bytes();
    let mut index = 0;
    let sign = parse_float_sign(bytes, &mut index);
    let mut mantissa = ParsedMantissa::default();

    while let Some(digit) = decimal_digit(bytes.get(index).copied()) {
        mantissa.push_integer_digit(digit);
        index += 1;
    }

    if bytes.get(index) == Some(&b'.') {
        index += 1;
        while let Some(digit) = decimal_digit(bytes.get(index).copied()) {
            mantissa.push_fractional_digit(digit);
            index += 1;
        }
    }

    if !mantissa.saw_digit {
        return None;
    }

    let exponent = mantissa.adjust_exponent(parse_float_exponent(bytes, &mut index)?);
    if index == bytes.len() {
        let value = if mantissa.value == 0.0 {
            sign * 0.0
        } else {
            sign * mantissa.value * 10_f64.powi(exponent)
        };
        Some(value as f32)
    } else {
        None
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct ParsedMantissa {
    value: f64,
    saw_digit: bool,
    kept_digits: i32,
    skipped_digits: i32,
    fractional_digits: i32,
}

impl ParsedMantissa {
    fn push_integer_digit(&mut self, digit: u8) {
        self.push_digit(digit);
    }

    fn push_fractional_digit(&mut self, digit: u8) {
        self.fractional_digits = self.fractional_digits.saturating_add(1);
        self.push_digit(digit);
    }

    fn push_digit(&mut self, digit: u8) {
        self.saw_digit = true;
        if self.kept_digits == 0 && digit == 0 {
            return;
        }
        if self.kept_digits < MAX_MANTISSA_DIGITS {
            self.value = self.value.mul_add(10.0, f64::from(digit));
            self.kept_digits += 1;
        } else {
            self.skipped_digits = self.skipped_digits.saturating_add(1);
        }
    }

    const fn adjust_exponent(self, exponent: i32) -> i32 {
        exponent
            .saturating_add(self.skipped_digits)
            .saturating_sub(self.fractional_digits)
    }
}

fn parse_float_sign(bytes: &[u8], index: &mut usize) -> f64 {
    match bytes.get(*index) {
        Some(b'-') => {
            *index += 1;
            -1.0
        }
        Some(b'+') => {
            *index += 1;
            1.0
        }
        _ => 1.0,
    }
}

fn parse_float_exponent(bytes: &[u8], index: &mut usize) -> Option<i32> {
    if !matches!(bytes.get(*index), Some(b'e' | b'E')) {
        return Some(0);
    }
    *index += 1;

    let sign = match bytes.get(*index) {
        Some(b'-') => {
            *index += 1;
            -1
        }
        Some(b'+') => {
            *index += 1;
            1
        }
        _ => 1,
    };

    let mut exponent = 0_i32;
    let mut saw_digit = false;
    while let Some(digit) = decimal_digit(bytes.get(*index).copied()) {
        exponent = exponent.saturating_mul(10).saturating_add(i32::from(digit));
        saw_digit = true;
        *index += 1;
    }
    saw_digit.then_some(sign * exponent)
}

const fn decimal_digit(byte: Option<u8>) -> Option<u8> {
    match byte {
        Some(value @ b'0'..=b'9') => Some(value - b'0'),
        _ => None,
    }
}
