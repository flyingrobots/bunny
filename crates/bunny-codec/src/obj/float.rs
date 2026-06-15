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
    let mut value = 0.0_f64;
    let mut saw_digit = false;
    let mut fractional_digits = 0_i32;

    while let Some(digit) = decimal_digit(bytes.get(index).copied()) {
        value = value.mul_add(10.0, f64::from(digit));
        saw_digit = true;
        index += 1;
    }

    if bytes.get(index) == Some(&b'.') {
        index += 1;
        while let Some(digit) = decimal_digit(bytes.get(index).copied()) {
            value = value.mul_add(10.0, f64::from(digit));
            saw_digit = true;
            fractional_digits = fractional_digits.saturating_add(1);
            index += 1;
        }
    }

    if !saw_digit {
        return None;
    }

    let exponent = parse_float_exponent(bytes, &mut index)? - fractional_digits;
    if index == bytes.len() {
        Some((sign * value * 10_f64.powi(exponent)) as f32)
    } else {
        None
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
