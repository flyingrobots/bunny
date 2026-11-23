//! The thin-lens camera `bunny-thin-lens/1`: rays from a point on a lens disc through each
//! sample's point on the focus plane.

use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::{FixedAngle, FixedQ32_32};
use bunny_query::{TraceRay, TRACE_COORD_LIMIT_RAW};

use crate::camera::{narrow, neighbour_directions, sample_direction};
use crate::footprint::{measure, RayFootprint, RayFootprintError, TexturedTriangle};
use crate::plane::{dot, l1, wide, Lens, Margin, Plane, Wide3};
use crate::{CameraError, CameraSpec, PinholeCamera, PixelSample};

/// The versioned name of [`ThinLensCamera::ray`] and [`lens_disc_point`]. Any change to their
/// output is a new version.
pub const THIN_LENS_ID: &str = "bunny-thin-lens/1";

/// `u` is multiplied by `DISC_SCALE_RAW / 2^32 = 1 - 2^-28` before its square root, so the
/// rounded disc point stays strictly inside the unit disc.
const DISC_SCALE_RAW: u128 = (1 << 32) - 16;
const FRACTIONAL_BITS: u32 = FixedQ32_32::FRACTIONAL_BITS;

/// What a [`ThinLensCamera`] is built from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThinLensSpec {
    /// The pinhole camera whose position is the lens centre and whose image fixes each ray's
    /// focus-plane point.
    pub camera: CameraSpec,
    /// The lens radius, at least zero. Zero gives the pinhole camera's rays bit for bit.
    pub aperture_radius: FixedQ32_32,
    /// The distance of the focus plane along `forward`, positive.
    pub focus_distance: FixedQ32_32,
}

/// Why a [`ThinLensCamera`] could not be built from a [`ThinLensSpec`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThinLensError {
    /// The pinhole camera spec is refused, for this reason.
    Camera(CameraError),
    /// The focus distance is zero or negative.
    FocusDistanceNotPositive,
    /// The aperture radius is negative.
    ApertureNegative,
    /// A position coordinate plus twice the aperture radius reaches the trace domain bound
    /// (`2^40` raw), so a lens point could fall outside it.
    LensOutsideTraceDomain,
    /// No scale keeps every direction within 1.0 per component and ahead of the camera: the
    /// aperture is too large for the focus distance.
    Unrepresentable,
}

impl core::fmt::Display for ThinLensError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Camera(error) => write!(f, "thin-lens camera: {error}"),
            Self::FocusDistanceNotPositive => f.write_str("focus distance must be positive"),
            Self::ApertureNegative => f.write_str("aperture radius must not be negative"),
            Self::LensOutsideTraceDomain => f.write_str("lens reaches outside the trace domain"),
            Self::Unrepresentable => f.write_str("no scale fits this aperture and focus distance"),
        }
    }
}

impl std::error::Error for ThinLensError {}

/// A deterministic thin-lens camera that generates [`TraceRay`]s.
///
/// Each ray starts at a point of the lens disc, centred on the position and spanned by `right`
/// and `up`, and passes the pixel sample's focus-plane point: where the pinhole ray through the
/// same sample reaches the plane `focus_distance` ahead. See
/// `docs/topics/optics/README.md#thin-lens-camera`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThinLensCamera {
    position: FixedVec3,
    lens: Lens,
    scale_shift: u32,
    plane: Plane,
    aperture_raw: i64,
    focus_raw: i64,
}

impl ThinLensCamera {
    /// Builds a camera.
    ///
    /// # Errors
    /// The first refusal, checked in this order: the pinhole spec ([`ThinLensError::Camera`]),
    /// the focus distance, the aperture radius, the lens against the trace domain, the scale.
    pub fn new(spec: &ThinLensSpec) -> Result<Self, ThinLensError> {
        let pinhole = PinholeCamera::new(&spec.camera).map_err(ThinLensError::Camera)?;
        let (focus_raw, aperture_raw) = (spec.focus_distance.raw(), spec.aperture_radius.raw());
        if focus_raw <= 0 {
            return Err(ThinLensError::FocusDistanceNotPositive);
        }
        if aperture_raw < 0 {
            return Err(ThinLensError::ApertureNegative);
        }
        let (position, lens) = pinhole.parts();
        check_lens_domain(position, aperture_raw)?;
        let margin = |shift| lens_margin(&lens, aperture_raw, focus_raw, shift);
        let (scale_shift, plane) =
            Plane::fit_with(&lens, margin).ok_or(ThinLensError::Unrepresentable)?;
        Ok(Self { position, lens, scale_shift, plane, aperture_raw, focus_raw })
    }

    /// The ray for `sample` through the lens point of `lens_sample`, a point of `[0, 1)^2` (one
    /// sampler pair; the caller chooses which). `None` when the sample is outside the image
    /// (as [`PinholeCamera::ray`]) or a lens coordinate is outside `[0, 1)`.
    ///
    /// Every accepted input yields a ray: each direction component is at most 1.0 in magnitude
    /// and the direction points ahead of the camera.
    #[must_use]
    pub fn ray(&self, sample: PixelSample, lens_sample: [FixedQ32_32; 2]) -> Option<TraceRay> {
        let (origin, correction) = self.lens_point(lens_sample)?;
        let pinhole = sample_direction(&self.plane, &self.lens, sample)?;
        let direction = map2(pinhole, correction, |d, c| d - c);
        TraceRay::new(vec3(origin)?, vec3(direction)?)
    }

    /// The texture-coordinate footprint of `sample`'s pixel on `surface` with the lens point of
    /// `lens_sample` held fixed (`bunny-ray-footprint/1`): the rule of
    /// [`PinholeCamera::footprint`], with this camera's three rays from the one lens point.
    /// With aperture 0 it is the pinhole camera's footprint, bit for bit.
    ///
    /// # Errors
    /// In this order: [`RayFootprintError::SampleOutsideImage`],
    /// [`RayFootprintError::LensSampleOutsideUnitSquare`], then the refusals of
    /// [`PinholeCamera::footprint`] for `sample`'s own ray.
    pub fn footprint(
        &self,
        sample: PixelSample,
        lens_sample: [FixedQ32_32; 2],
        surface: &TexturedTriangle,
    ) -> Result<RayFootprint, RayFootprintError> {
        let rays = neighbour_directions(&self.plane, &self.lens, sample)
            .ok_or(RayFootprintError::SampleOutsideImage)?;
        let (origin, correction) =
            self.lens_point(lens_sample).ok_or(RayFootprintError::LensSampleOutsideUnitSquare)?;
        measure(origin, rays.map(|d| map2(d, correction, |d, c| d - c)), surface)
    }

    /// The ray origin `position + o` and the lens correction `c` of `lens_sample`, or `None`
    /// when a coordinate is outside `[0, 1)`; see the README's Thin-Lens Camera steps.
    fn lens_point(&self, lens_sample: [FixedQ32_32; 2]) -> Option<(Wide3, Wide3)> {
        let [x, y] = lens_disc_point(lens_sample)?.map(|c| i128::from(c.raw()));
        let basis = self.lens.basis;
        let along = |axis| wide(axis).map(|c| c * i128::from(self.aperture_raw));
        let offset = map2(along(basis.right), along(basis.up), |r, u| {
            round_div(x * r + y * u, 1 << (2 * FRACTIONAL_BITS))
        });
        let divisor = i128::from(self.focus_raw) << self.scale_shift;
        let correction = offset.map(|o| round_div(o << FRACTIONAL_BITS, divisor));
        Some((map2(raw3(self.position), offset, |p, o| p + o), correction))
    }

    /// The lens centre.
    #[must_use]
    pub const fn position(&self) -> FixedVec3 {
        self.position
    }

    /// The unit viewing direction, the lens axis.
    #[must_use]
    pub const fn forward(&self) -> FixedUnitVec3 {
        self.lens.basis.forward
    }

    /// The unit direction of growing columns and of the lens disc's first coordinate.
    #[must_use]
    pub const fn right(&self) -> FixedUnitVec3 {
        self.lens.basis.right
    }

    /// The unit up direction and that of the lens disc's second coordinate.
    #[must_use]
    pub const fn up(&self) -> FixedUnitVec3 {
        self.lens.basis.up
    }

    /// The lens radius.
    #[must_use]
    pub const fn aperture_radius(&self) -> FixedQ32_32 {
        FixedQ32_32::from_raw(self.aperture_raw)
    }

    /// The focus distance.
    #[must_use]
    pub const fn focus_distance(&self) -> FixedQ32_32 {
        FixedQ32_32::from_raw(self.focus_raw)
    }

    /// `k` such that each pinhole part of a direction is `2^-k` times its image-plane offset;
    /// at least the pinhole camera's, and equal to it for aperture 0.
    #[must_use]
    pub const fn scale_shift(&self) -> u32 {
        self.scale_shift
    }
}

/// The point of the unit disc for a sampler point `[u, v]` in `[0, 1)^2`, or `None` outside it.
///
/// Polar and area-preserving: radius `sqrt(u * (1 - 2^-28))`, floored to raw, and angle `v`
/// turns counter-clockwise from the first coordinate; each coordinate is the radius times the
/// cosine or sine of `FixedAngle::sin_cos`, rounded once to nearest, ties to even. The result
/// has `x^2 + y^2 < 1` exactly.
#[must_use]
pub fn lens_disc_point(point: [FixedQ32_32; 2]) -> Option<[FixedQ32_32; 2]> {
    let [u, v] = point.map(FixedQ32_32::raw);
    if !(0..ONE_RAW).contains(&u) || !(0..ONE_RAW).contains(&v) {
        return None;
    }
    let radius = FixedQ32_32::sqrt_u128(u128::from(u.unsigned_abs()) * DISC_SCALE_RAW);
    let angle = FixedAngle::from_raw(v.unsigned_abs() << FRACTIONAL_BITS);
    let (sin, cos) = angle.sin_cos();
    // The radius is below 2^32, so it fits in `i128` with room for the products.
    let radius = i128::try_from(radius).ok()?;
    let coordinate = |c: FixedQ32_32| round_div(radius * i128::from(c.raw()), 1 << FRACTIONAL_BITS);
    let [x, y] = [coordinate(cos), coordinate(sin)].map(narrow);
    Some([x?, y?])
}

/// Refuses a lens whose points could leave the trace domain: every lens offset component is
/// at most `2 * aperture` raw (see the README's Thin-Lens Camera bounds).
fn check_lens_domain(position: FixedVec3, aperture_raw: i64) -> Result<(), ThinLensError> {
    let reach = 2 * i128::from(aperture_raw);
    let inside = |c: &i128| c.abs() + reach < i128::from(TRACE_COORD_LIMIT_RAW);
    if raw3(position).iter().all(inside) {
        Ok(())
    } else {
        Err(ThinLensError::LensOutsideTraceDomain)
    }
}

/// The room the lens correction needs at `shift`: `ceil(2a 2^32 / (f 2^k))` raw per
/// component, and the bound on `|c . forward|`. Both are zero when the aperture is zero.
fn lens_margin(lens: &Lens, aperture_raw: i64, focus_raw: i64, shift: u32) -> Margin {
    let (aperture, divisor) = (i128::from(aperture_raw), i128::from(focus_raw) << shift);
    let forward = wide(lens.basis.forward);
    let leak = dot(wide(lens.basis.right), forward).abs() + dot(wide(lens.basis.up), forward).abs();
    // One when the aperture is positive: the roundings of a zero offset are exact.
    let rounding = aperture.min(1) * l1(forward);
    let scaled = aperture * leak + (rounding << (FRACTIONAL_BITS - 1));
    Margin {
        component: ceil_div((2 * aperture) << FRACTIONAL_BITS, divisor),
        forward: ceil_div(scaled, divisor) + rounding,
    }
}

/// `numerator / denominator` rounded to nearest, ties to even, for `denominator > 0`.
const fn round_div(numerator: i128, denominator: i128) -> i128 {
    let (floor, remainder) = (numerator.div_euclid(denominator), numerator.rem_euclid(denominator));
    let twice = 2 * remainder;
    if twice > denominator || (twice == denominator && floor & 1 == 1) {
        floor + 1
    } else {
        floor
    }
}

/// `ceil(numerator / denominator)` for `numerator >= 0` and `denominator > 0`.
const fn ceil_div(numerator: i128, denominator: i128) -> i128 {
    (numerator + denominator - 1) / denominator
}

fn raw3(v: FixedVec3) -> Wide3 {
    [v.x, v.y, v.z].map(|c| i128::from(c.raw()))
}

fn vec3(v: Wide3) -> Option<FixedVec3> {
    let [x, y, z] = v.map(narrow);
    Some(FixedVec3::new(x?, y?, z?))
}

fn map2(a: Wide3, b: Wide3, f: impl Fn(i128, i128) -> i128) -> Wide3 {
    let ([a0, a1, a2], [b0, b1, b2]) = (a, b);
    [f(a0, b0), f(a1, b1), f(a2, b2)]
}
