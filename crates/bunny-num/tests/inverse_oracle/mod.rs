//! An independent high-precision oracle for `asin` and `acos`, built on `angle_oracle`.
//!
//! The cosine leg `sqrt(1 - x^2)` is the exact floor of the square root of a 248-bit
//! number, found by integer Newton iteration over `angle_oracle::ratio` (binary long
//! division) from above, with 124 fractional bits. The angle is then `atan` by its Taylor
//! series in `angle_oracle`. The kernel instead takes `u128::isqrt` of a 128-bit number
//! with 32 extra bits and runs CORDIC, so the two share no algorithm, constant or helper.
//! Each value is within `2^-50` of an angle unit of the exact angle.

use crate::angle_oracle::{ratio, Oracle, EXTRA_BITS, FRAC, ONE};

/// A full turn in the oracle's angle units, `2^-(64 + EXTRA_BITS)` turn.
const FULL: u128 = 1 << (64 + EXTRA_BITS);

/// `floor(sqrt(v))` with `FRAC` fractional bits, for `v` in `[0, 1]` with `FRAC` bits.
///
/// Integer Newton iteration on `N = v 2^FRAC` from `2^FRAC >= sqrt(N)`; from above it
/// decreases strictly until it reaches `floor(sqrt(N))`.
fn sqrt_q124(v: u128) -> u128 {
    if v == 0 {
        return 0;
    }
    let mut root = ONE;
    loop {
        let next = u128::midpoint(root, ratio(v, root, FRAC));
        if next >= root {
            return root;
        }
        root = next;
    }
}

/// `(|x|, sqrt(1 - x^2))` with `FRAC` fractional bits for raw Q32.32 `x`, `|x| <= 1`.
fn legs(raw: i64) -> (u128, u128) {
    let magnitude = u128::from(raw.unsigned_abs());
    assert!(magnitude <= 1 << 32, "the oracle is defined for |x| <= 1 only");
    let square = (magnitude * magnitude) << (FRAC - 64);
    (magnitude << (FRAC - 32), sqrt_q124(ONE - square))
}

/// The angle of `(along, across)` in `[0, 1/4]` turn, both non-negative and not both zero.
fn first_quadrant(oracle: &Oracle, across: u128, along: u128) -> u128 {
    if across <= along {
        oracle.turn_units(oracle.atan(across, along))
    } else {
        FULL / 4 - oracle.turn_units(oracle.atan(along, across))
    }
}

/// `asin(raw / 2^32)` in units of `2^-(64 + EXTRA_BITS)` turn, in `[0, FULL)`.
pub(crate) fn asin(oracle: &Oracle, raw: i64) -> u128 {
    let (sine, cosine) = legs(raw);
    let angle = first_quadrant(oracle, sine, cosine);
    if raw < 0 {
        (FULL - angle) % FULL
    } else {
        angle
    }
}

/// `acos(raw / 2^32)` in units of `2^-(64 + EXTRA_BITS)` turn, in `[0, FULL / 2]`.
pub(crate) fn acos(oracle: &Oracle, raw: i64) -> u128 {
    let (cosine, sine) = legs(raw);
    let angle = first_quadrant(oracle, sine, cosine);
    if raw < 0 {
        FULL / 2 - angle
    } else {
        angle
    }
}
