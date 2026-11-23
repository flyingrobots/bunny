//! The angle of a single ratio: `atan`, `asin` and `acos`, all through the `atan2` kernel.
//!
//! `atan(x)` is exactly `atan2(x, 1.0)`. `asin(x)` and `acos(x)` are the angles of the
//! directions `(sqrt(1 - x^2), x)` and `(x, sqrt(1 - x^2))`, with both legs carried 32 bits
//! wider than Q32.32 and the root the exact floor square root of the exact radicand.

use super::atan2::direction_angle;
use super::FixedAngle;
use crate::isqrt_u128;
use crate::FixedQ32_32;

/// `1.0` as raw Q32.32, the largest ratio magnitude `asin` and `acos` accept.
const ONE_MAGNITUDE: u128 = 1 << 32;

impl FixedAngle {
    /// The angle whose tangent is `ratio`: exactly `atan2(ratio, 1.0)`.
    ///
    /// The result is within `1/2 + 2^-9` angle units of the exact angle, the bound of
    /// [`Self::atan2`]. Every input is accepted. Read as a signed `i64`, the raw result is
    /// strictly inside `(-2^62, 2^62)`, a quarter turn either side of zero; it has the sign
    /// of `ratio`, so a negative ratio gives an angle just below a full turn. `atan(0)` is
    /// [`Self::ZERO`], `atan(±1.0)` exactly `±1/8` turn, and `atan(-x)` is exactly
    /// `atan(x).wrapping_neg()` for every `x` except `i64::MIN` raw, which has no negation.
    #[must_use]
    pub fn atan(ratio: FixedQ32_32) -> Self {
        let raw = ratio.raw();
        let across = u128::from(raw.unsigned_abs());
        Self(direction_angle(across, ONE_MAGNITUDE, false, raw < 0))
    }

    /// The angle whose sine is `sine`, in `[-1/4, 1/4]` turn read as a signed `i64`.
    ///
    /// Defined for `|sine| <= 1.0`, raw `[-2^32, 2^32]`; any other input returns `None`.
    /// The result is within `2/3` of an angle unit (`2^-64` turn) of the exact angle; it is
    /// not always the nearest. `asin(0)` is [`Self::ZERO`] and `asin(±1.0)` exactly `±1/4`
    /// turn. `asin(-x)` is exactly `asin(x).wrapping_neg()` and
    /// `asin(x).wrapping_add(acos(x))` is exactly [`Self::QUARTER_TURN`].
    #[must_use]
    pub fn asin(sine: FixedQ32_32) -> Option<Self> {
        let legs = Legs::of(sine)?;
        Some(Self(direction_angle(legs.ratio, legs.root, false, legs.negative)))
    }

    /// The angle whose cosine is `cosine`, in `[0, 1/2]` turn.
    ///
    /// Defined for `|cosine| <= 1.0`, raw `[-2^32, 2^32]`; any other input returns `None`.
    /// The result is within `2/3` of an angle unit of the exact angle; it is not always the
    /// nearest. `acos(1.0)` is [`Self::ZERO`], `acos(0)` exactly a quarter turn and
    /// `acos(-1.0)` exactly [`Self::HALF_TURN`]. `acos(-x)` is exactly
    /// `HALF_TURN - acos(x)`. The half turn read as a signed `i64` is `-1/2`; read the raw
    /// value unsigned, or with [`Self::to_turns`], to keep `acos` in `[0, 1/2]`.
    #[must_use]
    pub fn acos(cosine: FixedQ32_32) -> Option<Self> {
        let legs = Legs::of(cosine)?;
        Some(Self(direction_angle(legs.root, legs.ratio, legs.negative, false)))
    }
}

/// The two legs of the unit right triangle with one leg `|x|`, scaled by `2^64`.
struct Legs {
    /// `|x| 2^64`: the raw magnitude shifted left 32 bits, at most `2^64`.
    ratio: u128,
    /// `floor(sqrt(1 - x^2) 2^64)`, at most `2^64`; any positive value when `x` is zero.
    root: u128,
    negative: bool,
}

impl Legs {
    /// The legs of `x`, or `None` outside `|x| <= 1.0`.
    const fn of(x: FixedQ32_32) -> Option<Self> {
        let raw = x.raw();
        let magnitude = raw.unsigned_abs() as u128;
        if magnitude > ONE_MAGNITUDE {
            return None;
        }
        // (1 - x^2) 2^128 = (2^64 - raw^2) 2^64: exact, and below 2^128 once raw != 0. At
        // raw == 0 the direction is an axis whatever the root, so 1 stands in for 2^64.
        let root =
            if magnitude == 0 { 1 } else { isqrt_u128(((1 << 64) - magnitude * magnitude) << 64) };
        Some(Self { ratio: magnitude << 32, root, negative: raw < 0 })
    }
}
