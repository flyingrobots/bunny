//! The first root of a ray against a circle, rounded once.

use super::{rounded, RayHit2};
use crate::wide::Wide;

/// `round_half_even(2^32 (b - sqrt(disc)) / a)`, for `b > 0` and `0 <= disc <= b^2`.
pub(super) fn first_root(length: Wide, ahead: Wide, disc: Wide) -> RayHit2 {
    let shift = Wide::power_of_two(32);
    let (scaled, big) = (ahead.times(shift), disc.times(shift).times(shift));
    let root = Wide::from_unsigned(big.isqrt());
    let square = root.times(root);
    let gap = scaled.minus(root);
    if square == big {
        return rounded(gap, &length.magnitude());
    }
    // `sqrt(V) - s` is above one half exactly when `V > s^2 + s`.
    let above_half = big.minus(square).minus(root);
    let upper = if above_half.is_negative() || above_half.is_zero() {
        gap.plus(gap)
    } else {
        gap.plus(gap).minus(Wide::power_of_two(0))
    };
    let midpoint = upper.plus(upper).minus(Wide::power_of_two(0));
    rounded(midpoint, &length.times(Wide::power_of_two(2)).magnitude())
}

#[cfg(test)]
mod tests {
    use core::cmp::Ordering;

    use super::{first_root, RayHit2, Wide};
    use wasm_bindgen_test::wasm_bindgen_test;

    /// How `2^32 (b - sqrt(disc)) / a` compares with `m / 2`, by squaring:
    /// `L = 2^33 b - m a` against `2^33 sqrt(disc)`, so `L^2` against `2^66 disc` when `L >= 0`.
    fn against(a: u128, b: u128, disc: u128, m: u128) -> Ordering {
        (b << 33).checked_sub(m * a).map_or(Ordering::Less, |l| (l * l).cmp(&(disc << 66)))
    }

    /// Whether `t` is that value rounded once to nearest, ties to even.
    fn rounds_to(a: u128, b: u128, disc: u128, t: u128) -> bool {
        let even = t.is_multiple_of(2);
        let low =
            t.checked_sub(1).map_or(Ordering::Greater, |below| against(a, b, disc, 2 * below + 1));
        let high = against(a, b, disc, 2 * t + 1);
        on_side(low, Ordering::Greater, even) && on_side(high, Ordering::Less, even)
    }

    /// Whether `order` is `side`, or a tie that goes to an even `t`.
    fn on_side(order: Ordering, side: Ordering, even: bool) -> bool {
        order == side || (order.is_eq() && even)
    }

    /// Asserts `first_root(a, b, disc)` against the squaring oracle; whether it is a tie.
    fn check(a: u128, b: u128, disc: u128) -> bool {
        let got = first_root(Wide::from_unsigned(a), Wide::from_unsigned(b), {
            Wide::from_unsigned(disc)
        });
        let t = match got {
            RayHit2::At(t) => u128::try_from(t.raw()).unwrap_or(u128::MAX),
            _ => u128::MAX,
        };
        // dojo: allow panic-path -- test assertion against the squaring oracle
        assert!(t != u128::MAX && rounds_to(a, b, disc, t), "{a} {b} {disc} {got:?}");
        against(a, b, disc, 2 * t + 1).is_eq() || (t > 0 && against(a, b, disc, 2 * t - 1).is_eq())
    }

    /// Oracle: squaring, over `a` in `1..=10` and `2^33`, `3 2^33` and `2^35`, every `b` in
    /// `1..=16` and every `disc` in `0..=b^2`: perfect and non-perfect squares, both halves of
    /// `(s, s + 1)`, and exact ties (which need `2^33` to divide `a`).
    #[wasm_bindgen_test(unsupported = test)]
    fn planar_tp_021_first_root_rounds_once_on_a_small_exhaustive_grid() {
        let grid = (1..=10_u128).chain([1 << 33, 3 << 33, 1 << 35]).flat_map(|a| {
            (1..=16_u128).flat_map(move |b| (0..=b * b).map(move |disc| (a, b, disc)))
        });
        let ties = grid.filter(|&(a, b, disc)| check(a, b, disc)).count();
        // dojo: allow panic-path -- test assertion: the grid reaches exact ties
        assert_eq!(ties, 114, "ties");
    }
}
