// SPDX-License-Identifier: Apache-2.0

//! Checked finite planar primitives and exact straight-segment topology events.
//!
//! This module forms an initial 2D topology slice. Points and segments reject
//! nonfinite coordinates; [`orient2d`] returns the exact orientation of the
//! represented binary64 coordinates; [`intersect_segments`] classifies straight
//! segment intersections without constructing rounded intersection points; and
//! [`SimpleWire2`] admits a simple closed polygonal wire.
//!
//! Coordinates are in one caller-owned Cartesian frame and units. No tolerance,
//! snapping, or hidden coordinate scaling is applied. The exact predicate uses
//! fixed-width integer arithmetic over the exact dyadic values represented by
//! finite `f64` inputs. This does not make later geometric constructions exact.

use std::cmp::Ordering;
use std::fmt;

const COORDINATE_LIMBS: usize = 33;
const PREDICATE_LIMBS: usize = 66;

type CoordinateInteger = SignedMagnitude<COORDINATE_LIMBS>;
type PredicateInteger = SignedMagnitude<PREDICATE_LIMBS>;

/// A finite point in a caller-owned two-dimensional Cartesian frame.
///
/// Equality compares the exact stored `f64` values. Signed zeros compare equal;
/// no approximate equality or tolerance is used.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point2 {
    x: f64,
    y: f64,
}

impl Point2 {
    /// Creates a finite point, rejecting NaN and either infinity.
    pub fn try_new(x: f64, y: f64) -> Result<Self, Point2Error> {
        if !x.is_finite() || !y.is_finite() {
            return Err(Point2Error::NonFiniteCoordinate);
        }
        Ok(Self { x, y })
    }

    /// Returns the x coordinate in the caller's chosen units.
    pub const fn x(self) -> f64 {
        self.x
    }

    /// Returns the y coordinate in the caller's chosen units.
    pub const fn y(self) -> f64 {
        self.y
    }
}

/// A failure while constructing a checked [`Point2`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Point2Error {
    /// At least one coordinate was NaN or infinite.
    NonFiniteCoordinate,
}

impl fmt::Display for Point2Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteCoordinate => formatter.write_str("point coordinates must be finite"),
        }
    }
}

impl std::error::Error for Point2Error {}

/// A nonzero straight segment with finite endpoints.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment2 {
    start: Point2,
    end: Point2,
}

impl Segment2 {
    /// Creates a segment, rejecting endpoints that compare equal.
    pub fn try_new(start: Point2, end: Point2) -> Result<Self, Segment2Error> {
        if start == end {
            return Err(Segment2Error::ZeroLength);
        }
        Ok(Self { start, end })
    }

    /// Returns the segment's start endpoint.
    pub const fn start(self) -> Point2 {
        self.start
    }

    /// Returns the segment's end endpoint.
    pub const fn end(self) -> Point2 {
        self.end
    }
}

/// A failure while constructing a checked [`Segment2`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Segment2Error {
    /// The two finite endpoints have identical coordinates.
    ZeroLength,
}

impl fmt::Display for Segment2Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroLength => formatter.write_str("a segment must have distinct endpoints"),
        }
    }
}

impl std::error::Error for Segment2Error {}

/// The exact orientation of three finite points in a right-handed 2D frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Orientation2 {
    /// The points turn clockwise from the first-to-second direction.
    Clockwise,
    /// The points are exactly collinear as represented by their `f64` values.
    Collinear,
    /// The points turn counterclockwise from the first-to-second direction.
    Counterclockwise,
}

/// Returns the exact orientation sign for the represented binary64 coordinates.
///
/// The determinant is `(b.x-a.x)*(c.y-a.y) - (b.y-a.y)*(c.x-a.x)`. The sign is
/// computed without floating-point subtraction, product overflow, or underflow.
/// No intersection coordinate or approximate tolerance is involved.
pub fn orient2d(a: Point2, b: Point2, c: Point2) -> Orientation2 {
    let ax = CoordinateInteger::from_finite_f64(a.x);
    let ay = CoordinateInteger::from_finite_f64(a.y);
    let bx = CoordinateInteger::from_finite_f64(b.x);
    let by = CoordinateInteger::from_finite_f64(b.y);
    let cx = CoordinateInteger::from_finite_f64(c.x);
    let cy = CoordinateInteger::from_finite_f64(c.y);

    let ab_x = bx.subtract(ax);
    let ab_y = by.subtract(ay);
    let ac_x = cx.subtract(ax);
    let ac_y = cy.subtract(ay);
    let determinant = ab_x.multiply(ac_y).subtract(ab_y.multiply(ac_x));

    match determinant.sign() {
        Ordering::Less => Orientation2::Clockwise,
        Ordering::Equal => Orientation2::Collinear,
        Ordering::Greater => Orientation2::Counterclockwise,
    }
}

/// Whether a single-point segment intersection is transverse or touches a boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SegmentPointEvent {
    /// The interiors cross with opposite exact orientation signs.
    ProperCrossing,
    /// An endpoint lies on the other segment, including a shared endpoint.
    BoundaryTouch,
}

/// The exact set-theoretic intersection event for two nonzero finite segments.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SegmentIntersection {
    /// The closed segments have no point in common.
    Disjoint,
    /// The closed segments share exactly one point.
    Point(SegmentPointEvent),
    /// The closed segments share a collinear interval of positive length.
    Overlap,
}

/// Classifies the exact intersection of two checked straight segments.
///
/// Results distinguish disjoint segments, one-point events, and collinear
/// positive-length overlaps. The function deliberately does not construct or
/// round a new intersection coordinate; proper crossing coordinates require a
/// separate construction policy.
pub fn intersect_segments(first: Segment2, second: Segment2) -> SegmentIntersection {
    let a = first.start;
    let b = first.end;
    let c = second.start;
    let d = second.end;

    let abc = orient2d(a, b, c);
    let abd = orient2d(a, b, d);
    let cda = orient2d(c, d, a);
    let cdb = orient2d(c, d, b);

    if abc != Orientation2::Collinear
        && abd != Orientation2::Collinear
        && cda != Orientation2::Collinear
        && cdb != Orientation2::Collinear
        && opposite(abc, abd)
        && opposite(cda, cdb)
    {
        return SegmentIntersection::Point(SegmentPointEvent::ProperCrossing);
    }

    if abc == Orientation2::Collinear
        && abd == Orientation2::Collinear
        && cda == Orientation2::Collinear
        && cdb == Orientation2::Collinear
    {
        return collinear_intersection(first, second);
    }

    if (abc == Orientation2::Collinear && on_segment(a, b, c))
        || (abd == Orientation2::Collinear && on_segment(a, b, d))
        || (cda == Orientation2::Collinear && on_segment(c, d, a))
        || (cdb == Orientation2::Collinear && on_segment(c, d, b))
    {
        return SegmentIntersection::Point(SegmentPointEvent::BoundaryTouch);
    }

    SegmentIntersection::Disjoint
}

/// A simple closed polygonal wire in the plane.
///
/// Vertices are stored in traversal order. The closing edge from the final
/// vertex to the first is implicit, so callers must not repeat the first vertex
/// at the end. A valid wire has at least three vertices, nonzero edges, and no
/// intersection between nonadjacent edges; adjacent edges may share only their
/// common endpoint. Validation is exact for the represented finite coordinates
/// and takes O(n²) segment-pair checks.
#[derive(Clone, Debug, PartialEq)]
pub struct SimpleWire2 {
    vertices: Vec<Point2>,
}

impl SimpleWire2 {
    /// Validates vertices and constructs a simple closed wire.
    pub fn try_new(vertices: impl Into<Vec<Point2>>) -> Result<Self, Wire2Error> {
        let vertices = vertices.into();
        if vertices.len() < 3 {
            return Err(Wire2Error::TooFewVertices);
        }

        let edge_count = vertices.len();
        for edge in 0..edge_count {
            let next = (edge + 1) % edge_count;
            if vertices[edge] == vertices[next] {
                return Err(Wire2Error::ZeroLengthEdge { edge });
            }
        }

        for first_edge in 0..edge_count {
            let first = segment_at(&vertices, first_edge);
            for second_edge in (first_edge + 1)..edge_count {
                let second = segment_at(&vertices, second_edge);
                let event = intersect_segments(first, second);
                let adjacent = second_edge == first_edge + 1
                    || (first_edge == 0 && second_edge + 1 == edge_count);

                let valid_adjacent_event = adjacent
                    && event == SegmentIntersection::Point(SegmentPointEvent::BoundaryTouch);
                if (!adjacent && event != SegmentIntersection::Disjoint)
                    || (adjacent && !valid_adjacent_event)
                {
                    return Err(Wire2Error::IntersectingEdges {
                        first_edge,
                        second_edge,
                    });
                }
            }
        }

        Ok(Self { vertices })
    }

    /// Returns the vertices in traversal order; the closing vertex is implicit.
    pub fn vertices(&self) -> &[Point2] {
        &self.vertices
    }

    /// Returns the number of vertices and edges in the closed wire.
    pub fn len(&self) -> usize {
        self.vertices.len()
    }

    /// Returns whether the wire has no vertices (always `false` for a valid wire).
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }
}

/// A failure while validating a [`SimpleWire2`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Wire2Error {
    /// A closed wire requires at least three vertices.
    TooFewVertices,
    /// An edge's endpoints are equal, including the implicit closing edge.
    ZeroLengthEdge {
        /// The index of the edge that starts at the reported vertex.
        edge: usize,
    },
    /// Two edges intersect beyond the single common endpoint allowed for adjacent edges.
    IntersectingEdges {
        /// The lower edge index in traversal order.
        first_edge: usize,
        /// The higher edge index in traversal order.
        second_edge: usize,
    },
}

impl fmt::Display for Wire2Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewVertices => {
                formatter.write_str("a closed wire needs at least three vertices")
            }
            Self::ZeroLengthEdge { edge } => write!(formatter, "wire edge {edge} has zero length"),
            Self::IntersectingEdges {
                first_edge,
                second_edge,
            } => write!(
                formatter,
                "wire edges {first_edge} and {second_edge} intersect"
            ),
        }
    }
}

impl std::error::Error for Wire2Error {}

fn segment_at(vertices: &[Point2], edge: usize) -> Segment2 {
    Segment2 {
        start: vertices[edge],
        end: vertices[(edge + 1) % vertices.len()],
    }
}

fn opposite(first: Orientation2, second: Orientation2) -> bool {
    matches!(
        (first, second),
        (Orientation2::Clockwise, Orientation2::Counterclockwise)
            | (Orientation2::Counterclockwise, Orientation2::Clockwise)
    )
}

fn on_segment(start: Point2, end: Point2, point: Point2) -> bool {
    point.x >= start.x.min(end.x)
        && point.x <= start.x.max(end.x)
        && point.y >= start.y.min(end.y)
        && point.y <= start.y.max(end.y)
}

fn collinear_intersection(first: Segment2, second: Segment2) -> SegmentIntersection {
    let (a_min, a_max, b_min, b_max) = if first.start.x != first.end.x {
        (
            first.start.x.min(first.end.x),
            first.start.x.max(first.end.x),
            second.start.x.min(second.end.x),
            second.start.x.max(second.end.x),
        )
    } else {
        (
            first.start.y.min(first.end.y),
            first.start.y.max(first.end.y),
            second.start.y.min(second.end.y),
            second.start.y.max(second.end.y),
        )
    };
    let low = a_min.max(b_min);
    let high = a_max.min(b_max);
    match low.partial_cmp(&high) {
        Some(Ordering::Less) => SegmentIntersection::Overlap,
        Some(Ordering::Equal) => SegmentIntersection::Point(SegmentPointEvent::BoundaryTouch),
        Some(Ordering::Greater) | None => SegmentIntersection::Disjoint,
    }
}

#[derive(Clone, Copy)]
struct SignedMagnitude<const N: usize> {
    limbs: [u64; N],
    negative: bool,
}

impl<const N: usize> SignedMagnitude<N> {
    const fn zero() -> Self {
        Self {
            limbs: [0; N],
            negative: false,
        }
    }

    fn is_zero(self) -> bool {
        self.limbs.iter().all(|limb| *limb == 0)
    }

    fn canonical(mut self) -> Self {
        if self.is_zero() {
            self.negative = false;
        }
        self
    }

    fn magnitude_cmp(self, other: Self) -> Ordering {
        self.limbs.iter().rev().cmp(other.limbs.iter().rev())
    }

    fn add_magnitudes(self, other: Self) -> Self {
        let mut result = Self::zero();
        let mut carry = false;
        for index in 0..N {
            let (partial, first_carry) = self.limbs[index].overflowing_add(other.limbs[index]);
            let (sum, second_carry) = partial.overflowing_add(u64::from(carry));
            result.limbs[index] = sum;
            carry = first_carry || second_carry;
        }
        debug_assert!(!carry, "fixed-width exact coordinate addition overflowed");
        result
    }

    fn subtract_magnitudes(larger: Self, smaller: Self) -> Self {
        debug_assert!(larger.magnitude_cmp(smaller) != Ordering::Less);
        let mut result = Self::zero();
        let mut borrow = false;
        for index in 0..N {
            let (partial, first_borrow) = larger.limbs[index].overflowing_sub(smaller.limbs[index]);
            let (difference, second_borrow) = partial.overflowing_sub(u64::from(borrow));
            result.limbs[index] = difference;
            borrow = first_borrow || second_borrow;
        }
        debug_assert!(!borrow, "magnitude subtraction precondition violated");
        result
    }

    fn add(self, other: Self) -> Self {
        if self.negative == other.negative {
            let mut sum = self.add_magnitudes(other);
            sum.negative = self.negative;
            return sum.canonical();
        }

        match self.magnitude_cmp(other) {
            Ordering::Greater => {
                let mut difference = Self::subtract_magnitudes(self, other);
                difference.negative = self.negative;
                difference.canonical()
            }
            Ordering::Less => {
                let mut difference = Self::subtract_magnitudes(other, self);
                difference.negative = other.negative;
                difference.canonical()
            }
            Ordering::Equal => Self::zero(),
        }
    }

    fn negate(mut self) -> Self {
        if !self.is_zero() {
            self.negative = !self.negative;
        }
        self
    }

    fn subtract(self, other: Self) -> Self {
        self.add(other.negate())
    }

    fn sign(self) -> Ordering {
        if self.is_zero() {
            Ordering::Equal
        } else if self.negative {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }
}

impl CoordinateInteger {
    /// Embeds every finite binary64 value exactly as an integer multiple of 2^-1074.
    fn from_finite_f64(value: f64) -> Self {
        debug_assert!(value.is_finite());
        let bits = value.to_bits();
        let exponent = ((bits >> 52) & 0x7ff) as usize;
        let fraction = bits & ((1_u64 << 52) - 1);
        let significand = if exponent == 0 {
            fraction
        } else {
            fraction | (1_u64 << 52)
        };
        if significand == 0 {
            return Self::zero();
        }

        let shift = if exponent == 0 { 0 } else { exponent - 1 };
        let word = shift / 64;
        let offset = shift % 64;
        let mut limbs = [0; COORDINATE_LIMBS];
        limbs[word] = significand << offset;
        if offset != 0 {
            limbs[word + 1] = significand >> (64 - offset);
        }
        Self {
            limbs,
            negative: bits >> 63 != 0,
        }
    }

    fn multiply(self, other: Self) -> PredicateInteger {
        let mut limbs = [0; PREDICATE_LIMBS];
        let Some(first_left) = self.limbs.iter().position(|limb| *limb != 0) else {
            return PredicateInteger::zero();
        };
        let Some(last_left) = self.limbs.iter().rposition(|limb| *limb != 0) else {
            return PredicateInteger::zero();
        };
        let Some(first_right) = other.limbs.iter().position(|limb| *limb != 0) else {
            return PredicateInteger::zero();
        };
        let Some(last_right) = other.limbs.iter().rposition(|limb| *limb != 0) else {
            return PredicateInteger::zero();
        };

        for left_index in first_left..=last_left {
            if self.limbs[left_index] == 0 {
                continue;
            }
            let mut carry = 0_u128;
            for right_index in first_right..=last_right {
                if other.limbs[right_index] == 0 && carry == 0 {
                    continue;
                }
                let index = left_index + right_index;
                let sum = u128::from(self.limbs[left_index]) * u128::from(other.limbs[right_index])
                    + u128::from(limbs[index])
                    + carry;
                limbs[index] = sum as u64;
                carry = sum >> 64;
            }

            let mut index = left_index + last_right + 1;
            while carry != 0 {
                debug_assert!(index < PREDICATE_LIMBS);
                let sum = u128::from(limbs[index]) + carry;
                limbs[index] = sum as u64;
                carry = sum >> 64;
                index += 1;
            }
        }
        PredicateInteger {
            limbs,
            negative: self.negative != other.negative,
        }
        .canonical()
    }
}
