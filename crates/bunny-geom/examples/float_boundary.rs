//! Validate float geometry at the boundary into fixed-point primitives, and see what is refused.
//!
//! Run with `cargo run -p bunny-geom --example float_boundary`.
//!
//! Bunny computes in Q32.32 only. Float input from outside (a file, a UI, a physics engine)
//! enters through fallible conversions — `try_new`, `try_into_fixed`, `try_from_float` and
//! `TryFrom` — which refuse NaN, infinities, values Q32.32 cannot hold, a ray direction that
//! cannot be normalised and an inverted box, each with a typed `GeomError`. Nothing is clamped
//! or replaced silently.

use std::error::Error;

use bunny_geom::{Aabb3, FixedAabb3, FixedRay3, GeomError, Ray3};
use bunny_linalg::{FixedVec3, Vec3};
use bunny_num::FixedQ32_32;

/// What crossed the boundary, and what was refused.
#[derive(Debug)]
struct Report {
    ray: FixedRay3,
    aabb: FixedAabb3,
    lowest: FixedQ32_32,
    refusals: Vec<(&'static str, GeomError)>,
}

// dojo: allow float-boundary -- the example's float input, converted only through try_* APIs
/// Accepted input. Every value is exact in `f32` and in Q32.32, so the raw values are exact:
/// the direction `(0, 0, -2)` is not unit length and is normalised to `(0, 0, -1)`, and
/// `-2^31`, the most negative Q32.32 value, is in range.
fn accepted() -> Result<(FixedRay3, FixedAabb3, FixedQ32_32), Box<dyn Error>> {
    let ray = Ray3::try_new(Vec3::new(0.5, -1.25, 3.0), Vec3::new(0.0, 0.0, -2.0))?;
    let aabb = Aabb3::try_new(Vec3::new(-1.25, 0.0, 0.5), Vec3::new(3.0, 1.0, 0.5))?;
    let lowest = FixedQ32_32::try_from_f32(-2_147_483_648.0)?;
    Ok((ray.try_into_fixed()?, FixedAabb3::try_from(aabb)?, lowest))
}

/// The error of a conversion that must be refused.
fn refused<T>(result: Result<T, GeomError>) -> Result<GeomError, Box<dyn Error>> {
    result.err().ok_or_else(|| "a conversion that must be refused was accepted".into())
}

// dojo: allow float-boundary -- the example's float input, converted only through try_* APIs
/// One refused input per rule. `Ray3` and `Aabb3` have public fields, so a value can skip
/// `try_new`; the conversion checks again, as the NaN case shows.
fn refusals() -> Result<Vec<(&'static str, GeomError)>, Box<dyn Error>> {
    let unit_z = Vec3::new(0.0, 0.0, 1.0);
    let nan_ray = Ray3 { origin: Vec3::new(f32::NAN, 0.0, 0.0), direction: unit_z };
    let infinite_min = Vec3::new(f32::NEG_INFINITY, 0.0, 0.0);
    // 2^31 is the first `f32` above the largest Q32.32 value, 2^31 - 2^-32.
    let too_far = Vec3::new(2_147_483_648.0, 0.0, 0.0);
    let splat = |value| FixedVec3::new(value, value, value);
    let (low, high) = (splat(FixedQ32_32::ZERO), splat(FixedQ32_32::ONE));
    Ok(vec![
        ("NaN origin", refused(FixedRay3::try_from(nan_ray))?),
        ("infinite box corner", refused(Aabb3::try_new(infinite_min, unit_z))?),
        ("origin out of Q32.32 range", refused(Ray3::try_new(too_far, unit_z))?),
        ("zero ray direction", refused(Ray3::try_new(unit_z, Vec3::new(0.0, 0.0, 0.0)))?),
        ("inverted float box", refused(Aabb3::try_new(unit_z, Vec3::new(0.0, 0.0, 0.0)))?),
        ("inverted fixed box", refused(FixedAabb3::try_new(high, low))?),
    ])
}

fn run() -> Result<Report, Box<dyn Error>> {
    let (ray, aabb, lowest) = accepted()?;
    Ok(Report { ray, aabb, lowest, refusals: refusals()? })
}

fn raw(v: FixedVec3) -> [i64; 3] {
    [v.x.raw(), v.y.raw(), v.z.raw()]
}

fn main() -> Result<(), Box<dyn Error>> {
    let report = run()?;
    let direction = report.ray.direction.into_inner();
    println!("ray origin raw {:?}, direction raw {:?}", raw(report.ray.origin), raw(direction));
    println!("box min raw {:?}, max raw {:?}", raw(report.aabb.min), raw(report.aabb.max));
    println!("-2^31 raw {}", report.lowest.raw());
    for (input, error) in &report.refusals {
        println!("refused {input}: {error:?} ({error})");
    }
    Ok(())
}

// dojo: allow panic-path -- test assertions pin the example's output to independent values
#[cfg(test)]
mod tests {
    use super::*;
    use bunny_num::fixed_q32_32::ONE_RAW;

    /// Hand-derived: raw = value * 2^32 for these exactly representable values, so
    /// 0.5 is 2^31, -1.25 is -5 * 2^30, and -2^31 is -2^63 = `i64::MIN`.
    #[test]
    fn accepted_values_are_exact() -> Result<(), Box<dyn Error>> {
        let report = run()?;
        let (half, neg_five_quarters) = (ONE_RAW / 2, -5 * (ONE_RAW / 4));
        assert_eq!(raw(report.ray.origin), [half, neg_five_quarters, 3 * ONE_RAW]);
        assert_eq!(raw(report.ray.direction.into_inner()), [0, 0, -ONE_RAW]);
        assert_eq!(raw(report.aabb.min), [neg_five_quarters, 0, half]);
        assert_eq!(raw(report.aabb.max), [3 * ONE_RAW, ONE_RAW, half]);
        assert_eq!(report.lowest.raw(), i64::MIN);
        Ok(())
    }

    /// The rules in `Ray3::try_new`, `Aabb3::try_new` and `FixedAabb3::try_new`.
    #[test]
    fn each_refusal_has_its_typed_error() -> Result<(), Box<dyn Error>> {
        let errors: Vec<GeomError> = run()?.refusals.into_iter().map(|(_, e)| e).collect();
        assert_eq!(
            errors,
            [
                GeomError::NonFiniteCoordinate,
                GeomError::NonFiniteCoordinate,
                GeomError::FixedValueOutOfRange,
                GeomError::InvalidRayDirection,
                GeomError::InvalidAabbBounds,
                GeomError::InvalidAabbBounds,
            ]
        );
        Ok(())
    }
}
