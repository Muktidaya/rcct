// SPDX-License-Identifier: Apache-2.0
use rcct::planar::{
    Point2, Segment2, SegmentParameter2, SegmentSpan2, SpanIntersectionKind2, SpanQueryBudget,
    SpanQueryError, SpanSplitBudget, point_on_segment_witness, proper_crossing_witness,
};
use std::cmp::Ordering;
fn p(x: f64, y: f64) -> Point2 {
    Point2::try_new(x, y).unwrap()
}
fn s(a: (f64, f64), b: (f64, f64)) -> Segment2 {
    Segment2::try_new(p(a.0, a.1), p(b.0, b.1)).unwrap()
}
fn crossing(base: Segment2, u: f64, v: f64) -> SegmentParameter2 {
    proper_crossing_witness(base, s((0.0, u), (1.0, -v)))
        .unwrap()
        .parameter()
}
fn same(a: SegmentParameter2, b: SegmentParameter2) -> bool {
    a.compare_on_first(&b) == Some(Ordering::Equal)
}

#[test]
fn clips_point_and_overlap_events_exactly_to_rational_span() {
    let base = s((0.0, 0.0), (1.0, 0.0));
    let third = crossing(base, 1.0, 2.0);
    let two_thirds = crossing(base, 2.0, 1.0);
    let span = SegmentSpan2::try_new(third, two_thirds).unwrap();
    let half = point_on_segment_witness(base, p(0.5, 0.0))
        .unwrap()
        .parameter();
    let mut budget = SpanQueryBudget::new(8);
    let interior = span
        .intersect_raw(s((0.5, -1.0), (0.5, 1.0)), &mut budget)
        .unwrap();
    assert_eq!(interior.kind(), SpanIntersectionKind2::ProperCrossing);
    assert!(same(interior.first_parameter().unwrap(), half));
    assert!(interior.last_parameter().is_none());
    let boundary = span
        .intersect_raw(s((0.0, 1.0), (1.0, -2.0)), &mut budget)
        .unwrap();
    assert_eq!(boundary.kind(), SpanIntersectionKind2::BoundaryTouch);
    assert!(same(boundary.first_parameter().unwrap(), third));
    let touch = span
        .intersect_raw(s((0.5, 0.0), (0.5, 1.0)), &mut budget)
        .unwrap();
    assert_eq!(touch.kind(), SpanIntersectionKind2::BoundaryTouch);
    let outside = span
        .intersect_raw(s((0.25, -1.0), (0.25, 1.0)), &mut budget)
        .unwrap();
    assert_eq!(outside.kind(), SpanIntersectionKind2::Disjoint);
    assert!(outside.first_parameter().is_none());
    let overlap = span
        .intersect_raw(s((0.5, 0.0), (1.0, 0.0)), &mut budget)
        .unwrap();
    assert_eq!(overlap.kind(), SpanIntersectionKind2::Overlap);
    assert!(same(overlap.first_parameter().unwrap(), half));
    assert!(same(overlap.last_parameter().unwrap(), two_thirds));
    let all = span.intersect_raw(base, &mut budget).unwrap();
    assert_eq!(all.kind(), SpanIntersectionKind2::Overlap);
    assert!(same(all.first_parameter().unwrap(), third));
    assert!(same(all.last_parameter().unwrap(), two_thirds));
    let three_quarters = point_on_segment_witness(base, p(0.75, 0.0))
        .unwrap()
        .parameter();
    let right = SegmentSpan2::try_new(half, three_quarters).unwrap();
    let clipped = right
        .intersect_raw(s((0.0, 0.0), (0.5, 0.0)), &mut budget)
        .unwrap();
    assert_eq!(clipped.kind(), SpanIntersectionKind2::BoundaryTouch);
    assert!(same(clipped.first_parameter().unwrap(), half));
    assert!(clipped.last_parameter().is_none());
    assert_eq!(budget.remaining(), 1);
}

#[test]
fn repeated_queries_and_splits_are_budgeted_and_atomic() {
    let base = s((0.0, 0.0), (1.0, 0.0));
    let full = SegmentSpan2::whole(base);
    let mut query = SpanQueryBudget::new(2);
    let mut split = SpanSplitBudget::new(2);
    let first = full
        .intersect_raw(s((0.0, 1.0), (1.0, -2.0)), &mut query)
        .unwrap();
    let (_, right) = full
        .split_at(first.first_parameter().unwrap(), &mut split)
        .unwrap();
    let second = right
        .intersect_raw(s((0.0, 2.0), (1.0, -1.0)), &mut query)
        .unwrap();
    let (middle, _) = right
        .split_at(second.first_parameter().unwrap(), &mut split)
        .unwrap();
    assert_eq!(second.kind(), SpanIntersectionKind2::ProperCrossing);
    assert!(same(middle.start(), first.first_parameter().unwrap()));
    assert!(same(middle.end(), second.first_parameter().unwrap()));
    assert_eq!(query.remaining(), 0);
    assert_eq!(split.remaining(), 0);
    let before_start = middle.start();
    let before_end = middle.end();
    assert!(matches!(
        middle.intersect_raw(base, &mut query),
        Err(SpanQueryError::QueryLimitExceeded)
    ));
    assert_eq!(query.remaining(), 0);
    assert!(same(middle.start(), before_start));
    assert!(same(middle.end(), before_end));
}

#[test]
fn reversed_support_preserves_traversal_and_contact() {
    let base = s((1.0, 0.0), (0.0, -0.0));
    let full = SegmentSpan2::whole(base);
    let cut = proper_crossing_witness(base, s((0.25, -1.0), (0.25, 1.0)))
        .unwrap()
        .parameter();
    let mut split = SpanSplitBudget::new(1);
    let (left, _) = full.split_at(cut, &mut split).unwrap();
    let mut query = SpanQueryBudget::new(1);
    let event = left
        .intersect_raw(s((0.0, 0.0), (0.5, 0.0)), &mut query)
        .unwrap();
    assert_eq!(event.kind(), SpanIntersectionKind2::Overlap);
    let half = point_on_segment_witness(base, p(0.5, 0.0))
        .unwrap()
        .parameter();
    assert!(same(event.first_parameter().unwrap(), half));
    assert!(same(event.last_parameter().unwrap(), cut));
}

#[test]
fn span_point_queries_match_independent_fraction_oracle() {
    let base = s((0.0, 0.0), (1.0, 0.0));
    let mut count = 0;
    for line in include_str!("fixtures/span_query.tsv")
        .lines()
        .filter(|line| !line.starts_with('#'))
    {
        let f: Vec<_> = line.split('\t').collect();
        assert_eq!(f.len(), 7);
        let v: Vec<f64> = f[..6]
            .iter()
            .map(|x| f64::from_bits(u64::from_str_radix(x, 16).unwrap()))
            .collect();
        let start = crossing(base, v[0], v[1]);
        let end = crossing(base, v[2], v[3]);
        let cutter = s((0.0, v[4]), (1.0, -v[5]));
        let span = SegmentSpan2::try_new(start, end).unwrap();
        let mut budget = SpanQueryBudget::new(1);
        let event = span.intersect_raw(cutter, &mut budget).unwrap();
        let expected = match f[6] {
            "Disjoint" => SpanIntersectionKind2::Disjoint,
            "BoundaryTouch" => SpanIntersectionKind2::BoundaryTouch,
            "ProperCrossing" => SpanIntersectionKind2::ProperCrossing,
            _ => panic!("unknown event class"),
        };
        assert_eq!(event.kind(), expected, "case {count}");
        if expected == SpanIntersectionKind2::Disjoint {
            assert!(event.first_parameter().is_none());
        } else {
            let expected_parameter = crossing(base, v[4], v[5]);
            assert!(same(event.first_parameter().unwrap(), expected_parameter));
        }
        assert!(event.last_parameter().is_none());
        assert_eq!(budget.remaining(), 0);
        count += 1;
    }
    assert_eq!(count, 512);
}

#[test]
fn span_overlap_queries_match_independent_fraction_oracle() {
    let base = s((0.0, 0.0), (1.0, 0.0));
    let mut count = 0;
    for line in include_str!("fixtures/span_overlap_query.tsv")
        .lines()
        .filter(|line| !line.starts_with('#'))
    {
        let f: Vec<_> = line.split('\t').collect();
        assert_eq!(f.len(), 9);
        let v: Vec<f64> = f[..6]
            .iter()
            .map(|x| f64::from_bits(u64::from_str_radix(x, 16).unwrap()))
            .collect();
        let a = crossing(base, v[0], v[1]);
        let b = crossing(base, v[2], v[3]);
        let c = point_on_segment_witness(base, p(v[4], 0.0))
            .unwrap()
            .parameter();
        let d = point_on_segment_witness(base, p(v[5], 0.0))
            .unwrap()
            .parameter();
        let span = SegmentSpan2::try_new(a, b).unwrap();
        let mut budget = SpanQueryBudget::new(1);
        let event = span
            .intersect_raw(s((v[4], 0.0), (v[5], 0.0)), &mut budget)
            .unwrap();
        let expected = match f[6] {
            "Disjoint" => SpanIntersectionKind2::Disjoint,
            "BoundaryTouch" => SpanIntersectionKind2::BoundaryTouch,
            "Overlap" => SpanIntersectionKind2::Overlap,
            _ => panic!("unknown event class"),
        };
        assert_eq!(event.kind(), expected, "case {count}");
        let parameter = |label| match label {
            "A" => a,
            "B" => b,
            "C" => c,
            "D" => d,
            _ => panic!("unknown boundary source"),
        };
        if expected == SpanIntersectionKind2::Disjoint {
            assert!(event.first_parameter().is_none());
        } else {
            assert!(
                same(event.first_parameter().unwrap(), parameter(f[7])),
                "first case {count}"
            );
        }
        if expected == SpanIntersectionKind2::Overlap {
            assert!(
                same(event.last_parameter().unwrap(), parameter(f[8])),
                "last case {count}"
            );
        } else {
            assert!(event.last_parameter().is_none());
        }
        assert_eq!(budget.remaining(), 0);
        count += 1;
    }
    assert_eq!(count, 512);
}
