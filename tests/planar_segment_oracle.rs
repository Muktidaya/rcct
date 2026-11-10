// SPDX-License-Identifier: Apache-2.0

use rcct::planar::{Point2, Segment2, SegmentIntersection, SegmentPointEvent, intersect_segments};

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
