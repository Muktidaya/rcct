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
type ConstructionInteger = SignedMagnitude<99>;
type ComparisonInteger = SignedMagnitude<165>;
type OrientationInteger = SignedMagnitude<330>;

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

    /// Returns whether the exact finite point lies on this closed segment.
    ///
    /// Orientation is decided exactly for the represented binary64 inputs;
    /// endpoints are included, with no tolerance or rounding of coordinates.
    pub fn contains(self, point: Point2) -> bool {
        orient2d(self.start, self.end, point) == Orientation2::Collinear
            && on_segment(self.start, self.end, point)
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

    /// Returns the tight binary64 enclosure of this exact parameter in `[0, 1]`.
    ///
    /// If exactly representable, both bounds are equal. Otherwise they are
    /// adjacent binary64 values enclosing the rational parameter. Integer
    /// comparisons decide the bounds without rounded floating-point division.
    pub fn enclosure(&self) -> ParameterEnclosure2 {
        let target = shift_predicate_by_1074(self.numerator);
        let compare = |bits: u64| {
            let candidate = CoordinateInteger::from_finite_f64(f64::from_bits(bits));
            let product = multiply_predicate_coordinate(self.denominator, candidate);
            target.iter().rev().cmp(product.iter().rev())
        };
        let (mut low, mut high) = (0_u64, 1.0_f64.to_bits());
        while low < high {
            let middle = low + (high - low).div_ceil(2);
            if compare(middle) != Ordering::Less {
                low = middle;
            } else {
                high = middle - 1;
            }
        }
        let lower = f64::from_bits(low);
        let upper = if compare(low) == Ordering::Equal {
            lower
        } else {
            f64::from_bits(low + 1)
        };
        ParameterEnclosure2 { lower, upper }
    }
}

/// A nonempty parameter interval on an original finite directed segment.
///
/// Bounds retain exact raw-input parameters. This is a view of one original
/// support segment, not a new rational segment usable in intersection APIs.
#[derive(Clone)]
pub struct SegmentSpan2 {
    start: SegmentParameter2,
    end: SegmentParameter2,
}

impl SegmentSpan2 {
    /// Returns the whole original nonzero input segment as a parameter span.
    pub fn whole(segment: Segment2) -> Self {
        Self {
            start: parameter_for_input_point(segment, segment.start),
            end: parameter_for_input_point(segment, segment.end),
        }
    }

    /// Builds a nonempty span from ordered parameters of one directed segment.
    pub fn try_new(
        start: SegmentParameter2,
        end: SegmentParameter2,
    ) -> Result<Self, SpanSplitError> {
        match start.compare_on_first(&end) {
            None => Err(SpanSplitError::DifferentSource),
            Some(Ordering::Less) => Ok(Self { start, end }),
            Some(_) => Err(SpanSplitError::EmptyOrReversed),
        }
    }

    /// Returns the original directed support segment.
    pub const fn segment(&self) -> Segment2 {
        self.start.first
    }
    /// Returns the exact first boundary parameter.
    pub const fn start(&self) -> SegmentParameter2 {
        self.start
    }
    /// Returns the exact last boundary parameter.
    pub const fn end(&self) -> SegmentParameter2 {
        self.end
    }

    /// Splits strictly inside this span using a parameter on the same source.
    ///
    /// This immutable operation returns two new spans. On any error, `self` and
    /// the budget are unchanged. A successful split consumes one split unit.
    /// It does not construct a new rational segment or mutate topology.
    pub fn split_at(
        &self,
        at: SegmentParameter2,
        budget: &mut SpanSplitBudget,
    ) -> Result<(Self, Self), SpanSplitError> {
        let lower = self
            .start
            .compare_on_first(&at)
            .ok_or(SpanSplitError::DifferentSource)?;
        let upper = at
            .compare_on_first(&self.end)
            .ok_or(SpanSplitError::DifferentSource)?;
        if lower != Ordering::Less || upper != Ordering::Less {
            return Err(SpanSplitError::NotInterior);
        }
        if budget.remaining == 0 {
            return Err(SpanSplitError::SplitLimitExceeded);
        }
        let left = Self {
            start: self.start,
            end: at,
        };
        let right = Self {
            start: at,
            end: self.end,
        };
        budget.remaining -= 1;
        Ok((left, right))
    }
}

/// A caller-chosen cap on successful span splits.
///
/// Each attempt has fixed-width comparisons, but this cap does not limit the
/// number of rejected calls, caller-owned storage, wall time, or allocations.
#[derive(Debug, Eq, PartialEq)]
pub struct SpanSplitBudget {
    remaining: u64,
}

impl SpanSplitBudget {
    /// Creates a budget for at most `limit` successful splits.
    pub const fn new(limit: u64) -> Self {
        Self { remaining: limit }
    }
    /// Returns the remaining successful split units.
    pub const fn remaining(&self) -> u64 {
        self.remaining
    }
}

/// A rejected span construction or split.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpanSplitError {
    /// Parameters refer to different directed input segments.
    DifferentSource,
    /// A span's start is not strictly before its end.
    EmptyOrReversed,
    /// A split parameter is at or outside the span boundaries.
    NotInterior,
    /// The caller's successful-split budget is exhausted.
    SplitLimitExceeded,
}

impl fmt::Display for SpanSplitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DifferentSource => f.write_str("parameters have different directed sources"),
            Self::EmptyOrReversed => f.write_str("span boundaries must be strictly ordered"),
            Self::NotInterior => f.write_str("split parameter must be strictly inside the span"),
            Self::SplitLimitExceeded => f.write_str("span split budget is exhausted"),
        }
    }
}

impl std::error::Error for SpanSplitError {}

/// A tight, inclusive binary64 enclosure of a supported exact segment parameter.
///
/// The bounds lie in `[0, 1]` and are identical or adjacent representable
/// values. The constructor is private so callers cannot manufacture a claim.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParameterEnclosure2 {
    lower: f64,
    upper: f64,
}

impl ParameterEnclosure2 {
    /// Returns the inclusive lower bound.
    pub const fn lower(self) -> f64 {
        self.lower
    }
    /// Returns the inclusive upper bound.
    pub const fn upper(self) -> f64 {
        self.upper
    }
}

/// Tight binary64 bounds for one exact coordinate of a supported crossing.
///
/// The bounds are finite and identical or adjacent representable values. An
/// exact mathematical zero is represented as positive zero at both bounds;
/// negative zero may occur as the upper bound of a negative subnormal interval.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CoordinateEnclosure2 {
    lower: f64,
    upper: f64,
}

impl CoordinateEnclosure2 {
    /// Returns the inclusive lower coordinate bound.
    pub const fn lower(self) -> f64 {
        self.lower
    }
    /// Returns the inclusive upper coordinate bound.
    pub const fn upper(self) -> f64 {
        self.upper
    }
}

/// Independent coordinate enclosures containing an exact planar crossing.
///
/// The rectangle may contain points other than the crossing; it is not an
/// exact point identity or a general interval arithmetic representation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrossingEnclosure2 {
    x: CoordinateEnclosure2,
    y: CoordinateEnclosure2,
}

impl CrossingEnclosure2 {
    /// Returns the x coordinate enclosure.
    pub const fn x(self) -> CoordinateEnclosure2 {
        self.x
    }
    /// Returns the y coordinate enclosure.
    pub const fn y(self) -> CoordinateEnclosure2 {
        self.y
    }
}

/// An exact point constructed from one proper crossing of raw finite inputs.
///
/// Comparisons work across independently constructed crossings without rounding
/// or fraction reduction. This type cannot be used as a new segment endpoint;
/// general rational inputs and repeated cuts are not yet supported.
#[derive(Clone)]
pub struct ExactCrossingPoint2 {
    x: ConstructionInteger,
    y: ConstructionInteger,
    denominator: PredicateInteger,
}

impl ExactCrossingPoint2 {
    /// Compares exact x coordinates across supported crossing constructions.
    pub fn compare_x(&self, other: &Self) -> Ordering {
        compare_construction_coordinates(self.x, self.denominator, other.x, other.denominator)
    }
    /// Compares exact y coordinates across supported crossing constructions.
    pub fn compare_y(&self, other: &Self) -> Ordering {
        compare_construction_coordinates(self.y, self.denominator, other.y, other.denominator)
    }
    /// Returns whether both exact coordinates are equal.
    pub fn same_point(&self, other: &Self) -> bool {
        self.compare_x(other) == Ordering::Equal && self.compare_y(other) == Ordering::Equal
    }
    /// Compares x first, then y, without rounding or fraction reduction.
    pub fn compare_lexicographically(&self, other: &Self) -> Ordering {
        self.compare_x(other).then_with(|| self.compare_y(other))
    }
    /// Compares this exact point to a finite input point, x first and then y.
    pub fn compare_to_input(&self, other: Point2) -> Ordering {
        let x = self.x.signed_cmp(signed_coordinate_product(
            self.denominator,
            CoordinateInteger::from_finite_f64(other.x),
        ));
        x.then_with(|| {
            self.y.signed_cmp(signed_coordinate_product(
                self.denominator,
                CoordinateInteger::from_finite_f64(other.y),
            ))
        })
    }
    /// Returns whether both exact coordinates equal a finite input point.
    pub fn same_as_input(&self, other: Point2) -> bool {
        self.compare_to_input(other) == Ordering::Equal
    }
    /// Returns tight binary64 coordinate enclosures for this exact point.
    pub fn enclosure(&self) -> CrossingEnclosure2 {
        CrossingEnclosure2 {
            x: enclose_crossing_coordinate(self.x, self.denominator),
            y: enclose_crossing_coordinate(self.y, self.denominator),
        }
    }
}

/// A raw input point or first-generation exact crossing for a constructed predicate.
#[derive(Clone, Copy)]
pub enum ExactPredicatePoint2<'a> {
    /// A finite binary64 input point, interpreted exactly as stored.
    Input(Point2),
    /// A proper crossing constructed directly from finite binary64 input segments.
    Crossing(&'a ExactCrossingPoint2),
}

/// A conservative budget for fixed-width constructed-point predicates.
///
/// Units count the maximum limb multiplications charged per operation, not wall
/// time, memory, additions, or caller-owned storage. Inputs are bounded to raw
/// points and first-generation crossings; no arbitrary rational input is accepted.
#[derive(Debug, Eq, PartialEq)]
pub struct ConstructedPredicateBudget {
    remaining: u64,
}

impl ConstructedPredicateBudget {
    /// Creates a budget with the given maximum charged limb multiplications.
    pub const fn new(limb_products: u64) -> Self {
        Self {
            remaining: limb_products,
        }
    }
    /// Returns the remaining charged limb multiplications.
    pub const fn remaining(&self) -> u64 {
        self.remaining
    }
    /// Returns the conservative fixed charge for one constructed orientation.
    pub const fn orientation_cost() -> u64 {
        8 * 99 * 66 + 2 * 165 * 165
    }
}

/// A constructed predicate could not begin within the supplied work budget.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstructedPredicateError {
    /// The remaining limb-product budget is below the operation's fixed charge.
    WorkLimitExceeded,
}

impl fmt::Display for ConstructedPredicateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WorkLimitExceeded => f.write_str("constructed predicate work limit exceeded"),
        }
    }
}

impl std::error::Error for ConstructedPredicateError {}

/// Returns the exact orientation of three supported raw or constructed points.
///
/// Each crossing must be first-generation from raw finite binary64 segments.
/// Results are exact for those retained constructions; no enclosure or rounded
/// point is substituted. The fixed charge is checked before arithmetic; on
/// budget failure inputs and budget are unchanged. This does not support
/// arbitrary rational inputs, repeated intersection constructions or topology.
pub fn orient_constructed2d(
    a: ExactPredicatePoint2<'_>,
    b: ExactPredicatePoint2<'_>,
    c: ExactPredicatePoint2<'_>,
    budget: &mut ConstructedPredicateBudget,
) -> Result<Orientation2, ConstructedPredicateError> {
    let cost = ConstructedPredicateBudget::orientation_cost();
    if budget.remaining < cost {
        return Err(ConstructedPredicateError::WorkLimitExceeded);
    }
    let a = predicate_coordinates(a);
    let b = predicate_coordinates(b);
    let c = predicate_coordinates(c);
    let dx1 = multiply_construction_predicate(b.x, a.denominator)
        .subtract(multiply_construction_predicate(a.x, b.denominator));
    let dy1 = multiply_construction_predicate(b.y, a.denominator)
        .subtract(multiply_construction_predicate(a.y, b.denominator));
    let dx2 = multiply_construction_predicate(c.x, a.denominator)
        .subtract(multiply_construction_predicate(a.x, c.denominator));
    let dy2 = multiply_construction_predicate(c.y, a.denominator)
        .subtract(multiply_construction_predicate(a.y, c.denominator));
    let determinant =
        multiply_comparison_integers(dx1, dy2).subtract(multiply_comparison_integers(dy1, dx2));
    let result = match determinant.sign() {
        Ordering::Less => Orientation2::Clockwise,
        Ordering::Equal => Orientation2::Collinear,
        Ordering::Greater => Orientation2::Counterclockwise,
    };
    budget.remaining -= cost;
    Ok(result)
}

/// A retained finite input point on a nonzero directed input segment.
#[derive(Clone, Copy)]
pub struct PointOnSegment2 {
    point: Point2,
    parameter: SegmentParameter2,
}

impl PointOnSegment2 {
    /// Returns the source segment.
    pub const fn segment(&self) -> Segment2 {
        self.parameter.first
    }
    /// Returns the original input point without construction rounding.
    pub const fn point(&self) -> Point2 {
        self.point
    }
    /// Returns its exact parameter on the directed source segment.
    pub const fn parameter(&self) -> SegmentParameter2 {
        self.parameter
    }
}

/// Retains a finite input point and its exact parameter when it lies on a segment.
///
/// The returned parameter is orderable with supported event parameters on the
/// same directed segment. `None` means the exact stored point is not on the
/// closed segment. A point segment itself has no directed parameter.
pub fn point_on_segment_witness(segment: Segment2, point: Point2) -> Option<PointOnSegment2> {
    if !segment.contains(point) {
        return None;
    }
    Some(PointOnSegment2 {
        point,
        parameter: parameter_for_input_point(segment, point),
    })
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

    /// Returns tight coordinate enclosures for the exact crossing.
    ///
    /// Each bound is obtained by integer comparison of the retained exact
    /// construction, without first rounding the parameter or the point. This
    /// operation applies only to this raw finite-binary64 crossing witness.
    pub fn point_enclosure(&self) -> CrossingEnclosure2 {
        self.exact_point().enclosure()
    }

    /// Retains exact rational coordinates for this raw-input crossing.
    ///
    /// The result can be compared with other supported crossing constructions,
    /// but cannot yet be used as a segment endpoint for repeated cuts.
    pub fn exact_point(&self) -> ExactCrossingPoint2 {
        let segment = self.parameter.first;
        ExactCrossingPoint2 {
            x: crossing_coordinate_numerator(segment.start.x, segment.end.x, &self.parameter),
            y: crossing_coordinate_numerator(segment.start.y, segment.end.y, &self.parameter),
            denominator: self.parameter.denominator,
        }
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
        .find(|&p| point_on_both(first, second, p))
        .expect("a single-point boundary touch has a source endpoint");
    Some(EndpointTouch2 {
        second,
        point,
        parameter: parameter_for_input_point(first, point),
    })
}

/// A positive-length collinear overlap with endpoints ordered on the first segment.
///
/// Both endpoint points are original finite binary64 input endpoints. Their
/// exact parameters can be compared with other supported events on the same
/// directed first segment. Reversing that segment reverses the returned order.
#[derive(Clone, Copy)]
pub struct SegmentOverlap2 {
    second: Segment2,
    start: Point2,
    end: Point2,
    start_parameter: SegmentParameter2,
    end_parameter: SegmentParameter2,
}

impl SegmentOverlap2 {
    /// Returns the first directed source segment.
    pub const fn first_segment(&self) -> Segment2 {
        self.start_parameter.first
    }
    /// Returns the second source segment.
    pub const fn second_segment(&self) -> Segment2 {
        self.second
    }
    /// Returns the overlap endpoint encountered first along the first segment.
    pub const fn start_point(&self) -> Point2 {
        self.start
    }
    /// Returns the overlap endpoint encountered last along the first segment.
    pub const fn end_point(&self) -> Point2 {
        self.end
    }
    /// Returns the exact first-segment parameter of the first overlap endpoint.
    pub const fn start_parameter(&self) -> SegmentParameter2 {
        self.start_parameter
    }
    /// Returns the exact first-segment parameter of the last overlap endpoint.
    pub const fn end_parameter(&self) -> SegmentParameter2 {
        self.end_parameter
    }
}

/// Retains both source endpoints and exact parameters of a positive-length overlap.
///
/// Returns `None` for disjoint segments and single-point intersections; use
/// [`intersect_segments`] to distinguish those cases. Zero-length segments are
/// excluded by [`Segment2`]. No rounded point construction is performed.
pub fn overlap_witness(first: Segment2, second: Segment2) -> Option<SegmentOverlap2> {
    if intersect_segments(first, second) != SegmentIntersection::Overlap {
        return None;
    }
    let mut candidates = [first.start, first.end, second.start, second.end]
        .into_iter()
        .filter(|&p| point_on_both(first, second, p));
    let initial = candidates
        .next()
        .expect("a positive overlap has input endpoints");
    let initial_parameter = parameter_for_input_point(first, initial);
    let (mut start, mut end) = (initial, initial);
    let (mut start_parameter, mut end_parameter) = (initial_parameter, initial_parameter);
    for point in candidates {
        let parameter = parameter_for_input_point(first, point);
        if parameter.compare_on_first(&start_parameter) == Some(Ordering::Less) {
            start = point;
            start_parameter = parameter;
        }
        if parameter.compare_on_first(&end_parameter) == Some(Ordering::Greater) {
            end = point;
            end_parameter = parameter;
        }
    }
    debug_assert!(start_parameter.compare_on_first(&end_parameter) == Some(Ordering::Less));
    Some(SegmentOverlap2 {
        second,
        start,
        end,
        start_parameter,
        end_parameter,
    })
}

/// A closed finite segment set, including the degenerate case of a point.
///
/// The point variant has no directed parameter. Nonzero segments retain their
/// directed endpoints and can produce the existing exact event witnesses.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ClosedSegment2 {
    /// Two equal endpoints, interpreted as one point.
    Point(Point2),
    /// A nonzero directed segment.
    Segment(Segment2),
}

impl ClosedSegment2 {
    /// Constructs the point or nonzero segment set from checked finite endpoints.
    pub fn from_endpoints(start: Point2, end: Point2) -> Self {
        if start == end {
            Self::Point(start)
        } else {
            Self::Segment(Segment2 { start, end })
        }
    }
}

/// The exact intersection of two closed finite segment sets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ClosedSegmentIntersection2 {
    /// The two sets are disjoint.
    Disjoint,
    /// A contact involving at least one point segment, retaining that input point.
    ///
    /// A degenerate first segment has no parameter; no order is implied here.
    PointContact(Point2),
    /// A proper crossing of two nonzero segments.
    ProperCrossing,
    /// A single-point boundary touch of two nonzero segments.
    EndpointTouch,
    /// A positive-length overlap of two nonzero segments.
    Overlap,
}

/// Intersects closed finite segment sets, including point segments.
///
/// For two nonzero segments, request the corresponding raw-input witness
/// separately when needed. A point segment intersecting another set returns the
/// original point, without a segment parameter. No tolerance, rounding or
/// general rational construction is used.
pub fn intersect_closed_segments(
    first: ClosedSegment2,
    second: ClosedSegment2,
) -> ClosedSegmentIntersection2 {
    match (first, second) {
        (ClosedSegment2::Point(a), ClosedSegment2::Point(b)) => {
            if a == b {
                ClosedSegmentIntersection2::PointContact(a)
            } else {
                ClosedSegmentIntersection2::Disjoint
            }
        }
        (ClosedSegment2::Point(p), ClosedSegment2::Segment(s))
        | (ClosedSegment2::Segment(s), ClosedSegment2::Point(p)) => {
            if s.contains(p) {
                ClosedSegmentIntersection2::PointContact(p)
            } else {
                ClosedSegmentIntersection2::Disjoint
            }
        }
        (ClosedSegment2::Segment(a), ClosedSegment2::Segment(b)) => {
            match intersect_segments(a, b) {
                SegmentIntersection::Disjoint => ClosedSegmentIntersection2::Disjoint,
                SegmentIntersection::Point(SegmentPointEvent::ProperCrossing) => {
                    ClosedSegmentIntersection2::ProperCrossing
                }
                SegmentIntersection::Point(SegmentPointEvent::BoundaryTouch) => {
                    ClosedSegmentIntersection2::EndpointTouch
                }
                SegmentIntersection::Overlap => ClosedSegmentIntersection2::Overlap,
            }
        }
    }
}

fn point_on_both(first: Segment2, second: Segment2, p: Point2) -> bool {
    first.contains(p) && second.contains(p)
}

// The caller establishes that point lies on the nonzero first segment.
fn parameter_for_input_point(first: Segment2, point: Point2) -> SegmentParameter2 {
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
    SegmentParameter2 {
        first,
        numerator,
        denominator,
    }
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

// Finite binary64 values are integers times 2^-1074. Comparisons for an
// enclosure require n * 2^1074 versus d * candidate_integer. Both fit in 99
// limbs: 66 predicate limbs plus 33 coordinate limbs.
fn shift_predicate_by_1074(value: PredicateInteger) -> [u64; 99] {
    let mut result = [0_u64; 99];
    for (i, &limb) in value.limbs.iter().enumerate() {
        result[i + 16] |= limb << 50;
        result[i + 17] |= limb >> 14;
    }
    result
}

fn multiply_predicate_coordinate(left: PredicateInteger, right: CoordinateInteger) -> [u64; 99] {
    let mut product = [0_u64; 99];
    for i in 0..PREDICATE_LIMBS {
        if left.limbs[i] == 0 {
            continue;
        }
        let mut carry = 0_u128;
        for j in 0..COORDINATE_LIMBS {
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
        let mut index = i + COORDINATE_LIMBS;
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

fn signed_coordinate_product(
    weight: PredicateInteger,
    coordinate: CoordinateInteger,
) -> ConstructionInteger {
    ConstructionInteger {
        limbs: multiply_predicate_coordinate(weight, coordinate),
        negative: coordinate.negative,
    }
    .canonical()
}

// Coordinates are exact integers in units of 2^-1074. The weighted numerator
// is A*(d-n) + B*n.
fn crossing_coordinate_numerator(
    start: f64,
    end: f64,
    parameter: &SegmentParameter2,
) -> ConstructionInteger {
    let complement = parameter.denominator.subtract(parameter.numerator);
    signed_coordinate_product(complement, CoordinateInteger::from_finite_f64(start)).add(
        signed_coordinate_product(parameter.numerator, CoordinateInteger::from_finite_f64(end)),
    )
}

// Compare the weighted numerator against d*C for each candidate coordinate C.
fn enclose_crossing_coordinate(
    numerator: ConstructionInteger,
    denominator: PredicateInteger,
) -> CoordinateEnclosure2 {
    let compare = |key: u64| {
        let bits = if key & (1_u64 << 63) == 0 {
            !key
        } else {
            key ^ (1_u64 << 63)
        };
        let candidate = CoordinateInteger::from_finite_f64(f64::from_bits(bits));
        numerator.signed_cmp(signed_coordinate_product(denominator, candidate))
    };
    let (mut low, mut high) = (0x0010_0000_0000_0000_u64, 0xffef_ffff_ffff_ffff_u64);
    while low < high {
        let middle = low + (high - low).div_ceil(2);
        if compare(middle) != Ordering::Less {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    let value = |key: u64| {
        f64::from_bits(if key & (1_u64 << 63) == 0 {
            !key
        } else {
            key ^ (1_u64 << 63)
        })
    };
    let lower = value(low);
    let upper = if compare(low) == Ordering::Equal {
        lower
    } else {
        value(low + 1)
    };
    CoordinateEnclosure2 { lower, upper }
}

// Raw crossing coordinate numerators are < 2^6298 and denominators < 2^4199.
// Cross-products are < 2^10497, fitting 165 limbs (10560 bits).
fn multiply_construction_predicate(
    left: ConstructionInteger,
    right: PredicateInteger,
) -> ComparisonInteger {
    let mut product = [0_u64; 165];
    for i in 0..99 {
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
    ComparisonInteger {
        limbs: product,
        negative: left.negative,
    }
    .canonical()
}

fn compare_construction_coordinates(
    left: ConstructionInteger,
    left_denominator: PredicateInteger,
    right: ConstructionInteger,
    right_denominator: PredicateInteger,
) -> Ordering {
    multiply_construction_predicate(left, right_denominator)
        .signed_cmp(multiply_construction_predicate(right, left_denominator))
}

#[derive(Clone, Copy)]
struct PredicateCoordinates {
    x: ConstructionInteger,
    y: ConstructionInteger,
    denominator: PredicateInteger,
}

fn predicate_coordinates(point: ExactPredicatePoint2<'_>) -> PredicateCoordinates {
    match point {
        ExactPredicatePoint2::Crossing(point) => PredicateCoordinates {
            x: point.x,
            y: point.y,
            denominator: point.denominator,
        },
        ExactPredicatePoint2::Input(point) => {
            let widen = |value: f64| {
                let coordinate = CoordinateInteger::from_finite_f64(value);
                let mut limbs = [0_u64; 99];
                limbs[..COORDINATE_LIMBS].copy_from_slice(&coordinate.limbs);
                ConstructionInteger {
                    limbs,
                    negative: coordinate.negative,
                }
            };
            let mut denominator = PredicateInteger::zero();
            denominator.limbs[0] = 1;
            PredicateCoordinates {
                x: widen(point.x),
                y: widen(point.y),
                denominator,
            }
        }
    }
}

// Each first-generation coordinate difference numerator is < 2^10498;
// the orientation determinant is < 2^20997, fitting 330 limbs (21120 bits).
fn multiply_comparison_integers(
    left: ComparisonInteger,
    right: ComparisonInteger,
) -> OrientationInteger {
    let mut product = [0_u64; 330];
    for i in 0..165 {
        if left.limbs[i] == 0 {
            continue;
        }
        let mut carry = 0_u128;
        for j in 0..165 {
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
        let mut index = i + 165;
        while carry != 0 {
            debug_assert!(index < product.len());
            let sum = u128::from(product[index]) + carry;
            product[index] = sum as u64;
            carry = sum >> 64;
            index += 1;
        }
    }
    OrientationInteger {
        limbs: product,
        negative: left.negative != right.negative,
    }
    .canonical()
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

    fn signed_cmp(self, other: Self) -> Ordering {
        match (self.sign(), other.sign()) {
            (Ordering::Less, Ordering::Less) => self.magnitude_cmp(other).reverse(),
            (Ordering::Greater, Ordering::Greater) => self.magnitude_cmp(other),
            (left, right) => left.cmp(&right),
        }
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
