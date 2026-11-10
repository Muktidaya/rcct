// SPDX-License-Identifier: Apache-2.0

//! Checked 3D primitives and closed axis-aligned bounds.
//!
//! Finite storage and comparisons do not establish robust geometric incidence.
//! Line evaluation and plane offset use separately rounded `f64` arithmetic;
//! their results are not exact predicates. Frames and units remain caller-owned.
//! Equality compares stored representations, not geometric equivalence.
//! This module does not provide intersections, topology, or solid modeling.

use crate::gp::{Point3, UnitDirection3};
use std::fmt;

/// Failure to construct a checked spatial value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpatialError3 {
    /// A coordinate was infinite or NaN.
    NonFinitePoint,
    /// A lower box coordinate exceeded its upper coordinate.
    InvertedBounds,
    /// A segment's endpoints compare equal.
    DegenerateSegment,
}
impl fmt::Display for SpatialError3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinitePoint => f.write_str("point coordinates must be finite"),
            Self::InvertedBounds => f.write_str("lower bound exceeds upper bound"),
            Self::DegenerateSegment => f.write_str("segment endpoints must differ"),
        }
    }
}
impl std::error::Error for SpatialError3 {}

/// A point whose three stored coordinates are finite.
/// Signed zeros compare equal; no canonical representation is promised.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FinitePoint3(Point3);
impl FinitePoint3 {
    /// Checks a Cartesian point without rounding or changing its coordinates.
    pub fn try_new(point: Point3) -> Result<Self, SpatialError3> {
        if point.is_finite() {
            Ok(Self(point))
        } else {
            Err(SpatialError3::NonFinitePoint)
        }
    }
    /// Returns the stored Cartesian point.
    pub const fn get(self) -> Point3 {
        self.0
    }
}

/// A nonempty closed axis-aligned box with finite endpoints.
/// Zero width along any or all axes is allowed. Comparisons operate on the
/// stored coordinates, without tolerance or floating-point arithmetic.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb3 {
    lower: FinitePoint3,
    upper: FinitePoint3,
}
impl Aabb3 {
    /// Checks coordinatewise lower and upper bounds.
    pub fn try_new(lower: FinitePoint3, upper: FinitePoint3) -> Result<Self, SpatialError3> {
        let (a, b) = (lower.0, upper.0);
        if a.x > b.x || a.y > b.y || a.z > b.z {
            Err(SpatialError3::InvertedBounds)
        } else {
            Ok(Self { lower, upper })
        }
    }
    /// Creates the smallest box containing two checked points.
    pub fn from_points(a: FinitePoint3, b: FinitePoint3) -> Self {
        let (a, b) = (a.0, b.0);
        Self {
            lower: FinitePoint3(Point3::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z))),
            upper: FinitePoint3(Point3::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z))),
        }
    }
    /// Returns the lower corner.
    pub const fn lower(self) -> FinitePoint3 {
        self.lower
    }
    /// Returns the upper corner.
    pub const fn upper(self) -> FinitePoint3 {
        self.upper
    }
    /// Tests membership in the closed box.
    pub fn contains(self, point: FinitePoint3) -> bool {
        let (a, b, p) = (self.lower.0, self.upper.0, point.0);
        a.x <= p.x && p.x <= b.x && a.y <= p.y && p.y <= b.y && a.z <= p.z && p.z <= b.z
    }
    /// Returns the smallest closed box containing both boxes.
    pub fn union(self, other: Self) -> Self {
        let a = Self::from_points(self.lower, other.lower).lower;
        let b = Self::from_points(self.upper, other.upper).upper;
        Self { lower: a, upper: b }
    }
    /// Returns the closed intersection, including face, edge and point contacts.
    pub fn intersection(self, other: Self) -> Option<Self> {
        let a = Self::from_points(self.lower, other.lower).upper;
        let b = Self::from_points(self.upper, other.upper).lower;
        Self::try_new(a, b).ok()
    }
}

/// A straight finite-endpoint segment with distinct stored endpoints.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment3 {
    start: FinitePoint3,
    end: FinitePoint3,
}
impl Segment3 {
    /// Rejects equal endpoints, including differences only in zero signs.
    pub fn try_new(start: FinitePoint3, end: FinitePoint3) -> Result<Self, SpatialError3> {
        if start == end {
            Err(SpatialError3::DegenerateSegment)
        } else {
            Ok(Self { start, end })
        }
    }
    /// Returns the first endpoint.
    pub const fn start(self) -> FinitePoint3 {
        self.start
    }
    /// Returns the second endpoint.
    pub const fn end(self) -> FinitePoint3 {
        self.end
    }
    /// Returns the exact coordinatewise bounds of the stored endpoints.
    pub fn bounds(self) -> Aabb3 {
        Aabb3::from_points(self.start, self.end)
    }
}

/// A failure in a rounded spatial evaluation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpatialEvaluationError3 {
    /// A line parameter was NaN or infinite.
    NonFiniteParameter,
    /// An intermediate or final result was NaN or infinite.
    NonFiniteResult,
}
impl fmt::Display for SpatialEvaluationError3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteParameter => f.write_str("parameter must be finite"),
            Self::NonFiniteResult => f.write_str("spatial evaluation produced a nonfinite value"),
        }
    }
}
impl std::error::Error for SpatialEvaluationError3 {}

/// An infinite line represented by a finite origin and an approximately unit direction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Line3 {
    origin: FinitePoint3,
    direction: UnitDirection3,
}
impl Line3 {
    /// Creates a line from already validated components.
    pub const fn new(origin: FinitePoint3, direction: UnitDirection3) -> Self {
        Self { origin, direction }
    }
    /// Returns the origin.
    pub const fn origin(self) -> FinitePoint3 {
        self.origin
    }
    /// Returns the approximately unit direction.
    pub const fn direction(self) -> UnitDirection3 {
        self.direction
    }
    /// Evaluates `origin + direction * parameter` with separate rounded
    /// products and sums, checking all intermediate components for finiteness.
    /// The parameter is only approximately distance. Rounding and underflow
    /// occur; failure is possible even if the exact result is finite.
    pub fn try_evaluate(self, parameter: f64) -> Result<FinitePoint3, SpatialEvaluationError3> {
        if !parameter.is_finite() {
            return Err(SpatialEvaluationError3::NonFiniteParameter);
        }
        let d = self.direction.as_vector();
        let v = d * parameter;
        if !v.is_finite() {
            return Err(SpatialEvaluationError3::NonFiniteResult);
        }
        FinitePoint3::try_new(self.origin.0 + v)
            .map_err(|_| SpatialEvaluationError3::NonFiniteResult)
    }
}

/// An infinite plane represented by a finite origin and approximately unit normal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plane3 {
    origin: FinitePoint3,
    normal: UnitDirection3,
}
impl Plane3 {
    /// Creates a plane from already validated components.
    pub const fn new(origin: FinitePoint3, normal: UnitDirection3) -> Self {
        Self { origin, normal }
    }
    /// Returns a point on the represented plane.
    pub const fn origin(self) -> FinitePoint3 {
        self.origin
    }
    /// Returns the approximately unit normal.
    pub const fn normal(self) -> UnitDirection3 {
        self.normal
    }
    /// Computes `(point - origin) dot normal` with separate rounded component
    /// subtractions, products, and left-associated sums. Every intermediate is
    /// checked for finiteness. An error can occur when the exact offset is finite.
    /// A finite result may underflow or have the wrong sign: it is neither an
    /// exact distance nor a reliable incidence or side predicate.
    pub fn try_signed_offset(self, point: FinitePoint3) -> Result<f64, SpatialEvaluationError3> {
        let d = point.0 - self.origin.0;
        if !d.is_finite() {
            return Err(SpatialEvaluationError3::NonFiniteResult);
        }
        let n = self.normal.as_vector();
        let (x, y, z) = (d.x * n.x, d.y * n.y, d.z * n.z);
        if !x.is_finite() || !y.is_finite() || !z.is_finite() {
            return Err(SpatialEvaluationError3::NonFiniteResult);
        }
        let xy = x + y;
        if !xy.is_finite() {
            return Err(SpatialEvaluationError3::NonFiniteResult);
        }
        let result = xy + z;
        if result.is_finite() {
            Ok(result)
        } else {
            Err(SpatialEvaluationError3::NonFiniteResult)
        }
    }
}
