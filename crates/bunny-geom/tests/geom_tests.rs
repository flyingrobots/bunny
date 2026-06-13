use bunny_geom::{Aabb3, FixedAabb3, FixedRay3, FixedSphere3, Ray3, Sphere3};
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
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

    let fixed_ray_back = FixedRay3::from(float_ray);
    assert_eq!(fixed_ray_back, fixed_ray);
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

    let fixed_aabb_back = FixedAabb3::from(float_aabb);
    assert_eq!(fixed_aabb_back, fixed_aabb);
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

    let fixed_sphere_back = FixedSphere3::from(float_sphere);
    assert_eq!(fixed_sphere_back, fixed_sphere);
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
