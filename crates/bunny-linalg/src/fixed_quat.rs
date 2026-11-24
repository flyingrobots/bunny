//! Unit quaternions in Q32.32 (`bunny-quaternion/1`): storage order, the length band every value
//! satisfies, the one normalisation, and the exact sign operations.
//!
//! Products, vector rotation and the conversions are in the submodules. Every rule and bound is
//! in `docs/topics/quaternion/README.md`.

mod axis_angle;
mod normalize;
mod product;
mod rotate;
mod wide;

use core::fmt;
use core::ops::Neg;

use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;

/// Contract identity of [`FixedUnitQuat`] and its operations.
pub const QUATERNION_CONTRACT_ID: &str = "bunny-quaternion/1";

/// The smallest raw `x^2 + y^2 + z^2 + w^2` of a [`FixedUnitQuat`]: `(2^32 - 1)^2`.
pub const QUAT_LENGTH_SQUARED_MIN_RAW: u128 = (ONE_RAW as u128 - 1) * (ONE_RAW as u128 - 1);

/// The largest raw `x^2 + y^2 + z^2 + w^2` of a [`FixedUnitQuat`]: `(2^32 + 1)^2`.
pub const QUAT_LENGTH_SQUARED_MAX_RAW: u128 = (ONE_RAW as u128 + 1) * (ONE_RAW as u128 + 1);

/// Why a [`FixedUnitQuat`] constructor refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuatError {
    /// All four components are zero: there is no direction to normalise.
    Zero,
    /// [`FixedUnitQuat::try_from_unit`] only: the raw `x^2 + y^2 + z^2 + w^2` is outside
    /// `[QUAT_LENGTH_SQUARED_MIN_RAW, QUAT_LENGTH_SQUARED_MAX_RAW]`.
    NotUnit,
}

impl fmt::Display for QuatError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zero => formatter.write_str("the zero quaternion has no direction"),
            Self::NotUnit => {
                formatter.write_str("the quaternion's length is not within one raw unit of one")
            }
        }
    }
}

impl core::error::Error for QuatError {}

/// A quaternion `w + x i + y j + z k` of length within one raw unit of one, stored as four
/// [`FixedQ32_32`] components in the order `(x, y, z, w)`: the vector part first, in the
/// order of [`crate::FixedVec3`], then the scalar part `w`.
///
/// Every value satisfies `(2^32 - 1)^2 <= x^2 + y^2 + z^2 + w^2 <= (2^32 + 1)^2` on raw values,
/// so its exact length is within `2^-32` of one; an exactly unit quaternion is rare in fixed
/// point and is not required. The fields are private and every constructor and operation
/// keeps the band. `q` and `-q` are the same rotation but different values: `==` compares the
/// stored components, and nothing canonicalises the sign unless [`Self::canonical`] is called.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FixedUnitQuat {
    x: FixedQ32_32,
    y: FixedQ32_32,
    z: FixedQ32_32,
    w: FixedQ32_32,
}

impl FixedUnitQuat {
    /// The identity rotation, `(0, 0, 0, 1)`: exactly unit.
    pub const IDENTITY: Self = Self::from_raw([0, 0, 0, ONE_RAW]);

    /// Accepts components that already satisfy the length band, unchanged.
    ///
    /// The raw `x^2 + y^2 + z^2 + w^2` is computed exactly (a sum that would pass `u128` is out
    /// of the band).
    ///
    /// # Errors
    /// [`QuatError::NotUnit`] when the sum is outside `[(2^32 - 1)^2, (2^32 + 1)^2]`.
    pub const fn try_from_unit(
        x: FixedQ32_32,
        y: FixedQ32_32,
        z: FixedQ32_32,
        w: FixedQ32_32,
    ) -> Result<Self, QuatError> {
        let raw = [x.raw(), y.raw(), z.raw(), w.raw()];
        match length_squared_raw(raw) {
            Some(sum)
                if sum >= QUAT_LENGTH_SQUARED_MIN_RAW && sum <= QUAT_LENGTH_SQUARED_MAX_RAW =>
            {
                Ok(Self::from_raw(raw))
            }
            _ => Err(QuatError::NotUnit),
        }
    }

    /// Normalises any nonzero components: each result component is the exact
    /// `component / |(x, y, z, w)|` rounded once to the nearest raw unit (an exact half cannot
    /// occur), so its length is within one raw unit of one.
    ///
    /// Every nonzero input is accepted, from one raw unit to the extremes of the raw range:
    /// the rounding is of the exact quotient, so no input is too small or too large.
    ///
    /// # Errors
    /// [`QuatError::Zero`] when all four components are zero.
    pub const fn try_normalize(
        x: FixedQ32_32,
        y: FixedQ32_32,
        z: FixedQ32_32,
        w: FixedQ32_32,
    ) -> Result<Self, QuatError> {
        if x.raw() == 0 && y.raw() == 0 && z.raw() == 0 && w.raw() == 0 {
            return Err(QuatError::Zero);
        }
        let wide = [x.raw() as i128, y.raw() as i128, z.raw() as i128, w.raw() as i128];
        Ok(Self::from_raw(normalize::normalize(wide)))
    }

    /// The `x` component (the `i` coefficient).
    #[must_use]
    pub const fn x(self) -> FixedQ32_32 {
        self.x
    }

    /// The `y` component (the `j` coefficient).
    #[must_use]
    pub const fn y(self) -> FixedQ32_32 {
        self.y
    }

    /// The `z` component (the `k` coefficient).
    #[must_use]
    pub const fn z(self) -> FixedQ32_32 {
        self.z
    }

    /// The scalar part `w`.
    #[must_use]
    pub const fn w(self) -> FixedQ32_32 {
        self.w
    }

    /// The components in storage order, `[x, y, z, w]`.
    #[must_use]
    pub const fn to_array(self) -> [FixedQ32_32; 4] {
        [self.x, self.y, self.z, self.w]
    }

    /// The conjugate `(-x, -y, -z, w)`, exact: the inverse rotation.
    #[must_use]
    pub const fn conjugate(self) -> Self {
        Self::from_raw([-self.x.raw(), -self.y.raw(), -self.z.raw(), self.w.raw()])
    }

    /// The same rotation with a fixed sign: `self` when the first nonzero component in the
    /// order `w, x, y, z` is positive, else `-self`. So `w > 0` after it, and at `w = 0` the
    /// tie goes to `x`, then `y`, then `z`. `q.canonical() == (-q).canonical()` for every `q`.
    #[must_use]
    pub const fn canonical(self) -> Self {
        let lead = if self.w.raw() != 0 {
            self.w.raw()
        } else if self.x.raw() != 0 {
            self.x.raw()
        } else if self.y.raw() != 0 {
            self.y.raw()
        } else {
            self.z.raw()
        };
        if lead < 0 {
            self.negated()
        } else {
            self
        }
    }

    /// `(-x, -y, -z, -w)`, exact.
    const fn negated(self) -> Self {
        Self::from_raw([-self.x.raw(), -self.y.raw(), -self.z.raw(), -self.w.raw()])
    }

    /// Wraps raw components in storage order. Callers keep the band.
    const fn from_raw(raw: [i64; 4]) -> Self {
        let [x, y, z, w] = raw;
        Self {
            x: FixedQ32_32::from_raw(x),
            y: FixedQ32_32::from_raw(y),
            z: FixedQ32_32::from_raw(z),
            w: FixedQ32_32::from_raw(w),
        }
    }

    /// Raw components in storage order.
    const fn raw(self) -> [i64; 4] {
        [self.x.raw(), self.y.raw(), self.z.raw(), self.w.raw()]
    }
}

/// `-q`: every component negated, exact (each is at most `2^32 + 1` raw in magnitude). The
/// same rotation, a different value.
impl Neg for FixedUnitQuat {
    type Output = Self;

    fn neg(self) -> Self {
        self.negated()
    }
}

/// The exact raw `x^2 + y^2 + z^2 + w^2`, or `None` when it passes `u128`.
const fn length_squared_raw(raw: [i64; 4]) -> Option<u128> {
    let [x, y, z, w] = raw;
    let squares = [square(x), square(y), square(z), square(w)];
    let mut sum = 0_u128;
    let mut rest: &[u128] = &squares;
    while let [first, tail @ ..] = rest {
        match sum.checked_add(*first) {
            Some(next) => sum = next,
            None => return None,
        }
        rest = tail;
    }
    Some(sum)
}

/// The exact square of a raw component, at most `2^126`.
const fn square(raw: i64) -> u128 {
    let magnitude = raw.unsigned_abs() as u128;
    magnitude * magnitude
}
