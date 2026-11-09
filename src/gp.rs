// SPDX-License-Identifier: Apache-2.0

//! Cartesian points, displacement vectors, and checked unit directions.
//!
//! A point is a location; a vector is a displacement. Thus `point - point`
//! produces a vector and `point + vector` produces a point. Point addition and
//! point scaling are deliberately absent. All axes use the same caller-chosen
//! units, and cross products use a right-handed coordinate frame.
//!
//! Points and vectors accept every `f64`, including infinities and NaNs. Their
//! arithmetic follows IEEE 754 rounding, overflow, and NaN propagation. Equality
//! is exact floating-point equality, not a geometric tolerance test. No global
//! tolerance or model scale is assumed; callers choose tolerances for their own
//! algorithms. Only [`UnitDirection3`] enforces finite, nonzero input.

use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};

/// A location in a three-dimensional Cartesian coordinate frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point3 {
    /// Coordinate along the x axis.
    pub x: f64,
    /// Coordinate along the y axis.
    pub y: f64,
    /// Coordinate along the z axis.
    pub z: f64,
}

impl Point3 {
    /// The coordinate origin, also the default value.
    pub const ORIGIN: Self = Self::new(0.0, 0.0, 0.0);

    /// Creates a point without rejecting nonfinite coordinates.
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Returns whether all coordinates are finite.
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    /// Returns Euclidean distance in coordinate units.
    ///
    /// Uses [`Vector3::norm`] after coordinate subtraction. Finite points can
    /// produce infinity if their separation exceeds the representable range;
    /// subtraction involving nonfinite coordinates follows IEEE 754 semantics.
    pub fn distance(self, other: Self) -> f64 {
        (other - self).norm()
    }
}

/// A displacement in a three-dimensional Cartesian coordinate frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vector3 {
    /// Component along the x axis.
    pub x: f64,
    /// Component along the y axis.
    pub y: f64,
    /// Component along the z axis.
    pub z: f64,
}

impl Vector3 {
    /// The zero displacement, also the default value.
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);

    /// Creates a vector without rejecting nonfinite components.
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Returns whether all components are finite.
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    /// Returns `x*other.x + y*other.y + z*other.z`.
    ///
    /// Intermediate products can overflow or underflow; this is not an exact
    /// predicate and does not use compensated arithmetic.
    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Returns the right-handed cross product `self × other`.
    ///
    /// In particular, the positive x axis crossed with the positive y axis
    /// yields the positive z axis. Intermediate products can overflow or
    /// underflow; near-parallel inputs are not snapped to zero.
    pub fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    /// Returns Euclidean length using nested [`f64::hypot`].
    ///
    /// Unlike squaring each component directly, this avoids unnecessary
    /// intermediate overflow and underflow. The result can still be infinite
    /// when the length exceeds `f64::MAX`. Evaluation is `x.hypot(y).hypot(z)`.
    /// At either step, infinity takes precedence over NaN. Consequently, an
    /// infinite component always yields infinity; even with no infinite input,
    /// overflow in the first step can mask a NaN z component (for example,
    /// `(f64::MAX, f64::MAX, f64::NAN)` yields infinity). Otherwise a NaN
    /// component yields NaN.
    pub fn norm(self) -> f64 {
        self.x.hypot(self.y).hypot(self.z)
    }
}

impl Add<Vector3> for Point3 {
    type Output = Self;

    fn add(self, rhs: Vector3) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Sub<Vector3> for Point3 {
    type Output = Self;

    fn sub(self, rhs: Vector3) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Sub for Point3 {
    type Output = Vector3;

    fn sub(self, rhs: Self) -> Vector3 {
        Vector3::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Add for Vector3 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Sub for Vector3 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Neg for Vector3 {
    type Output = Self;

    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z)
    }
}

impl Mul<f64> for Vector3 {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl Mul<Vector3> for f64 {
    type Output = Vector3;

    fn mul(self, rhs: Vector3) -> Vector3 {
        rhs * self
    }
}

impl Div<f64> for Vector3 {
    type Output = Self;

    fn div(self, rhs: f64) -> Self {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}

/// Failure to construct a finite unit direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirectionError {
    /// At least one component is infinite or NaN.
    NonFinite,
    /// All components are zero (including signed zero).
    ZeroVector,
}

impl fmt::Display for DirectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NonFinite => "a direction requires finite components",
            Self::ZeroVector => "a zero vector has no direction",
        })
    }
}

impl std::error::Error for DirectionError {}

/// A finite direction normalized to unit length within floating-point rounding.
///
/// Construct with [`TryFrom<Vector3>`]. Zero and nonfinite inputs are errors;
/// there is no near-zero cutoff. Even the smallest positive subnormal input can
/// define a direction. Private storage prevents mutation of the unit invariant.
/// Normalization can round components too small relative to the largest to zero.
///
/// ```
/// use rcct::gp::{DirectionError, UnitDirection3, Vector3};
/// assert_eq!(UnitDirection3::try_from(Vector3::ZERO), Err(DirectionError::ZeroVector));
/// let d = UnitDirection3::try_from(Vector3::new(f64::MAX, f64::MAX, 0.0))?;
/// assert!((d.as_vector().norm() - 1.0).abs() < 1e-15);
/// # Ok::<(), DirectionError>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnitDirection3(Vector3);

impl UnitDirection3 {
    /// Returns the normalized, dimensionless components by value.
    pub const fn as_vector(self) -> Vector3 {
        self.0
    }
}

impl TryFrom<Vector3> for UnitDirection3 {
    type Error = DirectionError;

    /// Normalizes finite nonzero input without forming its unscaled length.
    ///
    /// Dividing by the largest absolute component first puts every component
    /// in `[-1, 1]` and at least one at magnitude one. The subsequent norm is
    /// therefore in `[1, sqrt(3)]`, avoiding overflow and a zero divisor even
    /// for subnormal inputs. Nonfinite validation precedes the zero test.
    fn try_from(value: Vector3) -> Result<Self, Self::Error> {
        if !value.is_finite() {
            return Err(DirectionError::NonFinite);
        }
        let scale = value.x.abs().max(value.y.abs()).max(value.z.abs());
        if scale == 0.0 {
            return Err(DirectionError::ZeroVector);
        }
        let scaled = value / scale;
        Ok(Self(scaled / scaled.norm()))
    }
}
