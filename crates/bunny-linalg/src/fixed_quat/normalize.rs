//! The one normalisation of `bunny-quaternion/1`: each component of `p / |p|`, exactly, rounded
//! once to the nearest raw unit.
//!
//! For a component `r` of an integer vector `p` with `S = |p|^2`, the result is the integer
//! nearest `q = |r| 2^32 / sqrt(S)`, with the sign of `r`. `m = floor(2q)` is the largest `m`
//! with `m^2 S <= 4 r^2 2^64`, found by bisection on exact 256-bit integers; the nearest
//! integer is `(m + 1) / 2`, floored. `2q` is never an odd integer (an exact half), so no tie
//! rule is needed: see `docs/topics/quaternion/README.md#no-exact-halves`.

use super::wide::U256;

/// `2q <= 2^33` because `|r| <= sqrt(S)`.
const TWICE_MAX: u128 = 1 << 33;

/// Each component of `p / |p|` rounded to the nearest raw Q32.32 unit, for `p` with every
/// component below `2^66` in magnitude. The zero vector gives zero, which callers refuse
/// first or prove impossible.
pub(super) const fn normalize(components: [i128; 4]) -> [i64; 4] {
    let [a, b, c, d] = components;
    let [ma, mb, mc, md] = [a.unsigned_abs(), b.unsigned_abs(), c.unsigned_abs(), d.unsigned_abs()];
    // Each square is below 2^132, so the sum is below 2^134.
    let sum = U256::product(ma, ma)
        .add(U256::product(mb, mb))
        .add(U256::product(mc, mc))
        .add(U256::product(md, md));
    if sum.is_zero() {
        return [0; 4];
    }
    [
        signed(a, nearest(ma, sum)),
        signed(b, nearest(mb, sum)),
        signed(c, nearest(mc, sum)),
        signed(d, nearest(md, sum)),
    ]
}

/// The integer nearest `magnitude 2^32 / sqrt(sum)`, at most `2^32`.
#[allow(clippy::cast_possible_truncation)]
const fn nearest(magnitude: u128, sum: U256) -> i64 {
    // 4 r^2 2^64 = r^2 2^66 is below 2^198.
    let target = U256::product(magnitude, magnitude).shl(66);
    // Bisection keeps lo^2 S <= target < hi^2 S; m^2 S is below 2^66 2^134 = 2^200.
    let (mut lo, mut hi) = (0_u128, TWICE_MAX + 1);
    while hi - lo > 1 {
        let mid = lo.midpoint(hi);
        if sum.times(mid * mid).le(target) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    // lo = floor(2q) <= 2^33, so the rounded value is at most 2^32 and fits.
    ((lo + 1) >> 1) as i64
}

/// `rounded` with the sign of `component`.
const fn signed(component: i128, rounded: i64) -> i64 {
    if component < 0 {
        -rounded
    } else {
        rounded
    }
}
