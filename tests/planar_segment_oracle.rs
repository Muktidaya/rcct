// SPDX-License-Identifier: Apache-2.0

use rcct::planar::{
    ClosedSegment2, ClosedSegmentIntersection2, Point2, Segment2, SegmentIntersection,
    SegmentPointEvent, endpoint_touch_witness, intersect_closed_segments, intersect_segments,
    overlap_witness, proper_crossing_witness,
};

type P = (i64, i64);
fn sub(a: P, b: P) -> P {
    (a.0 - b.0, a.1 - b.1)
}
fn cross(a: P, b: P) -> i64 {
    a.0 * b.1 - a.1 * b.0
}
fn segment(a: P, b: P) -> Segment2 {
    Segment2::try_new(
        Point2::try_new(a.0 as f64, a.1 as f64).unwrap(),
        Point2::try_new(b.0 as f64, b.1 as f64).unwrap(),
    )
    .unwrap()
}

// Independent parametric integer oracle: no orientation calls or float operations.
fn expected(a: P, b: P, c: P, d: P) -> SegmentIntersection {
    let (r, s, q) = (sub(b, a), sub(d, c), sub(c, a));
    let mut denominator = cross(r, s);
    if denominator != 0 {
        let (mut t, mut u) = (cross(q, s), cross(q, r));
        if denominator < 0 {
            denominator = -denominator;
            t = -t;
            u = -u;
        }
        if (0..=denominator).contains(&t) && (0..=denominator).contains(&u) {
            return SegmentIntersection::Point(
                if t > 0 && t < denominator && u > 0 && u < denominator {
                    SegmentPointEvent::ProperCrossing
                } else {
                    SegmentPointEvent::BoundaryTouch
                },
            );
        }
        return SegmentIntersection::Disjoint;
    }
    if cross(q, r) != 0 {
        return SegmentIntersection::Disjoint;
    }
    let (a, b, c, d) = if r.0 != 0 {
        (a.0, b.0, c.0, d.0)
    } else {
        (a.1, b.1, c.1, d.1)
    };
    let low = a.min(b).max(c.min(d));
    let high = a.max(b).min(c.max(d));
    if low < high {
        SegmentIntersection::Overlap
    } else if low == high {
        SegmentIntersection::Point(SegmentPointEvent::BoundaryTouch)
    } else {
        SegmentIntersection::Disjoint
    }
}

#[test]
fn all_small_grid_segment_pairs_match_parametric_oracle() {
    let points: Vec<P> = (-1..=1)
        .flat_map(|x| (-1..=1).map(move |y| (x, y)))
        .collect();
    let segments: Vec<_> = points
        .iter()
        .flat_map(|&a| {
            points
                .iter()
                .filter(move |&&b| a != b)
                .map(move |&b| (a, b))
        })
        .collect();
    assert_eq!(segments.len(), 72);
    for &(a, b) in &segments {
        for &(c, d) in &segments {
            assert_eq!(
                intersect_segments(segment(a, b), segment(c, d)),
                expected(a, b, c, d),
                "{a:?} {b:?} {c:?} {d:?}"
            );
        }
    }
}

fn on_segment(a: P, b: P, p: P) -> bool {
    cross(sub(b, a), sub(p, a)) == 0
        && (a.0.min(b.0)..=a.0.max(b.0)).contains(&p.0)
        && (a.1.min(b.1)..=a.1.max(b.1)).contains(&p.1)
}

#[test]
fn point_containment_matches_independent_integer_oracle() {
    let points: Vec<P> = (-2..=2)
        .flat_map(|x| (-2..=2).map(move |y| (x, y)))
        .collect();
    for &a in &points {
        for &b in &points {
            if a == b {
                continue;
            }
            let s = segment(a, b);
            for &p in &points {
                assert_eq!(
                    s.contains(Point2::try_new(p.0 as f64, p.1 as f64).unwrap()),
                    on_segment(a, b, p),
                    "{a:?} {b:?} {p:?}"
                );
            }
        }
    }
    let max = f64::MAX;
    let tiny = f64::from_bits(1);
    let s = Segment2::try_new(
        Point2::try_new(-max, 0.0).unwrap(),
        Point2::try_new(max, -0.0).unwrap(),
    )
    .unwrap();
    assert!(s.contains(Point2::try_new(tiny, 0.0).unwrap()));
    assert!(!s.contains(Point2::try_new(tiny, tiny).unwrap()));
}

#[test]
fn closed_segment_sets_match_independent_small_grid_oracle() {
    let points: Vec<P> = (-1..=1)
        .flat_map(|x| (-1..=1).map(move |y| (x, y)))
        .collect();
    let pairs: Vec<_> = points
        .iter()
        .flat_map(|&a| points.iter().map(move |&b| (a, b)))
        .collect();
    assert_eq!(pairs.len(), 81);
    for &(a, b) in &pairs {
        for &(c, d) in &pairs {
            let p = |p: P| Point2::try_new(p.0 as f64, p.1 as f64).unwrap();
            let first = ClosedSegment2::from_endpoints(p(a), p(b));
            let second = ClosedSegment2::from_endpoints(p(c), p(d));
            let result = intersect_closed_segments(first, second);
            if a == b || c == d {
                let expected_point = if a == b { a } else { c };
                let present = if a == b && c == d {
                    a == c
                } else if a == b {
                    on_segment(c, d, a)
                } else {
                    on_segment(a, b, c)
                };
                match result {
                    ClosedSegmentIntersection2::PointContact(actual) if present => {
                        assert_eq!(actual, p(expected_point))
                    }
                    ClosedSegmentIntersection2::Disjoint if !present => (),
                    _ => panic!("wrong point-segment result: {a:?} {b:?} {c:?} {d:?}"),
                }
            } else {
                let agrees = matches!(
                    (expected(a, b, c, d), result),
                    (
                        SegmentIntersection::Disjoint,
                        ClosedSegmentIntersection2::Disjoint
                    ) | (
                        SegmentIntersection::Point(SegmentPointEvent::ProperCrossing),
                        ClosedSegmentIntersection2::ProperCrossing
                    ) | (
                        SegmentIntersection::Point(SegmentPointEvent::BoundaryTouch),
                        ClosedSegmentIntersection2::EndpointTouch
                    ) | (
                        SegmentIntersection::Overlap,
                        ClosedSegmentIntersection2::Overlap
                    )
                );
                assert!(agrees, "wrong nonzero result: {a:?} {b:?} {c:?} {d:?}");
            }
        }
    }
}

#[test]
fn point_segment_contacts_respect_extremes_and_signed_zero() {
    let p = |x, y| Point2::try_new(x, y).unwrap();
    let max = f64::MAX;
    let tiny = f64::from_bits(1);
    let whole = ClosedSegment2::from_endpoints(p(-max, 0.0), p(max, -0.0));
    let on = ClosedSegment2::from_endpoints(p(tiny, 0.0), p(tiny, -0.0));
    let off = ClosedSegment2::from_endpoints(p(tiny, tiny), p(tiny, tiny));
    assert!(
        matches!(intersect_closed_segments(on,whole), ClosedSegmentIntersection2::PointContact(point) if point == p(tiny,0.0))
    );
    assert!(
        matches!(intersect_closed_segments(whole,on), ClosedSegmentIntersection2::PointContact(point) if point == p(tiny,0.0))
    );
    assert!(matches!(
        intersect_closed_segments(off, whole),
        ClosedSegmentIntersection2::Disjoint
    ));
    let z = ClosedSegment2::from_endpoints(p(0.0, -0.0), p(-0.0, 0.0));
    assert!(matches!(
        intersect_closed_segments(z, z),
        ClosedSegmentIntersection2::PointContact(_)
    ));
}

fn input_parameter(a: P, b: P, p: P) -> (i64, i64) {
    let (mut numerator, mut denominator) = if a.0 != b.0 {
        (p.0 - a.0, b.0 - a.0)
    } else {
        (p.1 - a.1, b.1 - a.1)
    };
    if denominator < 0 {
        numerator = -numerator;
        denominator = -denominator;
    }
    (numerator, denominator)
}

#[test]
fn all_event_parameters_match_integer_oracle() {
    let points: Vec<P> = (-1..=1)
        .flat_map(|x| (-1..=1).map(move |y| (x, y)))
        .collect();
    let segments: Vec<_> = points
        .iter()
        .flat_map(|&a| {
            points
                .iter()
                .filter(move |&&b| a != b)
                .map(move |&b| (a, b))
        })
        .collect();
    let mut comparisons = 0;
    for &(a, b) in &segments {
        let first = segment(a, b);
        let mut events = Vec::new();
        for &(c, d) in &segments {
            let second = segment(c, d);
            match expected(a, b, c, d) {
                SegmentIntersection::Point(SegmentPointEvent::BoundaryTouch) => {
                    let p = [a, b, c, d]
                        .into_iter()
                        .find(|&p| on_segment(a, b, p) && on_segment(c, d, p))
                        .unwrap();
                    let witness = endpoint_touch_witness(first, second).unwrap();
                    assert_eq!(
                        witness.point(),
                        Point2::try_new(p.0 as f64, p.1 as f64).unwrap()
                    );
                    let (numerator, denominator) = input_parameter(a, b, p);
                    events.push((witness.parameter(), numerator, denominator));
                    assert!(overlap_witness(first, second).is_none());
                }
                SegmentIntersection::Point(SegmentPointEvent::ProperCrossing) => {
                    let (r, s, q) = (sub(b, a), sub(d, c), sub(c, a));
                    let (mut numerator, mut denominator) = (cross(q, s), cross(r, s));
                    if denominator < 0 {
                        numerator = -numerator;
                        denominator = -denominator;
                    }
                    events.push((
                        proper_crossing_witness(first, second).unwrap().parameter(),
                        numerator,
                        denominator,
                    ));
                    assert!(endpoint_touch_witness(first, second).is_none());
                    assert!(overlap_witness(first, second).is_none());
                }
                SegmentIntersection::Overlap => {
                    let mut endpoints: Vec<_> = [a, b, c, d]
                        .into_iter()
                        .filter(|&p| on_segment(a, b, p) && on_segment(c, d, p))
                        .map(|p| {
                            let (n, d) = input_parameter(a, b, p);
                            (p, n, d)
                        })
                        .collect();
                    endpoints.sort_by(|l, r| (l.1 * r.2).cmp(&(r.1 * l.2)));
                    let (p0, n0, d0) = endpoints.first().copied().unwrap();
                    let (p1, n1, d1) = endpoints.last().copied().unwrap();
                    assert!(n0 * d1 < n1 * d0);
                    let witness = overlap_witness(first, second).unwrap();
                    assert_eq!(
                        witness.start_point(),
                        Point2::try_new(p0.0 as f64, p0.1 as f64).unwrap()
                    );
                    assert_eq!(
                        witness.end_point(),
                        Point2::try_new(p1.0 as f64, p1.1 as f64).unwrap()
                    );
                    events.push((witness.start_parameter(), n0, d0));
                    events.push((witness.end_parameter(), n1, d1));
                    assert!(endpoint_touch_witness(first, second).is_none());
                }
                SegmentIntersection::Disjoint => {
                    assert!(endpoint_touch_witness(first, second).is_none());
                    assert!(overlap_witness(first, second).is_none());
                }
            }
        }
        for (left, ln, ld) in &events {
            for (right, rn, rd) in &events {
                assert_eq!(
                    left.compare_on_first(right),
                    Some((ln * rd).cmp(&(rn * ld))),
                    "first={a:?} to {b:?}"
                );
                comparisons += 1;
            }
        }
    }
    assert!(comparisons > 10000);
}
