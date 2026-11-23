//! The exact closest point of a triangle: the projection onto its plane when that lies on
//! the triangle, otherwise the nearest of its three closed edges.
//!
//! Every value is exact; widths are stated at each step for coordinates below `2^40` in
//! magnitude (`docs/topics/mesh-queries/README.md#widths`).

use super::wide::I256;

type Raw3 = [i64; 3];
type Wide3 = [i128; 3];

/// A point on a triangle and its squared distance from the query point, both exact: the
/// point is `numerators / denominator` and the squared distance, in raw units squared,
/// `distance / denominator`, with `denominator > 0` and `distance >= 0`.
#[derive(Clone, Copy, Debug)]
pub(super) struct Candidate {
    pub(super) numerators: [I256; 3],
    pub(super) denominator: I256,
    pub(super) distance: I256,
}

impl Candidate {
    /// Compares the two exact squared distances: `distance / denominator` against the other's,
    /// by cross multiplication in 512 bits.
    pub(super) fn cmp_distance(&self, other: &Self) -> core::cmp::Ordering {
        self.distance.cmp_products(other.denominator, other.distance, self.denominator)
    }
}

/// The exact closest point of the closed triangle `a b c` to `p`.
///
/// When the triangle has a nonzero normal and the projection of `p` onto its plane has three
/// barycentric weights of at least zero, the projection is the closest point. Otherwise the
/// closest point is on the boundary, the union of the three closed edges, and the nearest of
/// their closest points is taken; edges at an equal distance share that point, because the
/// closest point of a convex set is unique. A triangle with a zero normal (collinear or
/// coincident vertices) is the union of its edges, so the same rule covers it.
pub(super) fn closest(p: Raw3, [a, b, c]: [Raw3; 3]) -> Candidate {
    if let Some(face) = face(p, a, b, c) {
        return face;
    }
    let nearer = |first: Candidate, second: Candidate| {
        if second.cmp_distance(&first).is_lt() {
            second
        } else {
            first
        }
    };
    nearer(nearer(segment(p, a, b), segment(p, b, c)), segment(p, c, a))
}

/// The projection of `p` onto the plane of `a b c`, when the triangle has a nonzero normal
/// `n` and the projection is on the closed triangle.
///
/// The weights are `n . ((b - p) x (c - p))`, `n . ((c - p) x (a - p))` and
/// `n . ((a - p) x (b - p))` over `|n|^2`: each cross product is unchanged by moving `p`
/// along `n`, so these are the weights of the projection. Edges below `2^41` make `n` and the
/// cross products below `2^83` per component and each weight numerator below `2^168`.
fn face(p: Raw3, a: Raw3, b: Raw3, c: Raw3) -> Option<Candidate> {
    let normal = cross(sub(b, a), sub(c, a));
    if normal == [0; 3] {
        return None;
    }
    let (ap, bp, cp) = (sub(p, a), sub(p, b), sub(p, c));
    let weights = [cross(bp, cp), cross(cp, ap), cross(ap, bp)];
    if weights.iter().any(|&weight| wide_dot(normal, weight).is_negative()) {
        return None;
    }
    // `|n . (p - a)| < 3 * 2^83 * 2^41 < 2^126`.
    let height = I256::from_i128(dot(normal, ap));
    let area = wide_dot(normal, normal);
    // `p |n|^2 - n (n . (p - a))`: below `2^208 + 2^209` per component.
    let numerators = map(p, normal, |coordinate, component| {
        let along = I256::from_i128(component).mul(height);
        I256::from_i128(i128::from(coordinate)).mul(area).sub(along)
    });
    // `(n . (p - a))^2 / |n|^2`, numerator below `2^252`.
    Some(Candidate { numerators, denominator: area, distance: height.mul(height) })
}

/// The closest point of the closed segment `u w` to `p`: `u + (w - u) t / L` with
/// `t = (w - u) . (p - u)` clamped to `0..=L`, `L = |w - u|^2`. A segment of length zero is
/// its one point.
fn segment(p: Raw3, u: Raw3, w: Raw3) -> Candidate {
    let (edge, offset) = (sub(w, u), sub(p, u));
    // Both below `3 * 2^82`.
    let (t, length) = (dot(edge, offset), dot(edge, edge));
    if t <= 0 {
        return vertex(p, u);
    }
    if t >= length {
        return vertex(p, w);
    }
    // `u L + e t`: below `3 * 2^122 + 3 * 2^123 < 2^126` per component.
    let numerators =
        map(u, edge, |start, along| I256::from_i128(i128::from(start) * length + along * t));
    // `|p - u|^2 L - t^2 = |(p - u) x e|^2`, at least zero and below `2^168`.
    let reach = I256::from_i128(dot(offset, offset)).mul(I256::from_i128(length));
    let t = I256::from_i128(t);
    let denominator = I256::from_i128(length);
    Candidate { numerators, denominator, distance: reach.sub(t.mul(t)) }
}

/// The vertex `v` itself, at its integer squared distance (below `3 * 2^82`).
fn vertex(p: Raw3, v: Raw3) -> Candidate {
    let offset = sub(p, v);
    Candidate {
        numerators: v.map(|coordinate| I256::from_i128(i128::from(coordinate))),
        denominator: I256::ONE,
        distance: I256::from_i128(dot(offset, offset)),
    }
}

/// Difference of two points with coordinates below `2^40`: below `2^41` per component.
fn sub(lhs: Raw3, rhs: Raw3) -> Wide3 {
    let [a0, a1, a2] = lhs.map(i128::from);
    let [b0, b1, b2] = rhs.map(i128::from);
    [a0 - b0, a1 - b1, a2 - b2]
}

/// Cross product of two vectors below `2^41` per component: below `2^83`.
const fn cross(lhs: Wide3, rhs: Wide3) -> Wide3 {
    let ([a0, a1, a2], [b0, b1, b2]) = (lhs, rhs);
    [a1 * b2 - a2 * b1, a2 * b0 - a0 * b2, a0 * b1 - a1 * b0]
}

/// Dot product whose three terms and their sum fit `i128` (each caller states why).
const fn dot(lhs: Wide3, rhs: Wide3) -> i128 {
    let ([a0, a1, a2], [b0, b1, b2]) = (lhs, rhs);
    a0 * b0 + a1 * b1 + a2 * b2
}

/// Dot product of two vectors below `2^83` per component, in 256 bits: below `2^168`.
fn wide_dot(lhs: Wide3, rhs: Wide3) -> I256 {
    let term = |a: i128, b: i128| I256::from_i128(a).mul(I256::from_i128(b));
    let ([a0, a1, a2], [b0, b1, b2]) = (lhs, rhs);
    term(a0, b0).add(term(a1, b1)).add(term(a2, b2))
}

fn map<A: Copy, B: Copy>(lhs: [A; 3], rhs: [B; 3], f: impl Fn(A, B) -> I256) -> [I256; 3] {
    let ([a0, a1, a2], [b0, b1, b2]) = (lhs, rhs);
    [f(a0, b0), f(a1, b1), f(a2, b2)]
}
