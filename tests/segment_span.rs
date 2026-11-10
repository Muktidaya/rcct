// SPDX-License-Identifier: Apache-2.0
use rcct::planar::{
    Point2, Segment2, SegmentParameter2, SegmentSpan2, SpanSplitBudget, SpanSplitError,
    proper_crossing_witness,
};
use std::cmp::Ordering;

fn p(x: f64, y: f64) -> Point2 {
    Point2::try_new(x, y).unwrap()
}
fn s(a: (f64, f64), b: (f64, f64)) -> Segment2 {
    Segment2::try_new(p(a.0, a.1), p(b.0, b.1)).unwrap()
}
fn parameter(base: Segment2, u: f64, v: f64) -> SegmentParameter2 {
    proper_crossing_witness(base, s((0.0, u), (1.0, -v)))
        .unwrap()
        .parameter()
}

#[test]
fn repeated_raw_source_cuts_preserve_exact_boundaries_and_budget() {
    let base = s((0.0, 0.0), (1.0, 0.0));
    let third = parameter(base, 1.0, 2.0);
    let two_thirds = parameter(base, 2.0, 1.0);
    let full = SegmentSpan2::whole(base);
    let mut budget = SpanSplitBudget::new(2);
    let (left, right) = full.split_at(third, &mut budget).unwrap();
    let (middle, last) = right.split_at(two_thirds, &mut budget).unwrap();
    assert_eq!(budget.remaining(), 0);
    assert_eq!(
        left.end().compare_on_first(&middle.start()),
        Some(Ordering::Equal)
    );
    assert_eq!(
        middle.end().compare_on_first(&last.start()),
        Some(Ordering::Equal)
    );
    assert_eq!(full.segment(), base);
    assert_eq!(full.start().enclosure().lower(), 0.0);
    assert_eq!(full.end().enclosure().upper(), 1.0);
    assert!(matches!(
        full.split_at(third, &mut budget),
        Err(SpanSplitError::SplitLimitExceeded)
    ));
    assert_eq!(budget.remaining(), 0);
}

#[test]
fn failed_cuts_and_invalid_spans_leave_inputs_and_budget_unchanged() {
    let base = s((0.0, 0.0), (1.0, 0.0));
    let third = parameter(base, 1.0, 2.0);
    let two_thirds = parameter(base, 2.0, 1.0);
    let full = SegmentSpan2::whole(base);
    let mut budget = SpanSplitBudget::new(1);
    let (left, _) = full.split_at(third, &mut budget).unwrap();
    let mut fresh = SpanSplitBudget::new(7);
    assert!(matches!(
        left.split_at(third, &mut fresh),
        Err(SpanSplitError::NotInterior)
    ));
    assert!(matches!(
        left.split_at(two_thirds, &mut fresh),
        Err(SpanSplitError::NotInterior)
    ));
    let other = s((1.0, 0.0), (0.0, 0.0));
    let other_at = proper_crossing_witness(other, s((0.5, -1.0), (0.5, 1.0)))
        .unwrap()
        .parameter();
    assert!(matches!(
        left.split_at(other_at, &mut fresh),
        Err(SpanSplitError::DifferentSource)
    ));
    assert!(matches!(
        SegmentSpan2::try_new(third, third),
        Err(SpanSplitError::EmptyOrReversed)
    ));
    assert!(matches!(
        SegmentSpan2::try_new(two_thirds, third),
        Err(SpanSplitError::EmptyOrReversed)
    ));
    assert!(matches!(
        SegmentSpan2::try_new(third, other_at),
        Err(SpanSplitError::DifferentSource)
    ));
    assert_eq!(fresh.remaining(), 7);
    assert_eq!(left.end().compare_on_first(&third), Some(Ordering::Equal));
}

#[test]
fn repeated_span_splits_follow_independent_fraction_order() {
    let base = s((0.0, 0.0), (1.0, 0.0));
    let mut count = 0;
    for line in include_str!("fixtures/crossing_order.tsv")
        .lines()
        .filter(|line| !line.starts_with('#'))
    {
        let f: Vec<_> = line.split('\t').collect();
        let v: Vec<f64> = f[..4]
            .iter()
            .map(|x| f64::from_bits(u64::from_str_radix(x, 16).unwrap()))
            .collect();
        let a = parameter(base, v[0], v[1]);
        let b = parameter(base, v[2], v[3]);
        let (low, high) = if f[4] == "-1" {
            (a, b)
        } else if f[4] == "1" {
            (b, a)
        } else {
            continue;
        };
        let mut budget = SpanSplitBudget::new(2);
        let (_, right) = SegmentSpan2::whole(base)
            .split_at(low, &mut budget)
            .unwrap();
        let (middle, _) = right.split_at(high, &mut budget).unwrap();
        assert_eq!(middle.start().compare_on_first(&low), Some(Ordering::Equal));
        assert_eq!(middle.end().compare_on_first(&high), Some(Ordering::Equal));
        assert_eq!(budget.remaining(), 0);
        count += 1;
    }
    assert!(count > 500);
}
