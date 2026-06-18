use crate::{vec3_is_finite, Aabb3, FixedAabb3, FixedRay3, FixedSphere3, GeomError, Ray3, Sphere3};
use bunny_linalg::{FixedUnitVec3, FixedVec3, Vec3};
use bunny_num::{is_finite, FixedQ32_32, Scalar};

fn fixed_vec3_from_float(value: Vec3) -> Result<FixedVec3, GeomError> {
    FixedVec3::try_from_float(value).map_err(|_| GeomError::FixedValueOutOfRange)
}

fn fixed_scalar_from_float(value: Scalar) -> Result<FixedQ32_32, GeomError> {
    FixedQ32_32::try_from_f32(value).map_err(|_| GeomError::FixedValueOutOfRange)
}

fn validate_fixed_vec3(value: Vec3) -> Result<(), GeomError> {
    fixed_vec3_from_float(value).map(|_| ())
}

fn validate_fixed_direction(direction: Vec3) -> Result<(), GeomError> {
    FixedUnitVec3::new(fixed_vec3_from_float(direction)?)
        .map(|_| ())
        .ok_or(GeomError::InvalidRayDirection)
}

fn validate_ray3(origin: Vec3, direction: Vec3) -> Result<(), GeomError> {
    if !vec3_is_finite(origin) || !vec3_is_finite(direction) {
        return Err(GeomError::NonFiniteCoordinate);
    }
    validate_fixed_vec3(origin)?;
    validate_fixed_direction(direction)?;
    Ok(())
}

fn validate_aabb3(min: Vec3, max: Vec3) -> Result<(), GeomError> {
    if !vec3_is_finite(min) || !vec3_is_finite(max) {
        return Err(GeomError::NonFiniteCoordinate);
    }
    if min.x > max.x || min.y > max.y || min.z > max.z {
        return Err(GeomError::InvalidAabbBounds);
    }
    validate_fixed_vec3(min)?;
    validate_fixed_vec3(max)?;
    Ok(())
}

fn validate_sphere3(center: Vec3, radius: Scalar) -> Result<(), GeomError> {
    if !vec3_is_finite(center) {
        return Err(GeomError::NonFiniteCoordinate);
    }
    if !is_finite(radius) {
        return Err(GeomError::NonFiniteRadius);
    }
    if radius < Scalar::from_bits(0) {
        return Err(GeomError::NegativeSphereRadius);
    }
    validate_fixed_vec3(center)?;
    fixed_scalar_from_float(radius)?;
    Ok(())
}

impl Ray3 {
    /// Creates a float ray that is valid for fixed-point boundary conversion.
    ///
    /// # Errors
    /// Returns `GeomError::NonFiniteCoordinate` if any component is not finite,
    /// or `GeomError::InvalidRayDirection` if the direction cannot be
    /// represented as a fixed unit vector.
    pub fn try_new(origin: Vec3, direction: Vec3) -> Result<Self, GeomError> {
        validate_ray3(origin, direction)?;
        Ok(Self { origin, direction })
    }

    /// Converts this float ray into fixed-point coordinates with validation.
    ///
    /// # Errors
    /// Returns a `GeomError` if the ray contains non-finite coordinates or the
    /// direction cannot be represented as a fixed unit vector.
    pub fn try_into_fixed(self) -> Result<FixedRay3, GeomError> {
        FixedRay3::try_from_float(self)
    }
}

impl Aabb3 {
    /// Creates a float AABB that is valid for fixed-point boundary conversion.
    ///
    /// # Errors
    /// Returns `GeomError::NonFiniteCoordinate` if any component is not finite,
    /// or `GeomError::InvalidAabbBounds` if any minimum component exceeds its
    /// matching maximum component.
    pub fn try_new(min: Vec3, max: Vec3) -> Result<Self, GeomError> {
        validate_aabb3(min, max)?;
        Ok(Self { min, max })
    }

    /// Converts this float AABB into fixed-point coordinates with validation.
    ///
    /// # Errors
    /// Returns a `GeomError` if the AABB contains non-finite coordinates or
    /// invalid bounds.
    pub fn try_into_fixed(self) -> Result<FixedAabb3, GeomError> {
        FixedAabb3::try_from_float(self)
    }
}

impl Sphere3 {
    /// Creates a float sphere that is valid for fixed-point boundary conversion.
    ///
    /// # Errors
    /// Returns `GeomError::NonFiniteCoordinate` if the center contains a
    /// non-finite component, `GeomError::NonFiniteRadius` if the radius is not
    /// finite, or `GeomError::NegativeSphereRadius` if the radius is negative.
    pub fn try_new(center: Vec3, radius: Scalar) -> Result<Self, GeomError> {
        validate_sphere3(center, radius)?;
        Ok(Self { center, radius })
    }

    /// Converts this float sphere into fixed-point coordinates with validation.
    ///
    /// # Errors
    /// Returns a `GeomError` if the sphere contains non-finite values or a
    /// negative radius.
    pub fn try_into_fixed(self) -> Result<FixedSphere3, GeomError> {
        FixedSphere3::try_from_float(self)
    }
}

impl FixedRay3 {
    /// Converts a float ray into fixed-point coordinates with validation.
    ///
    /// # Errors
    /// Returns a `GeomError` if the ray contains non-finite coordinates or the
    /// direction cannot be represented as a fixed unit vector.
    pub fn try_from_float(ray: Ray3) -> Result<Self, GeomError> {
        validate_ray3(ray.origin, ray.direction)?;
        Self::try_new(fixed_vec3_from_float(ray.origin)?, fixed_vec3_from_float(ray.direction)?)
    }

    /// Converts this fixed-point ray into float coordinates without validation.
    #[must_use]
    pub fn into_float(self) -> Ray3 {
        self.into()
    }
}

impl FixedAabb3 {
    /// Converts a float AABB into fixed-point coordinates with validation.
    ///
    /// # Errors
    /// Returns a `GeomError` if the AABB contains non-finite coordinates or
    /// invalid bounds.
    pub fn try_from_float(aabb: Aabb3) -> Result<Self, GeomError> {
        validate_aabb3(aabb.min, aabb.max)?;
        Self::try_new(fixed_vec3_from_float(aabb.min)?, fixed_vec3_from_float(aabb.max)?)
    }

    /// Converts this fixed-point AABB into float coordinates without validation.
    #[must_use]
    pub fn into_float(self) -> Aabb3 {
        self.into()
    }
}

impl FixedSphere3 {
    /// Converts a float sphere into fixed-point coordinates with validation.
    ///
    /// # Errors
    /// Returns a `GeomError` if the sphere contains non-finite values or a
    /// negative radius.
    pub fn try_from_float(sphere: Sphere3) -> Result<Self, GeomError> {
        validate_sphere3(sphere.center, sphere.radius)?;
        Self::try_new(
            fixed_vec3_from_float(sphere.center)?,
            fixed_scalar_from_float(sphere.radius)?,
        )
    }

    /// Converts this fixed-point sphere into float coordinates without validation.
    #[must_use]
    pub fn into_float(self) -> Sphere3 {
        self.into()
    }
}

impl TryFrom<Ray3> for FixedRay3 {
    type Error = GeomError;

    fn try_from(ray: Ray3) -> Result<Self, Self::Error> {
        Self::try_from_float(ray)
    }
}

impl From<FixedRay3> for Ray3 {
    fn from(ray: FixedRay3) -> Self {
        Self { origin: Vec3::from(ray.origin), direction: Vec3::from(ray.direction.into_inner()) }
    }
}

impl TryFrom<Aabb3> for FixedAabb3 {
    type Error = GeomError;

    fn try_from(aabb: Aabb3) -> Result<Self, Self::Error> {
        Self::try_from_float(aabb)
    }
}

impl From<FixedAabb3> for Aabb3 {
    fn from(aabb: FixedAabb3) -> Self {
        Self { min: Vec3::from(aabb.min), max: Vec3::from(aabb.max) }
    }
}

impl TryFrom<Sphere3> for FixedSphere3 {
    type Error = GeomError;

    fn try_from(sphere: Sphere3) -> Result<Self, Self::Error> {
        Self::try_from_float(sphere)
    }
}

impl From<FixedSphere3> for Sphere3 {
    fn from(sphere: FixedSphere3) -> Self {
        Self { center: Vec3::from(sphere.center), radius: sphere.radius.to_f32() }
    }
}
