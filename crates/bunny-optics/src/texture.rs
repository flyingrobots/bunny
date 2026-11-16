//! Texture coordinates at a hit and the texels and weights of a bilinear lookup:
//! `bunny-texture-bilinear/1`.

use bunny_linalg::FixedVec2;
use bunny_num::FixedQ32_32;
use bunny_query::BarycentricNumerators;

/// The versioned name of the rule [`interpolate_uv`] and [`bilinear`] compute. Any change to an
/// output bit is a new version.
pub const TEXTURE_BILINEAR_ID: &str = "bunny-texture-bilinear/1";

/// One raw unit of 1.0 in Q32.32.
const ONE: i128 = 1 << 32;

/// The offset that maps a raw `i64` to `0..2^64`.
const BIAS: i128 = 1 << 63;

/// The texture coordinates at the point with barycentric `weights` on a triangle whose vertex
/// coordinates are `uvs` (in the order `v0`, `v1`, `v2`).
///
/// Each component is `(w0 t0 + w1 t1 + w2 t2) / det` with the exact weights
/// `w = (det - u - v, u, v)`, rounded once to nearest raw unit, ties to even. Because the
/// weights are nonnegative and sum to `det`, the result lies between the smallest and largest
/// vertex component, so it is always representable. Proven infallible: the quotient is formed
/// by exact long division in `u128`, with every remainder below `det` and every partial sum
/// below `2 det <= 2^128`. A corner gives its vertex's coordinates exactly. See
/// `docs/topics/optics/README.md#texture-coordinates`.
///
/// # Examples
///
/// ```
/// use bunny_linalg::FixedVec2;
/// use bunny_num::FixedQ32_32;
/// use bunny_optics::interpolate_uv;
/// use bunny_query::BarycentricNumerators;
///
/// let uv = |u: i64, v: i64| FixedVec2::new(FixedQ32_32::from_raw(u), FixedQ32_32::from_raw(v));
/// let one = 1_i64 << 32;
/// // Weights (1, 1, 2) / 4.
/// let at = BarycentricNumerators::new(1, 2, 4).expect("on the triangle");
/// let t = interpolate_uv([uv(0, 0), uv(one, 0), uv(0, one)], at);
/// assert_eq!((t.x.raw(), t.y.raw()), (one / 4, one / 2));
/// ```
#[must_use]
pub fn interpolate_uv(uvs: [FixedVec2; 3], weights: BarycentricNumerators) -> FixedVec2 {
    let (u, v, det) = (weights.u(), weights.v(), weights.det());
    let exact = [det - u - v, u, v].map(i128::unsigned_abs);
    let det = det.unsigned_abs();
    let x = convex_round(uvs.map(|t| t.x.raw()), exact, det);
    let y = convex_round(uvs.map(|t| t.y.raw()), exact, det);
    FixedVec2::new(x, y)
}

/// `round(sum w_i t_i / det)`, ties to even, for `w_i >= 0` with `sum w_i = det > 0`.
fn convex_round(values: [i64; 3], weights: [u128; 3], det: u128) -> FixedQ32_32 {
    // Shift each value to `0..2^64`; the weights sum to `det`, so the quotient shifts the same.
    let biased = values.map(|t| (i128::from(t) + BIAS).unsigned_abs());
    let mut division = LongDivision { quotient: 0, remainder: 0, det };
    for bit in (0..64).rev() {
        division.double();
        for (value, weight) in biased.iter().zip(weights) {
            division.add(weight * ((value >> bit) & 1));
        }
    }
    // The rounded quotient lies between the smallest and largest biased value.
    let raw = i128::try_from(division.rounded()).unwrap_or(BIAS) - BIAS;
    FixedQ32_32::from_raw(i64::try_from(raw).unwrap_or(0))
}

/// `sum / det` as a quotient and a remainder below `det`, built one term at a time.
struct LongDivision {
    quotient: u128,
    remainder: u128,
    det: u128,
}

impl LongDivision {
    /// Doubles the sum. `remainder < det <= 2^127`, so twice it fits.
    const fn double(&mut self) {
        self.quotient <<= 1;
        self.remainder <<= 1;
        self.reduce();
    }

    /// Adds `term`, at most `det`. `remainder + term < 2 det <= 2^128`, so it fits.
    const fn add(&mut self, term: u128) {
        self.remainder += term;
        self.reduce();
    }

    /// Brings the remainder below `det`; it is below `2 det` here.
    const fn reduce(&mut self) {
        if self.remainder >= self.det {
            self.remainder -= self.det;
            self.quotient += 1;
        }
    }

    /// The quotient rounded to nearest, ties to even.
    const fn rounded(&self) -> u128 {
        // Compare the remainder with `det / 2` as `remainder` against `det - remainder`.
        let (below, above) = (self.remainder, self.det - self.remainder);
        if below > above || (below == above && self.quotient & 1 == 1) {
            self.quotient + 1
        } else {
            self.quotient
        }
    }
}

/// How a texel index outside the texture is brought back into it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextureWrap {
    /// The index modulo the size (Euclidean, so `-1` is the last texel): the texture tiles.
    Repeat,
    /// The index clamped to `0..=size - 1`: the edge texels extend outward.
    Clamp,
}

impl TextureWrap {
    fn apply(self, index: i128, size: u32) -> u32 {
        let size = i128::from(size);
        let wrapped = match self {
            Self::Repeat => index.rem_euclid(size),
            Self::Clamp => index.clamp(0, size - 1),
        };
        u32::try_from(wrapped).unwrap_or(0)
    }
}

/// The four texels of a bilinear lookup and their weights.
///
/// Texels are `[column, row]`, in the order `(x0, y0)`, `(x1, y0)`, `(x0, y1)`, `(x1, y1)`, with
/// `x1` the column after `x0` and `y1` the row after `y0`, each wrapped. The weights are in
/// `0..=1.0` and sum to exactly 1.0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BilinearTaps {
    texels: [[u32; 2]; 4],
    weights: [FixedQ32_32; 4],
}

impl BilinearTaps {
    /// The texels as `[column, row]`, each in range for the texture.
    #[must_use]
    pub const fn texels(&self) -> [[u32; 2]; 4] {
        self.texels
    }

    /// The weight of each texel, in the order of [`Self::texels`]; they sum to exactly 1.0.
    #[must_use]
    pub const fn weights(&self) -> [FixedQ32_32; 4] {
        self.weights
    }
}

/// The texels and weights of a bilinear lookup at texture coordinates `uv = (u, v)` in a texture
/// of `[width, height]` texels (columns, rows), or `None` when either size is zero.
///
/// **Texel centres.** Column `i` covers `u` in `[i / width, (i + 1) / width)` and its centre is
/// at `u = (i + 1/2) / width`; rows are the same along `v`, with row 0 at `v = 0` (no flip).
///
/// 1. **Position.** `x = u width - 1/2` and `y = v height - 1/2`, exact (`i128`).
/// 2. **Split.** `x0 = floor(x)` and `fx = x - x0` in `[0, 1)`, exact raw units; the same for `y`.
/// 3. **Wrap.** Columns `x0` and `x0 + 1` and rows `y0` and `y0 + 1` are each wrapped by
///    `wrap`, independently.
/// 4. **Weights.** `w11 = fx fy` and `w10 = fx (1 - fy)`, each rounded once to nearest raw
///    unit, ties to even; `w01 = fy - w11` and `w00 = (1 - fy) - w10`, exact. Each is within
///    half a raw unit of the exact product, and the four sum to exactly 1.0.
///
/// At a texel centre `fx = fy = 0`: the first texel is that texel, with weight exactly 1.0.
/// Proven infallible: `|u width|` is below `2^95`.
///
/// # Examples
///
/// ```
/// use bunny_linalg::FixedVec2;
/// use bunny_num::FixedQ32_32;
/// use bunny_optics::{bilinear, TextureWrap};
///
/// let one = 1_i64 << 32;
/// // The centre of texel (1, 0) of a 4 by 2 texture: u = 1.5 / 4, v = 0.5 / 2.
/// let (u, v) = (FixedQ32_32::from_raw(3 * one / 8), FixedQ32_32::from_raw(one / 4));
/// let taps = bilinear(FixedVec2::new(u, v), [4, 2], TextureWrap::Repeat).expect("nonempty");
/// assert_eq!(taps.texels()[0], [1, 0]);
/// assert_eq!(taps.weights()[0], FixedQ32_32::ONE);
/// ```
#[must_use]
pub fn bilinear(
    uv: FixedVec2,
    [width, height]: [u32; 2],
    wrap: TextureWrap,
) -> Option<BilinearTaps> {
    let (u, v) = (uv.x, uv.y);
    if width == 0 || height == 0 {
        return None;
    }
    let (x0, fx) = split(u, width);
    let (y0, fy) = split(v, height);
    let (c0, c1) = (wrap.apply(x0, width), wrap.apply(x0 + 1, width));
    let (r0, r1) = (wrap.apply(y0, height), wrap.apply(y0 + 1, height));
    let w11 = round_unit(fx * fy);
    let w10 = round_unit(fx * (ONE - fy));
    let weights = [ONE - fy - w10, w10, fy - w11, w11]
        .map(|w| FixedQ32_32::from_raw(i64::try_from(w).unwrap_or(0)));
    Some(BilinearTaps { texels: [[c0, r0], [c1, r0], [c0, r1], [c1, r1]], weights })
}

/// `floor(t size - 1/2)` and the fraction above it in raw units, `0..2^32`.
fn split(t: FixedQ32_32, size: u32) -> (i128, i128) {
    let position = i128::from(t.raw()) * i128::from(size) - ONE / 2;
    (position >> 32, position & (ONE - 1))
}

/// A product of two fractions in `0..2^32` raw, rounded from Q64.64 to raw units, ties to even.
const fn round_unit(product: i128) -> i128 {
    let (whole, rest) = (product >> 32, product & (ONE - 1));
    if rest > ONE / 2 || (rest == ONE / 2 && whole & 1 == 1) {
        whole + 1
    } else {
        whole
    }
}
