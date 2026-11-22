//! The exact entry and exit of a line through a capped cylinder, for `bunny-ray-cylinder/1`.

use super::super::sphere::roots;
use super::super::wide::U256;
use super::super::{vec3, TraceRay};
use super::TraceCylinder;
use bunny_linalg::FixedUnitVec3;
use bunny_num::FixedQ32_32;
use std::cmp::Ordering;

/// The surface a crossing is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Surface {
    Bottom,
    Top,
    Side,
}

/// The exact distance of a crossing, as the comparisons need it.
#[derive(Clone, Copy, Debug)]
enum Time {
    /// A cap: `n / |d_y|` in direction multiples, `n = (height - o_y) * sign(d_y)`.
    Cap(i128),
    /// A root of the side's quadratic: the far one when true.
    Side(bool),
}

/// Where the line crosses one surface: the exact time and its floor in raw units.
#[derive(Clone, Copy, Debug)]
pub(super) struct Crossing {
    pub(super) surface: Surface,
    time: Time,
    pub(super) distance: i128,
}

/// The part of the line a surface pair leaves inside: all of it, or between two crossings.
#[derive(Clone, Copy, Debug)]
enum Bound {
    Free,
    Between(Crossing, Crossing),
}

/// The line against one cylinder, with the integers its exact comparisons need.
///
/// Bounds in the trace domain: `|o| < 2^40` and `|d| <= 2^32 + 1` per component, so
/// `a = d_x^2 + d_z^2 < 2^66`, `|b| = |o_x d_x + o_z d_z| < 2^74`, `q = |d_y| < 2^33`, the
/// two-dimensional cross product `|o_x d_z - o_z d_x| < 2^74`, and `q^2 D` (with
/// `D = a r^2 - cross^2` the side's quarter discriminant) below `2^212`.
pub(super) struct Line {
    origin: [i64; 3],
    direction: [i64; 3],
    cylinder: TraceCylinder,
    a: i128,
    b: i128,
    q: i128,
    /// `o_x d_z - o_z d_x`.
    cross: i128,
    /// `q^2 D`; the line misses the infinite cylinder when `D` is negative.
    scaled_discriminant: U256,
    /// `D == 0`: the two side roots coincide.
    tangent: bool,
}

impl Line {
    /// `None` when the line misses the infinite cylinder (`D < 0`).
    pub(super) fn new(ray: &TraceRay, cylinder: &TraceCylinder) -> Option<Self> {
        let ([ox, _, oz], [dx, dy, dz]) = (ray.origin.map(i128::from), ray.direction);
        let [dx, dz] = [dx, dz].map(i128::from);
        let (a, b, q) = (dx * dx + dz * dz, ox * dx + oz * dz, i128::from(dy.unsigned_abs()));
        let (cross, r) = (ox * dz - oz * dx, i128::from(cylinder.radius));
        let discriminant = product(a, r * r).checked_sub(product(cross, cross))?;
        let tangent = discriminant == product(0, 0);
        let scaled_discriminant =
            product(a * q, q * r * r).checked_sub(product(q * cross, q * cross))?;
        let (origin, direction, cylinder) = (ray.origin, ray.direction, *cylinder);
        Some(Self { origin, direction, cylinder, a, b, q, cross, scaled_discriminant, tangent })
    }

    /// The entry and exit of the line through the solid cylinder, or `None` when it misses.
    pub(super) fn span(&self) -> Option<(Crossing, Crossing)> {
        let (entry, exit) = match (self.caps()?, self.side()?) {
            (Bound::Between(cap_in, cap_out), Bound::Between(side_in, side_out)) => {
                (self.later(cap_in, side_in), self.earlier(cap_out, side_out))
            }
            (Bound::Between(entry, exit), Bound::Free)
            | (Bound::Free, Bound::Between(entry, exit)) => (entry, exit),
            // A zero direction is refused by `TraceRay`, so one pair always bounds the line.
            (Bound::Free, Bound::Free) => return None,
        };
        match self.compare(entry.time, exit.time) {
            Ordering::Less => Some((entry, exit)),
            // The line only touches the cylinder, at one point; at a rim the cap wins there
            // too, whichever of the two crossings it is.
            Ordering::Equal if entry.surface == Surface::Side => Some((exit, exit)),
            Ordering::Equal => Some((entry, entry)),
            Ordering::Greater => None,
        }
    }

    /// The cap crossings in order of travel; `None` when the line is parallel to the caps
    /// and outside the slab, `Free` when it is parallel and inside (or in a cap's plane).
    fn caps(&self) -> Option<Bound> {
        let ([_, oy, _], [_, dy, _]) = (self.origin, self.direction);
        let TraceCylinder { bottom, top, .. } = self.cylinder;
        if dy == 0 {
            return (bottom..=top).contains(&oy).then_some(Bound::Free);
        }
        let crossing = |surface, height: i64| {
            // `|height - o_y| < 2^41`, so `n * 2^32 < 2^73`.
            let n = i128::from(height - oy) * i128::from(dy.signum());
            let distance = (n << FixedQ32_32::FRACTIONAL_BITS).div_euclid(self.q);
            Crossing { surface, time: Time::Cap(n), distance }
        };
        let (low, high) = (crossing(Surface::Bottom, bottom), crossing(Surface::Top, top));
        Some(if dy > 0 { Bound::Between(low, high) } else { Bound::Between(high, low) })
    }

    /// The side roots in order of travel; `None` when the line is parallel to the axis and
    /// outside the cylinder, `Free` when it is parallel and inside or on the side.
    fn side(&self) -> Option<Bound> {
        let ([ox, _, oz], [dx, _, dz]) = (self.origin, self.direction);
        if self.a == 0 {
            let [ox, oz, r] = [ox, oz, self.cylinder.radius].map(i128::from);
            return (ox * ox + oz * oz <= r * r).then_some(Bound::Free);
        }
        let flat = |x: i64, z: i64| [x, 0, z].map(i128::from);
        let (near, far) = roots(flat(ox, oz), flat(dx, dz), self.cylinder.radius)?;
        let side =
            |far, distance| Crossing { surface: Surface::Side, time: Time::Side(far), distance };
        Some(Bound::Between(side(false, near), side(true, far)))
    }

    /// The later of a cap crossing and a side root; the cap on a tie.
    fn later(&self, cap: Crossing, side: Crossing) -> Crossing {
        if self.compare(side.time, cap.time) == Ordering::Greater {
            side
        } else {
            cap
        }
    }

    /// The earlier of a cap crossing and a side root; the cap on a tie.
    fn earlier(&self, cap: Crossing, side: Crossing) -> Crossing {
        if self.compare(side.time, cap.time) == Ordering::Less {
            side
        } else {
            cap
        }
    }

    /// Exact order of two crossing times.
    fn compare(&self, lhs: Time, rhs: Time) -> Ordering {
        match (lhs, rhs) {
            (Time::Cap(lhs), Time::Cap(rhs)) => lhs.cmp(&rhs),
            (Time::Side(_), Time::Side(_)) if self.tangent => Ordering::Equal,
            (Time::Side(lhs), Time::Side(rhs)) => lhs.cmp(&rhs),
            (Time::Cap(n), Time::Side(far)) => self.cap_against_root(n, far),
            (Time::Side(far), Time::Cap(n)) => self.cap_against_root(n, far).reverse(),
        }
    }

    /// The outward unit normal of a crossing; see `CylinderHit::normal`.
    pub(super) fn normal(&self, crossing: &Crossing) -> Option<FixedUnitVec3> {
        match (crossing.surface, crossing.time) {
            (Surface::Bottom, _) => Some(FixedUnitVec3::NEG_UNIT_Y),
            (Surface::Top, _) => Some(FixedUnitVec3::UNIT_Y),
            (Surface::Side, Time::Side(far)) => self.side_normal(far),
            // Side crossings are built with side times only.
            (Surface::Side, Time::Cap(_)) => None,
        }
    }

    /// The side's normal at the exact root, by the unit-vector rule.
    ///
    /// At the root `t = (-b -+ sqrt(D)) / a` the hit's `(x, z)` times `a` is
    /// `cross (d_z, -d_x) -+ sqrt(D) (d_x, d_z)`, of length `a r`. Scaled by `2^s`, with `s`
    /// the largest shift up to 64 that keeps `a r 2^s < 2^125`, each term is below `2^125`;
    /// `sqrt(D 2^2s)` is floored, an error of at most `|d|` in a vector of length `a r 2^s`.
    /// The vector is shifted down to below `2^62` and normalized by `FixedUnitVec3::new`.
    fn side_normal(&self, far: bool) -> Option<FixedUnitVec3> {
        let ([dx, _, dz], r) = (self.direction.map(i128::from), i128::from(self.cylinder.radius));
        let size = self.a * r;
        let shift = (125 - (128 - size.leading_zeros())).min(64);
        let (scaled_r, scaled_cross) = (r << shift, self.cross << shift);
        let scaled =
            product(size << shift, scaled_r).checked_sub(product(scaled_cross, scaled_cross))?;
        let root = i128::try_from(scaled.isqrt()).ok()?;
        let root = if far { root } else { -root };
        let (x, z) = (scaled_cross * dz + root * dx, root * dz - scaled_cross * dx);
        let widest = x.unsigned_abs().max(z.unsigned_abs());
        let excess = (128 - widest.leading_zeros()).saturating_sub(62);
        let narrow = |v: i128| i64::try_from(v >> excess).ok();
        FixedUnitVec3::new(vec3([narrow(x)?, 0, narrow(z)?]))
    }

    /// The order of the cap time `n / q` against the side root `(-b -+ sqrt(D)) / a`.
    ///
    /// Multiplied by `a q > 0` this is `x = a n + q b` against `-+ sqrt(q^2 D)`, with
    /// `|a n| < 2^107` and `|q b| < 2^107`, so `x` fits `i128` and `x^2 < 2^216`.
    fn cap_against_root(&self, n: i128, far: bool) -> Ordering {
        let x = self.a * n + self.q * self.b;
        let (x_squared, root_squared) = (product(x, x), self.scaled_discriminant);
        match (far, x < 0) {
            (true, true) => Ordering::Less,
            (true, false) => x_squared.cmp(&root_squared),
            // `x >= 0` against `-sqrt(q^2 D) <= 0`: equal only when both are zero.
            (false, false) if x_squared.max(root_squared) == product(0, 0) => Ordering::Equal,
            (false, false) => Ordering::Greater,
            (false, true) => root_squared.cmp(&x_squared),
        }
    }
}

/// The exact product of two `i128` magnitudes.
fn product(lhs: i128, rhs: i128) -> U256 {
    U256::product(lhs.unsigned_abs(), rhs.unsigned_abs())
}
