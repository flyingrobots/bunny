//! Reference models for `bunny-ggx-conductor/1`, written from the published formulas and not
//! from the kernel: Heitz's listing for the visible normal at 60 fractional bits, and exact
//! integer bounds for the Smith ratio. No floating point.

/// One in the model's fixed point: 60 fractional bits.
pub(crate) const UNIT: i128 = 1 << 60;
/// Raw Q32.32 values gain this many bits when they enter the model.
pub(crate) const LIFT: u32 = 28;

type V3 = [i128; 3];

/// `a * b`, floored to 60 fractional bits (operands up to `2^62`).
fn mul(a: i128, b: i128) -> i128 {
    (a * b) >> 60
}

/// `a / b`, truncated to 60 fractional bits (`a` up to `2^62`, `b` nonzero).
fn div(a: i128, b: i128) -> i128 {
    (a << 60) / b
}

/// `sqrt(max(0, a))`, floored to 60 fractional bits.
fn sqrt(a: i128) -> i128 {
    let radicand = u128::try_from(a.max(0)).expect("non-negative") << 60;
    i128::try_from(radicand.isqrt()).expect("a root below 2^62")
}

fn dot(a: V3, b: V3) -> i128 {
    mul(a[0], b[0]) + mul(a[1], b[1]) + mul(a[2], b[2])
}

fn cross(a: V3, b: V3) -> V3 {
    [
        mul(a[1], b[2]) - mul(a[2], b[1]),
        mul(a[2], b[0]) - mul(a[0], b[2]),
        mul(a[0], b[1]) - mul(a[1], b[0]),
    ]
}

fn normalize(v: V3) -> V3 {
    let length = sqrt(dot(v, v));
    v.map(|c| div(c, length))
}

/// What the model needs of one sample, all raw Q32.32.
pub(crate) struct ModelInput {
    /// The frame's normal and tangent.
    pub(crate) normal: V3,
    pub(crate) tangent: V3,
    /// The incoming direction, pointing at the surface.
    pub(crate) incoming: V3,
    pub(crate) alpha: i128,
    /// The first sample coordinate.
    pub(crate) u: i128,
    /// `(sin, cos)` of the second coordinate in turns, from `FixedAngle::sin_cos`.
    pub(crate) sin_cos: (i128, i128),
    /// Whether the incoming direction is exactly parallel to the normal (`d x n = 0`).
    pub(crate) parallel: bool,
}

/// The model's sample: the world direction, its cosine against the normal and the cosine of
/// the view against the microfacet normal, each with 60 fractional bits.
pub(crate) struct ModelSample {
    pub(crate) direction: V3,
    pub(crate) cosine: i128,
    pub(crate) facet_cosine: i128,
}

/// Heitz, "Sampling the GGX Distribution of Visible Normals" (JCGT 7(4), 2018), listing 1 with
/// isotropic roughness, in an orthonormal frame `(T, B, N)` rebuilt at 60 bits from the frame's
/// normal and tangent, followed by the reflection `2 (V . Ne) Ne - V`.
pub(crate) fn heitz(input: &ModelInput) -> ModelSample {
    let lift = |v: V3| v.map(|c| c << LIFT);
    let normal = normalize(lift(input.normal));
    let tangent = lift(input.tangent);
    let along = dot(tangent, normal);
    let tangent = normalize([0, 1, 2].map(|i| tangent[i] - mul(along, normal[i])));
    let bitangent = cross(normal, tangent);
    // The direction may be short: scale it by a power of two to a largest component in
    // [2^59, 2^60) before normalizing, which is exact.
    let top = input.incoming.iter().map(|c| c.unsigned_abs()).max().expect("three");
    let view = normalize(input.incoming.map(|c| -(c << (59 - top.ilog2()))));
    let ve = [dot(view, tangent), dot(view, bitangent), dot(view, normal)];
    let alpha = input.alpha << LIFT;
    // Section 3.2: transforming the view direction to the hemisphere configuration.
    let stretched = normalize([mul(alpha, ve[0]), mul(alpha, ve[1]), ve[2]]);
    let length = sqrt(mul(stretched[0], stretched[0]) + mul(stretched[1], stretched[1]));
    let (vh, t1_axis) = if input.parallel || length == 0 {
        ([0, 0, UNIT], [UNIT, 0, 0])
    } else {
        (stretched, [div(-stretched[1], length), div(stretched[0], length), 0])
    };
    // Section 4.1: orthonormal basis.
    let t2_axis = cross(vh, t1_axis);
    // Section 4.2: parameterization of the projected area.
    let radius = sqrt(input.u << LIFT);
    let (sin, cos) = (input.sin_cos.0 << LIFT, input.sin_cos.1 << LIFT);
    let t1 = mul(radius, cos);
    let s = i128::midpoint(UNIT, vh[2]);
    let t2 = mul(UNIT - s, sqrt(UNIT - mul(t1, t1))) + mul(s, mul(radius, sin));
    // Section 4.3: reprojection onto the hemisphere.
    let t3 = sqrt(UNIT - mul(t1, t1) - mul(t2, t2));
    let nh = [0, 1, 2].map(|i| mul(t1, t1_axis[i]) + mul(t2, t2_axis[i]) + mul(t3, vh[i]));
    // Section 3.4: transforming the normal back to the ellipsoid configuration.
    let ne = normalize([mul(alpha, nh[0]), mul(alpha, nh[1]), nh[2].max(0)]);
    let facet_cosine = dot(ve, ne);
    let local = [0, 1, 2].map(|i| 2 * mul(facet_cosine, ne[i]) - ve[i]);
    let direction = [0, 1, 2].map(|i| {
        mul(local[0], tangent[i]) + mul(local[1], bitangent[i]) + mul(local[2], normal[i])
    });
    ModelSample { direction, cosine: local[2], facet_cosine }
}

/// Integers `(low, high)` with `low <= 2^64 sqrt(alpha^2 + (1 - alpha^2) c^2) <= high` for raw
/// `alpha` and `c` in `0..=2^32`, checked here: the radicand `2^128 - (2^64 - alpha^2)(2^64 -
/// c^2)` is exact in units of `2^-128`.
fn root_bounds(alpha: i128, cosine: i128) -> (u128, u128) {
    let square = |x: i128| u128::try_from(x * x).expect("a square");
    let product = ((1_u128 << 64) - square(alpha)) * ((1_u128 << 64) - square(cosine));
    if product == 0 {
        return (1 << 64, 1 << 64);
    }
    let radicand = u128::MAX - product + 1;
    let low = radicand.isqrt();
    // low^2 <= radicand < (low + 1)^2, without overflow: low < 2^64.
    assert!(low * low <= radicand && radicand - low * low <= 2 * low);
    (low, low + 1)
}

/// Exact bounds `(n_low, d_low, n_high, d_high)` of the Smith ratio
/// `G2 / G1 = c_l (c_v + s(c_v)) / (c_l s(c_v) + c_v s(c_l))` at raw `alpha`, `c_v`, `c_l`:
/// `n_low / d_low <= G2 / G1 <= n_high / d_high`. The ratio rises with `s(c_v)` and falls with
/// `s(c_l)`, so the low end takes the low root of the view and the high root of the light.
pub(crate) fn ratio_bounds(alpha: i128, view: i128, light: i128) -> [u128; 4] {
    let (view_roots, light_roots) = (root_bounds(alpha, view), root_bounds(alpha, light));
    let (view, light) = (u128::try_from(view).expect("c_v"), u128::try_from(light).expect("c_l"));
    let numerator = |root: u128| light * ((view << 32) + root);
    let denominator = |view_root: u128, light_root: u128| light * view_root + view * light_root;
    [
        numerator(view_roots.0),
        denominator(view_roots.0, light_roots.1),
        numerator(view_roots.1),
        denominator(view_roots.1, light_roots.0),
    ]
}
