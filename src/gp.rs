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
//!
//! # Numerical contract
//!
//! | Operation | Meaning and evaluation | Important limit |
//! | --- | --- | --- |
//! | Point/vector `+`, `-`, scalar `*`, `/` | Componentwise `f64` operations | No validation, clamping, or zero-divisor check |
//! | [`Vector3::dot`] | `(x*other.x + y*other.y) + z*other.z` | Rounded products and sums, not an exact predicate |
//! | [`Vector3::cross`] | Right-handed component determinants | Cancellation can lose a near-parallel result |
//! | [`Vector3::norm`] | `x.hypot(y).hypot(z)` | Platform-dependent precision, possible final overflow |
//! | [`Point3::distance`] | Subtract coordinates, then take the norm | Subtraction can overflow or lose a small separation |
//! | [`UnitDirection3::try_from`] | Validate, scale by largest magnitude, normalize | Rounding can erase relatively tiny components |
//!
//! If coordinates have units `U`, displacements and distances also have units
//! `U`; dot and cross products of two such vectors have units `U²`. Unit
//! directions are dimensionless. These are caller obligations, not types
//! enforced by the crate. Scalars carry whatever units the caller assigns.
//!
//! Signed zeros are accepted and compare equal. Componentwise arithmetic
//! retains their ordinary floating-point behavior, but there is no geometric
//! distinction between them or canonicalization step. NaN sign and payload are
//! not part of the contract. No operation establishes parallelism, orthogonality,
//! coincidence, or orientation robustly for arbitrary inputs.
//!
//! See the repository's [numerical contract and verification notes](
//! https://github.com/Muktidaya/rcct/blob/root/docs/numerics.md) for conditioning,
//! independent reference tests, and the distinction between test budgets and
//! mathematical guarantees.

use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};

/// A location in a three-dimensional Cartesian coordinate frame.
///
/// Public coordinates are unrestricted `f64` values. Equality is componentwise
/// exact floating-point equality: signed zeros are equal and NaNs are unequal.
/// Translation and subtraction perform ordinary rounded component operations;
/// see the [module numerical contract](self#numerical-contract).
///
/// # Displacement between locations
///
/// `end - start` points from `start` toward `end`; reversing the subtraction
/// reverses the displacement. Choose an origin explicitly when a location must
/// be expressed as a vector. The crate does not identify points with vectors.
///
/// ```
/// use rcct::gp::{Point3, Vector3};
///
/// let start = Point3::new(1.0, 2.0, 3.0);
/// let end = Point3::new(4.0, 6.0, 3.0);
/// let displacement = end - start;
/// assert_eq!(displacement, Vector3::new(3.0, 4.0, 0.0));
/// assert_eq!(start + displacement, end);
/// assert_eq!(end - displacement, start);
/// assert_eq!(start - Point3::ORIGIN, Vector3::new(1.0, 2.0, 3.0));
/// ```
///
/// These small integer coordinates make the example exact; in general,
/// floating-point rounding means translation and subtraction need not undo
/// each other exactly.
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
    /// produce infinity if their separation exceeds the representable range.
    /// Coordinates already rounded to the same `f64` cannot recover a smaller
    /// separation. Subtraction involving nonfinite coordinates follows IEEE 754
    /// semantics: in particular `infinity - infinity` produces NaN. The resulting
    /// norm applies its documented infinity/NaN precedence. No coincidence
    /// tolerance or unit conversion is applied.
    pub fn distance(self, other: Self) -> f64 {
        (other - self).norm()
    }
}

/// A displacement in a three-dimensional Cartesian coordinate frame.
///
/// Public components are unrestricted `f64` values. Equality is componentwise
/// exact floating-point equality, not approximate geometric equivalence.
/// Arithmetic operators act componentwise without checking overflow, nonfinite
/// input, or division by zero. See the [module numerical contract](self#numerical-contract).
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
    /// Evaluation is `(x*other.x + y*other.y) + z*other.z`, with separately
    /// rounded products and sums, not fused or compensated arithmetic.
    /// Intermediate products and sums can overflow or underflow even when the
    /// exact result is representable. Opposite infinities can produce NaN even
    /// from finite inputs. Cancellation can destroy relative accuracy near zero;
    /// the result is not a robust orthogonality or sign predicate.
    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Returns the right-handed cross product `self × other`.
    ///
    /// In particular, the positive x axis crossed with the positive y axis
    /// yields the positive z axis. Components are evaluated as
    /// `(y*other.z - z*other.y, z*other.x - x*other.z, x*other.y - y*other.x)`.
    /// Products and differences round separately. They can overflow or underflow,
    /// and `infinity - infinity` can produce NaN even from finite inputs.
    /// Cancellation can destroy relative accuracy for near-parallel inputs;
    /// results are not snapped to zero and do not robustly decide parallelism.
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
    /// component yields NaN. An all-zero vector has positive-zero length.
    ///
    /// [`f64::hypot`] has unspecified precision that can vary by platform and
    /// Rust version; this method promises neither correct rounding nor a fixed
    /// ULP bound. The verification suite's error budgets are empirical acceptance
    /// criteria for tested cases, not universal bounds.
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
/// Normalization can round components too small relative to the largest to zero,
/// including signed zero. The result is dimensionless; it need not have a norm
/// whose floating-point representation is exactly `1.0`. Equality remains exact
/// component equality, so differently scaled inputs need not produce identical
/// bits. No fixed angular-error bound is promised.
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
    ///
    /// The returned vector is an independent copy: modifying or scaling it does
    /// not change this direction. Multiplying it by a caller-chosen length gives
    /// a displacement in that length's units, subject to ordinary rounding.
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
    /// mathematically in `[1, sqrt(3)]` (subject to rounding), avoiding overflow
    /// and a zero divisor even for subnormal inputs. The final division uses this
    /// scaled norm rather than the potentially overflowing original length.
    /// Nonfinite validation precedes the zero test. No caller-selected tolerance
    /// is applied implicitly; callers needing a minimum magnitude must check it
    /// separately before construction.
    fn try_from(value: Vector3) -> Result<Self, Self::Error> {
        // Validate before reducing with f64::max: max can ignore a NaN operand,
        // so a finite scale alone would not establish finite input components.
        if !value.is_finite() {
            return Err(DirectionError::NonFinite);
        }
        let scale = value.x.abs().max(value.y.abs()).max(value.z.abs());
        if scale == 0.0 {
            return Err(DirectionError::ZeroVector);
        }
        // Divide components directly. Multiplying by scale.recip() instead
        // would overflow that reciprocal for sufficiently small subnormal scale.
        let scaled = value / scale;
        // Keep the norm in scaled coordinates. Reconstructing scale * norm
        // could overflow even though the direction itself is representable.
        Ok(Self(scaled / scaled.norm()))
    }
}
