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

/// A closed axis-aligned box with finite coordinates in a caller-owned frame.
///
/// The lower and upper coordinates may coincide on either axis. All operations
/// use comparisons and select existing coordinates; no rounded arithmetic,
/// tolerance, transformation, or implicit enlargement is performed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb2 {
    min: Point2,
    max: Point2,
}

impl Aabb2 {
    /// Creates a closed box, rejecting a lower coordinate above its upper coordinate.
    pub fn try_new(min: Point2, max: Point2) -> Result<Self, Aabb2Error> {
        if min.x > max.x || min.y > max.y {
            return Err(Aabb2Error::InvertedBounds);
        }
        Ok(Self { min, max })
    }

    /// Returns the smallest closed box containing the points, or `None` for no points.
    pub fn from_points(points: impl IntoIterator<Item = Point2>) -> Option<Self> {
        let mut points = points.into_iter();
        let first = points.next()?;
        let mut result = Self {
            min: first,
            max: first,
        };
        for point in points {
            result.min.x = result.min.x.min(point.x);
            result.min.y = result.min.y.min(point.y);
            result.max.x = result.max.x.max(point.x);
            result.max.y = result.max.y.max(point.y);
        }
        Some(result)
    }

    /// Returns the lower corner.
    pub const fn min(self) -> Point2 {
        self.min
    }

    /// Returns the upper corner.
    pub const fn max(self) -> Point2 {
        self.max
    }

    /// Returns whether the point lies inside or on the boundary.
    pub fn contains(self, point: Point2) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    /// Returns the smallest closed box containing both boxes.
    pub fn union(self, other: Self) -> Self {
        Self {
            min: Point2 {
                x: self.min.x.min(other.min.x),
                y: self.min.y.min(other.min.y),
            },
            max: Point2 {
                x: self.max.x.max(other.max.x),
                y: self.max.y.max(other.max.y),
            },
        }
    }

    /// Returns the shared closed box, including point or line contact, if any.
    pub fn intersection(self, other: Self) -> Option<Self> {
        let min = Point2 {
            x: self.min.x.max(other.min.x),
            y: self.min.y.max(other.min.y),
        };
        let max = Point2 {
            x: self.max.x.min(other.max.x),
            y: self.max.y.min(other.max.y),
        };
        Self::try_new(min, max).ok()
    }
}

/// A failure while constructing an [`Aabb2`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Aabb2Error {
    /// A lower coordinate is greater than the corresponding upper coordinate.
    InvertedBounds,
}

impl fmt::Display for Aabb2Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvertedBounds => {
                formatter.write_str("lower bounds must not exceed upper bounds")
            }
        }
    }
}

impl std::error::Error for Aabb2Error {}

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

/// An exact parameter on a finite directed input segment.
///
/// Values are produced by supported intersection witnesses and can be ordered
/// on the same directed segment. They are not general rational numbers.
#[derive(Clone, Copy)]
pub struct SegmentParameter2 {
    first: Segment2,
    numerator: PredicateInteger,
    denominator: PredicateInteger,
}

impl SegmentParameter2 {
    /// Returns the directed source segment whose parameter defines order.
    pub const fn segment(&self) -> Segment2 {
        self.first
    }

    /// Compares exact parameters on the same directed first segment.
    ///
    /// Returns `None` when the first segments differ, including reversed traversal.
    /// Distinct rational representations of the same parameter compare equal.
    pub fn compare_on_first(&self, other: &Self) -> Option<Ordering> {
        if self.first != other.first {
            return None;
        }
        let left = multiply_predicate_magnitudes(self.numerator, other.denominator);
        let right = multiply_predicate_magnitudes(other.numerator, self.denominator);
        Some(left.iter().rev().cmp(right.iter().rev()))
    }
}

/// An exact proper-crossing witness for two finite, nonzero input segments.
///
/// The witness retains both original segments and the exact rational parameter
/// on the first directed segment. It is limited to intersections of raw
/// binary64 endpoints; it is not a general rational point or a rounded location.
#[derive(Clone, Copy)]
pub struct ProperCrossing2 {
    second: Segment2,
    parameter: SegmentParameter2,
}

impl ProperCrossing2 {
    /// Returns the first directed source segment, whose parameter defines order.
    pub const fn first_segment(&self) -> Segment2 {
        self.parameter.first
    }

    /// Returns the second source segment.
    pub const fn second_segment(&self) -> Segment2 {
        self.second
    }

    /// Returns the exact parameter for comparison with other supported events.
    pub const fn parameter(&self) -> SegmentParameter2 {
        self.parameter
    }

    /// Compares exact parameters on the same directed first segment.
    ///
    /// Returns `None` when the first segments differ, including reversed traversal.
    /// Distinct rational representations of the same parameter compare equal.
    pub fn compare_on_first(&self, other: &Self) -> Option<Ordering> {
        self.parameter.compare_on_first(&other.parameter)
    }
}

/// Retains the exact rational parameter of a proper crossing, if one exists.
///
/// This returns `None` for disjoint segments, endpoint touches, and collinear
/// overlaps. Use [`intersect_segments`] to distinguish those cases. The witness
/// supports exact ordering on the same directed first segment; it does not
/// construct a binary64 intersection point or accept previously constructed
/// rational points as inputs.
pub fn proper_crossing_witness(first: Segment2, second: Segment2) -> Option<ProperCrossing2> {
    if intersect_segments(first, second)
        != SegmentIntersection::Point(SegmentPointEvent::ProperCrossing)
    {
        return None;
    }
    let numerator = exact_cross(second.start, first.start, second.end, second.start);
    let denominator = exact_cross(first.end, first.start, second.end, second.start);
    let (numerator, denominator) = if denominator.negative {
        (numerator.negate(), denominator.negate())
    } else {
        (numerator, denominator)
    };
    debug_assert!(numerator.sign() == Ordering::Greater);
    debug_assert!(denominator.sign() == Ordering::Greater);
    debug_assert!(numerator.magnitude_cmp(denominator) == Ordering::Less);
    Some(ProperCrossing2 {
        second,
        parameter: SegmentParameter2 {
            first,
            numerator,
            denominator,
        },
    })
}

/// An endpoint-touch witness containing the original exact binary64 endpoint.
#[derive(Clone, Copy)]
pub struct EndpointTouch2 {
    second: Segment2,
    point: Point2,
    parameter: SegmentParameter2,
}

impl EndpointTouch2 {
    /// Returns the first directed source segment.
    pub const fn first_segment(&self) -> Segment2 {
        self.parameter.first
    }
    /// Returns the second source segment.
    pub const fn second_segment(&self) -> Segment2 {
        self.second
    }
    /// Returns an original input endpoint at the contact, with no construction rounding.
    pub const fn point(&self) -> Point2 {
        self.point
    }
    /// Returns the exact parameter on the first directed segment.
    pub const fn parameter(&self) -> SegmentParameter2 {
        self.parameter
    }
}

/// Retains the source endpoint and exact first-segment parameter for a single-point touch.
///
/// This includes collinear segments meeting at one endpoint. It returns `None`
/// for proper crossings, overlaps, and disjoint segments; use
/// [`intersect_segments`] to distinguish those cases.
pub fn endpoint_touch_witness(first: Segment2, second: Segment2) -> Option<EndpointTouch2> {
    if intersect_segments(first, second)
        != SegmentIntersection::Point(SegmentPointEvent::BoundaryTouch)
    {
        return None;
    }
    let point = [first.start, first.end, second.start, second.end]
        .into_iter()
        .find(|&p| {
            orient2d(first.start, first.end, p) == Orientation2::Collinear
                && on_segment(first.start, first.end, p)
                && orient2d(second.start, second.end, p) == Orientation2::Collinear
                && on_segment(second.start, second.end, p)
        })
        .expect("a single-point boundary touch has a source endpoint");
    let (a, b, p) = if first.start.x != first.end.x {
        (first.start.x, first.end.x, point.x)
    } else {
        (first.start.y, first.end.y, point.y)
    };
    let numerator = widen_coordinate(
        CoordinateInteger::from_finite_f64(p).subtract(CoordinateInteger::from_finite_f64(a)),
    );
    let denominator = widen_coordinate(
        CoordinateInteger::from_finite_f64(b).subtract(CoordinateInteger::from_finite_f64(a)),
    );
    let (numerator, denominator) = if denominator.negative {
        (numerator.negate(), denominator.negate())
    } else {
        (numerator, denominator)
    };
    debug_assert!(numerator.sign() != Ordering::Less);
    debug_assert!(denominator.sign() == Ordering::Greater);
    debug_assert!(numerator.magnitude_cmp(denominator) != Ordering::Greater);
    Some(EndpointTouch2 {
        second,
        point,
        parameter: SegmentParameter2 {
            first,
            numerator,
            denominator,
        },
    })
}

fn widen_coordinate(value: CoordinateInteger) -> PredicateInteger {
    let mut limbs = [0; PREDICATE_LIMBS];
    limbs[..COORDINATE_LIMBS].copy_from_slice(&value.limbs);
    PredicateInteger {
        limbs,
        negative: value.negative,
    }
}

// Computes (a - b) × (c - d) without rounded coordinate subtraction.
fn exact_cross(a: Point2, b: Point2, c: Point2, d: Point2) -> PredicateInteger {
    let x1 =
        CoordinateInteger::from_finite_f64(a.x).subtract(CoordinateInteger::from_finite_f64(b.x));
    let y1 =
        CoordinateInteger::from_finite_f64(a.y).subtract(CoordinateInteger::from_finite_f64(b.y));
    let x2 =
        CoordinateInteger::from_finite_f64(c.x).subtract(CoordinateInteger::from_finite_f64(d.x));
    let y2 =
        CoordinateInteger::from_finite_f64(c.y).subtract(CoordinateInteger::from_finite_f64(d.y));
    x1.multiply(y2).subtract(y1.multiply(x2))
}

// Each raw-input determinant is < 2^4199; multiplying two needs < 8398 bits.
// 132 limbs supply 8448 bits, including for unnormalized equivalent fractions.
fn multiply_predicate_magnitudes(left: PredicateInteger, right: PredicateInteger) -> [u64; 132] {
    let mut product = [0_u64; 132];
    for i in 0..PREDICATE_LIMBS {
        if left.limbs[i] == 0 {
            continue;
        }
        let mut carry = 0_u128;
        for j in 0..PREDICATE_LIMBS {
            if right.limbs[j] == 0 && carry == 0 {
                continue;
            }
            let index = i + j;
            let sum = u128::from(left.limbs[i]) * u128::from(right.limbs[j])
                + u128::from(product[index])
                + carry;
            product[index] = sum as u64;
            carry = sum >> 64;
        }
        let mut index = i + PREDICATE_LIMBS;
        while carry != 0 {
            debug_assert!(index < product.len());
            let sum = u128::from(product[index]) + carry;
            product[index] = sum as u64;
            carry = sum >> 64;
            index += 1;
        }
    }
    product
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
