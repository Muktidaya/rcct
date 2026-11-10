// SPDX-License-Identifier: Apache-2.0

use rcct::planar::{
    Orientation2, Point2, Point2Error, Segment2, Segment2Error, SegmentIntersection,
    SegmentPointEvent, SimpleWire2, Wire2Error, intersect_segments, orient2d,
};

fn point(x: f64, y: f64) -> Point2 {
    Point2::try_new(x, y).unwrap()
}

fn segment(ax: f64, ay: f64, bx: f64, by: f64) -> Segment2 {
    Segment2::try_new(point(ax, ay), point(bx, by)).unwrap()
}

#[test]
fn checked_points_and_segments_reject_invalid_inputs() {
    for (x, y) in [
        (f64::NAN, 0.0),
        (0.0, f64::NAN),
        (f64::INFINITY, 0.0),
        (0.0, f64::NEG_INFINITY),
    ] {
        assert_eq!(Point2::try_new(x, y), Err(Point2Error::NonFiniteCoordinate));
    }
    assert_eq!(
        Segment2::try_new(point(1.0, -0.0), point(1.0, 0.0)),
        Err(Segment2Error::ZeroLength)
    );
}

#[test]
fn orientation_reports_exact_sign_for_ordinary_and_collinear_points() {
    let origin = point(0.0, 0.0);
    assert_eq!(
        orient2d(origin, point(1.0, 0.0), point(0.0, 1.0)),
        Orientation2::Counterclockwise
    );
    assert_eq!(
        orient2d(origin, point(0.0, 1.0), point(1.0, 0.0)),
        Orientation2::Clockwise
    );
    assert_eq!(
        orient2d(point(-1.0, -1.0), origin, point(1.0, 1.0)),
        Orientation2::Collinear
    );
}

#[test]
fn orientation_matches_an_integer_determinant_over_a_small_grid() {
    let points: Vec<_> = (-2..=2)
        .flat_map(|x| (-2..=2).map(move |y| point(f64::from(x), f64::from(y))))
        .collect();

    for &a in &points {
        for &b in &points {
            for &c in &points {
                let ax = i128::from(a.x() as i32);
                let ay = i128::from(a.y() as i32);
                let bx = i128::from(b.x() as i32);
                let by = i128::from(b.y() as i32);
                let cx = i128::from(c.x() as i32);
                let cy = i128::from(c.y() as i32);
                let determinant = (bx - ax) * (cy - ay) - (by - ay) * (cx - ax);
                let expected = match determinant.cmp(&0) {
                    std::cmp::Ordering::Less => Orientation2::Clockwise,
                    std::cmp::Ordering::Equal => Orientation2::Collinear,
                    std::cmp::Ordering::Greater => Orientation2::Counterclockwise,
                };
                assert_eq!(orient2d(a, b, c), expected, "a={a:?}, b={b:?}, c={c:?}");
            }
        }
    }
}

#[test]
fn orientation_matches_independent_fraction_fixtures() {
    for line in include_str!("fixtures/orientation.tsv").lines() {
        if line.starts_with('#') {
            continue;
        }
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 8);
        let coordinates: Vec<_> = fields[1..7]
            .iter()
            .map(|field| f64::from_bits(u64::from_str_radix(field, 16).unwrap()))
            .collect();
        let a = Point2::try_new(coordinates[0], coordinates[1]).unwrap();
        let b = Point2::try_new(coordinates[2], coordinates[3]).unwrap();
        let c = Point2::try_new(coordinates[4], coordinates[5]).unwrap();
        let expected = match fields[7] {
            "-1" => Orientation2::Clockwise,
            "0" => Orientation2::Collinear,
            "1" => Orientation2::Counterclockwise,
            sign => panic!("unexpected exact orientation sign {sign}"),
        };
        assert_eq!(orient2d(a, b, c), expected, "case {}", fields[0]);
    }
}

#[test]
fn orientation_remains_exact_when_binary64_products_overflow_or_underflow() {
    let maximum = f64::MAX;
    let next_below_maximum = f64::from_bits(maximum.to_bits() - 1);
    // Both floating-point products overflow to infinity, so their subtraction is NaN.
    assert!((maximum * maximum - maximum * next_below_maximum).is_nan());
    assert_eq!(
        orient2d(
            point(0.0, 0.0),
            point(maximum, maximum),
            point(maximum, next_below_maximum),
        ),
        Orientation2::Clockwise
    );

    let least_subnormal = f64::from_bits(1);
    assert_eq!(least_subnormal * least_subnormal, 0.0);
    assert_eq!(
        orient2d(
            point(0.0, 0.0),
            point(least_subnormal, 0.0),
            point(0.0, least_subnormal),
        ),
        Orientation2::Counterclockwise
    );

    // Exact coordinate subtraction itself can overflow for finite inputs.
    assert_eq!(
        orient2d(
            point(-maximum, 0.0),
            point(maximum, 0.0),
            point(0.0, least_subnormal),
        ),
        Orientation2::Counterclockwise
    );
}

#[test]
fn segment_intersection_distinguishes_disjoint_crossing_touch_and_overlap() {
    assert_eq!(
        intersect_segments(segment(0.0, 0.0, 2.0, 0.0), segment(0.0, 1.0, 2.0, 1.0),),
        SegmentIntersection::Disjoint
    );
    assert_eq!(
        intersect_segments(segment(0.0, 0.0, 2.0, 2.0), segment(0.0, 2.0, 2.0, 0.0),),
        SegmentIntersection::Point(SegmentPointEvent::ProperCrossing)
    );
    assert_eq!(
        intersect_segments(segment(0.0, 0.0, 2.0, 0.0), segment(1.0, 0.0, 1.0, 1.0),),
        SegmentIntersection::Point(SegmentPointEvent::BoundaryTouch)
    );
    assert_eq!(
        intersect_segments(segment(0.0, 0.0, 2.0, 0.0), segment(2.0, 0.0, 3.0, 0.0),),
        SegmentIntersection::Point(SegmentPointEvent::BoundaryTouch)
    );
    assert_eq!(
        intersect_segments(segment(0.0, 0.0, 3.0, 0.0), segment(1.0, 0.0, 4.0, 0.0),),
        SegmentIntersection::Overlap
    );
    assert_eq!(
        intersect_segments(segment(0.0, 0.0, 1.0, 0.0), segment(2.0, 0.0, 3.0, 0.0),),
        SegmentIntersection::Disjoint
    );
    assert_eq!(
        intersect_segments(segment(1.0, -2.0, 1.0, 2.0), segment(0.0, 0.0, 2.0, 0.0),),
        SegmentIntersection::Point(SegmentPointEvent::ProperCrossing)
    );
}

#[test]
fn simple_wire_accepts_convex_concave_and_collinear_vertex_cases() {
    let square = SimpleWire2::try_new(vec![
        point(0.0, 0.0),
        point(2.0, 0.0),
        point(2.0, 2.0),
        point(0.0, 2.0),
    ])
    .unwrap();
    assert_eq!(square.len(), 4);
    assert!(!square.is_empty());
    assert_eq!(square.vertices()[0], point(0.0, 0.0));

    let concave = SimpleWire2::try_new(vec![
        point(0.0, 0.0),
        point(3.0, 0.0),
        point(1.0, 1.0),
        point(3.0, 3.0),
        point(0.0, 3.0),
    ]);
    assert!(concave.is_ok());

    let straight_through_vertex = SimpleWire2::try_new(vec![
        point(0.0, 0.0),
        point(1.0, 0.0),
        point(2.0, 0.0),
        point(2.0, 1.0),
        point(0.0, 1.0),
    ]);
    assert!(straight_through_vertex.is_ok());
}

#[test]
fn simple_wire_rejects_underspecified_degenerate_and_intersecting_cycles() {
    assert_eq!(
        SimpleWire2::try_new(vec![point(0.0, 0.0), point(1.0, 0.0)]),
        Err(Wire2Error::TooFewVertices)
    );
    assert_eq!(
        SimpleWire2::try_new(vec![point(0.0, 0.0), point(1.0, 0.0), point(0.0, 0.0)]),
        Err(Wire2Error::ZeroLengthEdge { edge: 2 })
    );
    assert_eq!(
        SimpleWire2::try_new(vec![
            point(0.0, 0.0),
            point(2.0, 0.0),
            point(1.0, 0.0),
            point(1.0, 1.0),
        ]),
        Err(Wire2Error::IntersectingEdges {
            first_edge: 0,
            second_edge: 1,
        })
    );
    assert!(matches!(
        SimpleWire2::try_new(vec![
            point(0.0, 0.0),
            point(2.0, 2.0),
            point(0.0, 2.0),
            point(2.0, 0.0),
        ]),
        Err(Wire2Error::IntersectingEdges { .. })
    ));
    assert!(matches!(
        SimpleWire2::try_new(vec![
            point(0.0, 0.0),
            point(2.0, 0.0),
            point(2.0, 2.0),
            point(1.0, 0.0),
            point(0.0, 2.0),
        ]),
        Err(Wire2Error::IntersectingEdges { .. })
    ));
}
