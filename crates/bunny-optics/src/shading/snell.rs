//! The exact integer geometry of a ray meeting a boundary between two refractive indices:
//! Snell's law, total internal reflection and the unpolarized Fresnel reflectance.
//!
//! See `docs/topics/optics/README.md#smooth-dielectric` for the derivation and the bounds.

use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::FixedQ32_32;

use super::wide::{dot, narrow, pick, round_shift, wide, Wide, ONE};
use super::wide256::U256;

/// The bit a direction's largest component magnitude is scaled up to.
const DIRECTION_BIT: u32 = 31;
/// The bit a refracted vector's largest component magnitude is scaled down to before it is
/// normalized, as `wide::unit` does.
const PRESCALE_BIT: u32 = 61;
/// The fraction bits of an amplitude ratio.
const RATIO_BITS: u32 = 62;

/// A direction `d` arriving at a surface with unit normal `n`, from the medium of index `a`
/// into the medium of index `b`, with `m = -(d . n) > 0`.
///
/// `d` is the incoming direction scaled by the power of two that puts its largest component
/// magnitude in `[2^31, 2^32]`, which is exact. Bounds: `|d_i| <= 2^32`, `|n_i| <= 2^32 + 1`,
/// so `d . d` and `n . n` are below `2^66`, `m` is below `2^67`, and `a`, `b` are in
/// `2^32..=2^35`.
#[derive(Clone, Copy)]
pub struct Incidence {
    a: u128,
    b: u128,
    d: Wide,
    n: Wide,
    dd: u128,
    nn: u128,
    m: u128,
}

impl Incidence {
    /// The incidence of `direction` on `normal`, or `None` when `direction . normal >= 0`:
    /// the direction does not arrive against the normal.
    pub fn new(direction: Wide, normal: Wide, indices: [i64; 2]) -> Option<Self> {
        let top = direction.map(i128::unsigned_abs).into_iter().max()?;
        let shift = DIRECTION_BIT.saturating_sub(top.checked_ilog2()?);
        let d = direction.map(|c| c << shift);
        let m = u128::try_from(-dot(d, normal)).ok().filter(|&m| m > 0)?;
        let [a, b] = indices.map(|index| u128::from(index.unsigned_abs()));
        let (dd, nn) = (dot(d, d).unsigned_abs(), dot(normal, normal).unsigned_abs());
        Some(Self { a, b, d, n: normal, dd, nn, m })
    }

    /// `floor(sqrt(K))` with `K = b^2 (d.d)(n.n) - a^2 ((d.d)(n.n) - (d.n)^2)`, or `None`
    /// when `K <= 0`: total internal reflection, decided exactly.
    ///
    /// `K / (b^2 (d.d)(n.n))` is the squared cosine of the refracted angle. Every product is
    /// below `2^203`.
    pub fn root(&self) -> Option<u128> {
        let lengths = U256::product(self.dd, self.nn);
        // Cauchy-Schwarz: (d.n)^2 <= (d.d)(n.n).
        let sine = lengths.checked_sub(U256::product(self.m, self.m))?;
        let reflected = sine.checked_times(self.a * self.a)?;
        let k = lengths.checked_times(self.b * self.b)?.checked_sub(reflected)?;
        (k != U256::ZERO).then(|| k.isqrt())
    }

    /// The unpolarized Fresnel reflectance `(r_s^2 + r_p^2) / 2` for the root `q` of
    /// [`Self::root`], with `r_s = (a m - q) / (a m + q)` and
    /// `r_p = (b^2 m - a q) / (b^2 m + a q)`: raw Q32.32 in `0..=1.0`, rounded once to
    /// nearest, ties to even, from ratios held to 62 fraction bits.
    pub fn reflectance(&self, q: u128) -> FixedQ32_32 {
        // a m and q are below 2^103; b^2 m and a q are below 2^138.
        let (am, q_wide) = (U256::product(self.a, self.m), U256::from_u128(q));
        let s = ratio(am.abs_diff(q_wide), am.checked_add(q_wide));
        let (bm, aq) = (U256::product(self.b * self.b, self.m), U256::product(self.a, q));
        let p = ratio(bm.abs_diff(aq), bm.checked_add(aq));
        let raw = round_shift(s * s + p * p, 2 * RATIO_BITS + 1 - 32).clamp(0, ONE);
        FixedQ32_32::from_raw(i64::try_from(raw).unwrap_or(0))
    }

    /// The unit vector along `T = a (d (n.n) - (d.n) n) - q n` for the root `q` of
    /// [`Self::root`], clamped to `-1.0..=1.0` per component: the refracted direction.
    ///
    /// `T` is exact in 256 bits. Its length is `b (n.n) |d|`, at least `2^126`, so it is always
    /// scaled down: each component magnitude is divided by the power of two that puts the
    /// largest in `[2^61, 2^62)`, truncated toward zero, and the result is normalized by
    /// `FixedUnitVec3::new`. Zero if that refuses, which no input is known to reach.
    pub fn refracted(&self, q: u128) -> FixedVec3 {
        let (nn, dn) = (self.nn.cast_signed(), -self.m.cast_signed());
        let parts = [0, 1, 2].map(|i| {
            let tangent = pick(self.d, i) * nn - dn * pick(self.n, i);
            scaled_difference(self.a, tangent, q, pick(self.n, i))
        });
        let bits = parts.iter().map(|(_, magnitude)| magnitude.bit_length()).max().unwrap_or(0);
        let shift = bits.saturating_sub(PRESCALE_BIT + 1);
        let scaled = parts.map(|(negative, magnitude)| {
            let value = magnitude.shifted_right(shift).cast_signed();
            if negative {
                -value
            } else {
                value
            }
        });
        let unit = FixedUnitVec3::new(narrow(scaled)).map_or([0; 3], |u| wide(u.into_inner()));
        narrow(unit.map(|c| c.clamp(-ONE, ONE)))
    }
}

/// `floor(2^62 n' / d')` for a ratio `numerator / denominator` in `0..=1`, where `n'` and `d'`
/// are both terms divided by the power of two that leaves the denominator below `2^64`,
/// truncated: within `2^-61` of the exact ratio. Zero for a missing or zero denominator, which
/// no caller passes.
fn ratio(numerator: U256, denominator: Option<U256>) -> i128 {
    let Some(denominator) = denominator else { return 0 };
    let shift = denominator.bit_length().saturating_sub(u64::BITS);
    let (n, d) = (numerator.shifted_right(shift), denominator.shifted_right(shift));
    // n <= d < 2^64, so the shifted numerator is below 2^126 and the quotient at most 2^62.
    (n << RATIO_BITS).checked_div(d).map_or(0, u128::cast_signed)
}

/// The sign (`true` for negative) and magnitude of `scale * tangent - root * normal`, exact,
/// for non-negative `scale` and `root`. Every caller's terms are below `2^138`.
fn scaled_difference(scale: u128, tangent: i128, root: u128, normal: i128) -> (bool, U256) {
    let along = U256::product(scale, tangent.unsigned_abs());
    let across = U256::product(root, normal.unsigned_abs());
    if (tangent < 0) != (normal < 0) {
        // Both terms are on the side of zero that `tangent < 0` names.
        return (tangent < 0, along.checked_add(across).unwrap_or(U256::ZERO));
    }
    (if along >= across { tangent < 0 } else { tangent >= 0 }, along.abs_diff(across))
}
