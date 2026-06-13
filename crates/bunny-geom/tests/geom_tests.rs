use bunny_geom::{Aabb3, FixedAabb3, FixedRay3, FixedSphere3, GeomError, Ray3, Sphere3};
use bunny_linalg::{FixedVec3, Vec3};
use bunny_num::FixedQ32_32;
use std::convert::TryFrom;
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test(unsupported = test)]
fn test_fixed_ray3_creation_and_conversion() {
    let origin = FixedVec3::new(
        FixedQ32_32::from_f32(1.0),
        FixedQ32_32::from_f32(2.0),
        FixedQ32_32::from_f32(3.0),
    );
    let direction = FixedVec3::new(
        FixedQ32_32::from_f32(0.0),
        FixedQ32_32::from_f32(1.0),
        FixedQ32_32::from_f32(0.0),
    );

    let udir = bunny_linalg::FixedUnitVec3::new(direction).unwrap();
    let fixed_ray = FixedRay3::new(origin, udir);
    assert_eq!(fixed_ray.origin, origin);
    assert_eq!(fixed_ray.direction, udir);

    // Roundtrip
    let float_ray = Ray3::from(fixed_ray);
    assert_eq!(float_ray.origin.x, 1.0);
    assert_eq!(float_ray.origin.y, 2.0);
    assert_eq!(float_ray.origin.z, 3.0);
    assert_eq!(float_ray.direction.x, 0.0);
    assert_eq!(float_ray.direction.y, 1.0);
    assert_eq!(float_ray.direction.z, 0.0);

    assert_eq!(FixedRay3::try_from(float_ray), Ok(fixed_ray));
}

#[wasm_bindgen_test(unsupported = test)]
fn test_fixed_aabb3_creation_and_conversion() {
    let min = FixedVec3::new(
        FixedQ32_32::from_f32(-1.0),
        FixedQ32_32::from_f32(-1.0),
        FixedQ32_32::from_f32(-1.0),
    );
    let max = FixedVec3::new(
        FixedQ32_32::from_f32(1.0),
        FixedQ32_32::from_f32(1.0),
        FixedQ32_32::from_f32(1.0),
    );

    let fixed_aabb = FixedAabb3::new(min, max);
    assert_eq!(fixed_aabb.min, min);
    assert_eq!(fixed_aabb.max, max);

    // Roundtrip
    let float_aabb = Aabb3::from(fixed_aabb);
    assert_eq!(float_aabb.min.x, -1.0);
    assert_eq!(float_aabb.max.z, 1.0);

    let fixed_aabb_back = FixedAabb3::try_from(float_aabb);
    assert_eq!(fixed_aabb_back, Ok(fixed_aabb));
}

#[wasm_bindgen_test(unsupported = test)]
fn test_fixed_sphere3_creation_and_conversion() {
    let center = FixedVec3::new(
        FixedQ32_32::from_f32(5.0),
        FixedQ32_32::from_f32(6.0),
        FixedQ32_32::from_f32(7.0),
    );
    let radius = FixedQ32_32::from_f32(2.5);

    let fixed_sphere = FixedSphere3::new(center, radius);
    assert_eq!(fixed_sphere.center, center);
    assert_eq!(fixed_sphere.radius, radius);

    // Roundtrip
    let float_sphere = Sphere3::from(fixed_sphere);
    assert_eq!(float_sphere.center.x, 5.0);
    assert_eq!(float_sphere.radius, 2.5);

    let fixed_sphere_back = FixedSphere3::try_from(float_sphere);
    assert_eq!(fixed_sphere_back, Ok(fixed_sphere));
}

#[wasm_bindgen_test(unsupported = test)]
fn test_geom_validation_constructors() {
    use bunny_geom::GeomError;

    let p0 = FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO);
    let p1 = FixedVec3::new(FixedQ32_32::ONE, FixedQ32_32::ONE, FixedQ32_32::ONE);

    // 1. AABB validation
    assert!(FixedAabb3::try_new(p0, p1).is_ok());
    assert_eq!(
        FixedAabb3::try_new(p1, p0),
        Err(GeomError::InvalidAabbBounds)
    );

    // 2. Sphere validation
    assert!(FixedSphere3::try_new(p0, FixedQ32_32::ZERO).is_ok());
    assert!(FixedSphere3::try_new(p0, FixedQ32_32::ONE).is_ok());
    assert_eq!(
        FixedSphere3::try_new(p0, FixedQ32_32::from_f32(-1.0)),
        Err(GeomError::NegativeSphereRadius)
    );

    // 3. Ray validation
    assert!(FixedRay3::try_new(p0, p1).is_ok());
    assert_eq!(
        FixedRay3::try_new(p0, p0),
        Err(GeomError::InvalidRayDirection)
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn test_ray3_conversion_rejects_invalid_direction() {
    use bunny_geom::GeomError;

    let ray = Ray3 {
        origin: Vec3::new(0.0, 0.0, 0.0),
        direction: Vec3::new(0.0, 0.0, 0.0),
    };
    let converted: Result<FixedRay3, GeomError> = FixedRay3::try_from(ray);

    assert_eq!(converted, Err(GeomError::InvalidRayDirection));

    let non_finite_origin = Ray3 {
        origin: Vec3::new(f32::INFINITY, 0.0, 0.0),
        direction: Vec3::new(1.0, 0.0, 0.0),
    };
    let converted_origin: Result<FixedRay3, GeomError> = FixedRay3::try_from(non_finite_origin);
    assert_eq!(converted_origin, Err(GeomError::NonFiniteCoordinate));

    let non_finite_direction = Ray3 {
        origin: Vec3::new(0.0, 0.0, 0.0),
        direction: Vec3::new(f32::INFINITY, 0.0, 0.0),
    };
    let converted_direction: Result<FixedRay3, GeomError> =
        FixedRay3::try_from(non_finite_direction);
    assert_eq!(converted_direction, Err(GeomError::NonFiniteCoordinate));
}

#[wasm_bindgen_test(unsupported = test)]
fn test_geom_error_implements_std_error() {
    use bunny_geom::GeomError;

    fn assert_error<E: std::error::Error>() {}
    assert_error::<GeomError>();

    assert_eq!(
        GeomError::InvalidAabbBounds.to_string(),
        "AABB min boundary exceeds max boundary"
    );
    assert_eq!(
        GeomError::NonFiniteCoordinate.to_string(),
        "coordinate is not finite"
    );
    assert_eq!(
        GeomError::NegativeSphereRadius.to_string(),
        "sphere radius is negative"
    );
    assert_eq!(
        GeomError::NonFiniteRadius.to_string(),
        "sphere radius is not finite"
    );
    assert_eq!(
        GeomError::InvalidRayDirection.to_string(),
        "ray direction normalization failed"
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn test_float_shape_conversions_reject_invalid_bounds() {
    let invalid_aabb = Aabb3 {
        min: Vec3::new(1.0, 0.0, 0.0),
        max: Vec3::new(0.0, 0.0, 0.0),
    };
    let converted_aabb: Result<FixedAabb3, GeomError> = FixedAabb3::try_from(invalid_aabb);
    assert_eq!(converted_aabb, Err(GeomError::InvalidAabbBounds));

    let sub_lsb_invalid_aabb = Aabb3 {
        min: Vec3::new(1e-12, 0.0, 0.0),
        max: Vec3::new(0.0, 0.0, 0.0),
    };
    let converted_sub_lsb_aabb: Result<FixedAabb3, GeomError> =
        FixedAabb3::try_from(sub_lsb_invalid_aabb);
    assert_eq!(converted_sub_lsb_aabb, Err(GeomError::InvalidAabbBounds));

    let non_finite_aabb = Aabb3 {
        min: Vec3::new(f32::NAN, 0.0, 0.0),
        max: Vec3::new(0.0, 0.0, 0.0),
    };
    let converted_non_finite_aabb: Result<FixedAabb3, GeomError> =
        FixedAabb3::try_from(non_finite_aabb);
    assert_eq!(
        converted_non_finite_aabb,
        Err(GeomError::NonFiniteCoordinate)
    );

    let invalid_sphere = Sphere3 {
        center: Vec3::new(0.0, 0.0, 0.0),
        radius: -1.0,
    };
    let converted_sphere: Result<FixedSphere3, GeomError> = FixedSphere3::try_from(invalid_sphere);
    assert_eq!(converted_sphere, Err(GeomError::NegativeSphereRadius));

    let sub_lsb_invalid_sphere = Sphere3 {
        center: Vec3::new(0.0, 0.0, 0.0),
        radius: -1e-12,
    };
    let converted_sub_lsb_sphere: Result<FixedSphere3, GeomError> =
        FixedSphere3::try_from(sub_lsb_invalid_sphere);
    assert_eq!(
        converted_sub_lsb_sphere,
        Err(GeomError::NegativeSphereRadius)
    );

    let non_finite_sphere = Sphere3 {
        center: Vec3::new(0.0, 0.0, 0.0),
        radius: f32::INFINITY,
    };
    let converted_non_finite_sphere: Result<FixedSphere3, GeomError> =
        FixedSphere3::try_from(non_finite_sphere);
    assert_eq!(converted_non_finite_sphere, Err(GeomError::NonFiniteRadius));

    let non_finite_center_sphere = Sphere3 {
        center: Vec3::new(f32::INFINITY, 0.0, 0.0),
        radius: 1.0,
    };
    let converted_non_finite_center_sphere: Result<FixedSphere3, GeomError> =
        FixedSphere3::try_from(non_finite_center_sphere);
    assert_eq!(
        converted_non_finite_center_sphere,
        Err(GeomError::NonFiniteCoordinate)
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn test_explicit_float_boundary_conversion_apis() {
    let ray = Ray3::try_new(Vec3::new(1.0, 2.0, 3.0), Vec3::new(0.0, 1.0, 0.0))
        .expect("valid ray should be accepted");
    let fixed_ray = ray.try_into_fixed().expect("valid ray should convert");
    assert_eq!(FixedRay3::try_from_float(ray), Ok(fixed_ray));
    assert_eq!(fixed_ray.into_float(), Ray3::from(fixed_ray));
    assert_eq!(
        Ray3::try_new(Vec3::new(f32::NAN, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)),
        Err(GeomError::NonFiniteCoordinate)
    );
    assert_eq!(
        Ray3::try_new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 0.0)),
        Err(GeomError::InvalidRayDirection)
    );

    let aabb = Aabb3::try_new(Vec3::new(-1.0, -2.0, -3.0), Vec3::new(1.0, 2.0, 3.0))
        .expect("valid AABB should be accepted");
    let fixed_aabb = aabb.try_into_fixed().expect("valid AABB should convert");
    assert_eq!(FixedAabb3::try_from_float(aabb), Ok(fixed_aabb));
    assert_eq!(fixed_aabb.into_float(), Aabb3::from(fixed_aabb));
    assert_eq!(
        Aabb3::try_new(Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 0.0)),
        Err(GeomError::InvalidAabbBounds)
    );
    assert_eq!(
        Aabb3::try_new(Vec3::new(f32::INFINITY, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0)),
        Err(GeomError::NonFiniteCoordinate)
    );

    let sphere =
        Sphere3::try_new(Vec3::new(0.5, 1.5, 2.5), 3.5).expect("valid sphere should be accepted");
    let fixed_sphere = sphere
        .try_into_fixed()
        .expect("valid sphere should convert");
    assert_eq!(FixedSphere3::try_from_float(sphere), Ok(fixed_sphere));
    assert_eq!(fixed_sphere.into_float(), Sphere3::from(fixed_sphere));
    assert_eq!(
        Sphere3::try_new(Vec3::new(0.0, 0.0, 0.0), -1.0),
        Err(GeomError::NegativeSphereRadius)
    );
    assert_eq!(
        Sphere3::try_new(Vec3::new(0.0, f32::NAN, 0.0), 1.0),
        Err(GeomError::NonFiniteCoordinate)
    );
    assert_eq!(
        Sphere3::try_new(Vec3::new(0.0, 0.0, 0.0), f32::INFINITY),
        Err(GeomError::NonFiniteRadius)
    );
}
