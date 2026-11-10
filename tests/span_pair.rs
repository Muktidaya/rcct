// SPDX-License-Identifier: Apache-2.0
use rcct::planar::{
    LocatedPoint2, LocatedSpan2, Point2, Segment2, SegmentParameter2, SegmentSpan2,
    SpanBoundarySource2, SpanIntersectionKind2, SpanLocationBudget, SpanQueryBudget,
    SpanQueryError, point_on_segment_witness, proper_crossing_witness,
};
use std::cmp::Ordering;

fn p(x: f64, y: f64) -> Point2 {
    Point2::try_new(x, y).unwrap()
}
fn s(a: (f64, f64), b: (f64, f64)) -> Segment2 {
    Segment2::try_new(p(a.0, a.1), p(b.0, b.1)).unwrap()
}
fn axis_segment(a: i64, b: i64, vertical: bool) -> Segment2 {
    if vertical {
        s((0., a as f64), (0., b as f64))
    } else {
        s((a as f64, 0.), (b as f64, 0.))
    }
}
fn parameter(segment: Segment2, n: i64, d: i64, vertical: bool) -> SegmentParameter2 {
    if n == 0 {
        return point_on_segment_witness(segment, segment.start())
            .unwrap()
            .parameter();
    }
    if n == d {
        return point_on_segment_witness(segment, segment.end())
            .unwrap()
            .parameter();
    }
    let (a, b) = (segment.start(), segment.end());
    let cutter = if vertical {
        s((n as f64, a.y()), ((n - d) as f64, b.y()))
    } else {
        s((a.x(), n as f64), (b.x(), (n - d) as f64))
    };
    proper_crossing_witness(segment, cutter)
        .unwrap()
        .parameter()
}
fn same(a: SegmentParameter2, b: SegmentParameter2) -> bool {
    a.compare_on_first(&b) == Some(Ordering::Equal)
}

fn compare_located_and_source_spans(
    a: &SegmentSpan2,
    b: &SegmentSpan2,
    expected: SpanIntersectionKind2,
) {
    let mut budget = SpanQueryBudget::new(2);
    let source = a.intersect_span(b, &mut budget).unwrap();
    let located = LocatedSpan2::from_source_span(a)
        .intersect_span(&LocatedSpan2::from_source_span(b), &mut budget)
        .unwrap();
    assert_eq!(located.kind(), expected);
    if expected == SpanIntersectionKind2::Disjoint {
        assert!(located.first_boundary().is_none());
        assert!(located.last_boundary().is_none());
        assert!(located.point_on_second().is_none());
        return;
    }
    let first = source.first_boundary().unwrap();
    let actual = located.first_boundary().unwrap();
    assert_eq!(first.source(), actual.source());
    assert!(
        first
            .parameter()
            .same_point(&actual.point().retained_parameter())
    );
    if expected == SpanIntersectionKind2::Overlap {
        let last = source.last_boundary().unwrap();
        let actual = located.last_boundary().unwrap();
        assert_eq!(last.source(), actual.source());
        assert!(
            last.parameter()
                .same_point(&actual.point().retained_parameter())
        );
        assert!(located.point_on_second().is_none());
    } else {
        assert!(located.last_boundary().is_none());
        assert!(
            source
                .point_on_second()
                .unwrap()
                .same_point(&located.point_on_second().unwrap().retained_parameter())
        );
    }
}

#[derive(Clone, Copy)]
struct Fraction {
    n: i128,
    d: i128,
}
impl Fraction {
    fn cmp(self, b: Self) -> Ordering {
        (self.n * b.d).cmp(&(b.n * self.d))
    }
}
fn value(start: i64, end: i64, n: i64, d: i64) -> Fraction {
    Fraction {
        n: i128::from(start * d + (end - start) * n),
        d: i128::from(d),
    }
}

#[test]
fn collinear_span_pairs_match_independent_rational_oracle() {
    // The oracle uses small signed i128 cross-products, independently from
    // RCCT's floating-input fixed-limb construction and comparison code.
    let ratios = [(0, 1), (1, 5), (1, 3), (1, 2), (2, 3), (4, 5), (1, 1)];
    let mut random = 0x9e37_79b9_u64;
    let mut next = |n: usize| {
        random = random
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((random >> 32) as usize) % n
    };
    for case in 0..2400 {
        let mut ends = [0_i64; 4];
        ends[0] = next(11) as i64 - 5;
        ends[1] = next(11) as i64 - 5;
        ends[2] = next(11) as i64 - 5;
        ends[3] = next(11) as i64 - 5;
        if ends[0] == ends[1] {
            ends[1] += 1;
        }
        if ends[2] == ends[3] {
            ends[3] += 1;
        }
        let mut a = [next(ratios.len()), next(ratios.len())];
        let mut b = [next(ratios.len()), next(ratios.len())];
        if a[0] == a[1] {
            a[1] = (a[1] + 1) % ratios.len();
        }
        if b[0] == b[1] {
            b[1] = (b[1] + 1) % ratios.len();
        }
        a.sort();
        b.sort();
        let vertical = case % 2 == 0;
        let first = axis_segment(ends[0], ends[1], vertical);
        let second = axis_segment(ends[2], ends[3], vertical);
        let mut params = Vec::new();
        let mut values = Vec::new();
        for (segment, start, end, indexes) in
            [(first, ends[0], ends[1], a), (second, ends[2], ends[3], b)]
        {
            for index in indexes {
                let (n, d) = ratios[index];
                params.push(parameter(segment, n, d, vertical));
                values.push(value(start, end, n, d));
            }
        }
        let first_span = SegmentSpan2::try_new(params[0], params[1]).unwrap();
        let second_span = SegmentSpan2::try_new(params[2], params[3]).unwrap();
        for i in 0..2 {
            for j in 2..4 {
                assert_eq!(
                    params[i].compare_point_lexicographically(&params[j]),
                    values[i].cmp(values[j]),
                    "point case {case}"
                );
            }
        }
        // Order coordinates along the first support; pick max lower/min upper.
        let cmp = |i: usize, j: usize| {
            let c = values[i].cmp(values[j]);
            if ends[0] < ends[1] { c } else { c.reverse() }
        };
        let (b_low, b_high) = if cmp(2, 3) == Ordering::Less {
            (2, 3)
        } else {
            (3, 2)
        };
        let low = if cmp(0, b_low) == Ordering::Less {
            b_low
        } else {
            0
        };
        let high = if cmp(1, b_high) == Ordering::Greater {
            b_high
        } else {
            1
        };
        let expected = match cmp(low, high) {
            Ordering::Greater => SpanIntersectionKind2::Disjoint,
            Ordering::Equal => SpanIntersectionKind2::BoundaryTouch,
            Ordering::Less => SpanIntersectionKind2::Overlap,
        };
        let mut budget = SpanQueryBudget::new(1);
        let result = first_span
            .intersect_span(&second_span, &mut budget)
            .unwrap();
        let mut reference_budget = SpanQueryBudget::new(2);
        let raw = first_span
            .intersect_raw(second, &mut reference_budget)
            .unwrap();
        let whole = first_span
            .intersect_span(&SegmentSpan2::whole(second), &mut reference_budget)
            .unwrap();
        assert_eq!(raw.kind(), whole.kind(), "raw equivalence case {case}");
        if let Some(boundary) = raw.first_parameter() {
            assert!(
                boundary.same_point(&whole.first_boundary().unwrap().parameter()),
                "raw first case {case}"
            );
        }
        if let Some(boundary) = raw.last_parameter() {
            assert!(
                boundary.same_point(&whole.last_boundary().unwrap().parameter()),
                "raw last case {case}"
            );
        }
        assert_eq!(result.kind(), expected, "case {case}: {ends:?}");
        compare_located_and_source_spans(&first_span, &second_span, expected);
        if case % 4 == 0 {
            // Reconstruct every endpoint on a reversed provenance support, then
            // locate it back onto its original target before the pair query.
            let reverse_a = axis_segment(ends[1], ends[0], vertical);
            let reverse_b = axis_segment(ends[3], ends[2], vertical);
            let mut location_budget = SpanLocationBudget::new(4);
            let mut locate_reversed = |segment, reverse, index| {
                let (n, d) = ratios[index];
                let p = parameter(reverse, d - n, d, vertical);
                LocatedPoint2::locate(segment, p, &mut location_budget)
                    .unwrap()
                    .unwrap()
            };
            let located_a = LocatedSpan2::try_new(
                locate_reversed(first, reverse_a, a[0]),
                locate_reversed(first, reverse_a, a[1]),
            )
            .unwrap();
            let located_b = LocatedSpan2::try_new(
                locate_reversed(second, reverse_b, b[0]),
                locate_reversed(second, reverse_b, b[1]),
            )
            .unwrap();
            assert_eq!(location_budget.remaining(), 0);
            let mut query = SpanQueryBudget::new(1);
            let event = located_a.intersect_span(&located_b, &mut query).unwrap();
            assert_eq!(event.kind(), expected, "foreign origin case {case}");
            if expected != SpanIntersectionKind2::Disjoint {
                assert!(
                    event
                        .first_boundary()
                        .unwrap()
                        .point()
                        .retained_parameter()
                        .same_point(&result.first_boundary().unwrap().parameter())
                );
            }
            if expected == SpanIntersectionKind2::Overlap {
                assert!(
                    event
                        .last_boundary()
                        .unwrap()
                        .point()
                        .retained_parameter()
                        .same_point(&result.last_boundary().unwrap().parameter())
                );
            } else if expected == SpanIntersectionKind2::BoundaryTouch {
                assert!(
                    event
                        .point_on_second()
                        .unwrap()
                        .retained_parameter()
                        .same_point(&result.point_on_second().unwrap())
                );
            }
        }
        assert_eq!(budget.remaining(), 0);
        if expected == SpanIntersectionKind2::Overlap {
            for (boundary, index) in [
                (result.first_boundary().unwrap(), low),
                (result.last_boundary().unwrap(), high),
            ] {
                assert_eq!(
                    boundary.source(),
                    if index < 2 {
                        SpanBoundarySource2::First
                    } else {
                        SpanBoundarySource2::Second
                    },
                    "case {case}"
                );
                assert!(same(boundary.parameter(), params[index]), "case {case}");
            }
            assert!(result.point_on_second().is_none());
        } else if expected == SpanIntersectionKind2::Disjoint {
            assert!(result.first_boundary().is_none());
            assert!(result.last_boundary().is_none());
            assert!(result.point_on_second().is_none());
        } else {
            let first_param = result.first_boundary().unwrap().parameter();
            let second_param = result.point_on_second().unwrap();
            assert!(first_param.same_point(&second_param), "case {case}");
            let point = values[low];
            // A support endpoint may touch the interior of the other support.
            for (param, segment, start, end) in [
                (first_param, first, ends[0], ends[1]),
                (second_param, second, ends[2], ends[3]),
            ] {
                let mut n = point.n - i128::from(start) * point.d;
                let mut d = i128::from(end - start) * point.d;
                if d < 0 {
                    n = -n;
                    d = -d;
                }
                assert!(
                    same(param, parameter(segment, n as i64, d as i64, vertical)),
                    "case {case}"
                );
            }
        }
    }
}

#[test]
fn crossing_touch_disjoint_and_budget_failure() {
    let horizontal = s((0., 0.), (1., 0.));
    let vertical = s((0.5, -1.), (0.5, 1.));
    let first = SegmentSpan2::whole(horizontal);
    let second = SegmentSpan2::whole(vertical);
    let mut budget = SpanQueryBudget::new(2);
    let event = first.intersect_span(&second, &mut budget).unwrap();
    assert_eq!(event.kind(), SpanIntersectionKind2::ProperCrossing);
    assert!(same(
        event.first_boundary().unwrap().parameter(),
        proper_crossing_witness(horizontal, vertical)
            .unwrap()
            .parameter()
    ));
    assert!(same(
        event.point_on_second().unwrap(),
        proper_crossing_witness(vertical, horizontal)
            .unwrap()
            .parameter()
    ));
    let boundary = event.first_boundary().unwrap().parameter();
    let restricted = SegmentSpan2::try_new(boundary, first.end()).unwrap();
    let event = restricted.intersect_span(&second, &mut budget).unwrap();
    assert_eq!(event.kind(), SpanIntersectionKind2::BoundaryTouch);
    assert!(matches!(
        first.intersect_span(&second, &mut budget),
        Err(SpanQueryError::QueryLimitExceeded)
    ));
    assert_eq!(budget.remaining(), 0);
    assert!(same(restricted.start(), boundary));
}

#[test]
fn identical_rational_contact_at_extreme_coordinate_scales() {
    for (a, b) in [(-f64::MAX, f64::MAX), (0.0, f64::from_bits(1))] {
        let forward = s((a, 0.0), (b, 0.0));
        let reverse = s((b, 0.0), (a, 0.0));
        let cutter = s((a, 1.0), (b, -2.0));
        let first_cut = proper_crossing_witness(forward, cutter)
            .unwrap()
            .parameter();
        let second_cut = proper_crossing_witness(reverse, cutter)
            .unwrap()
            .parameter();
        let first = SegmentSpan2::try_new(SegmentSpan2::whole(forward).start(), first_cut).unwrap();
        let second =
            SegmentSpan2::try_new(SegmentSpan2::whole(reverse).start(), second_cut).unwrap();
        let mut budget = SpanQueryBudget::new(1);
        let event = first.intersect_span(&second, &mut budget).unwrap();
        assert_eq!(event.kind(), SpanIntersectionKind2::BoundaryTouch);
        assert!(same(event.first_boundary().unwrap().parameter(), first_cut));
        assert!(same(event.point_on_second().unwrap(), second_cut));
        assert!(first_cut.same_point(&second_cut));
    }
}

#[test]
fn exact_point_identity_across_distinct_supports() {
    let a = axis_segment(0, 1, false);
    let b = axis_segment(0, 3, false);
    let third = parameter(a, 1, 3, false);
    let ninth = parameter(b, 1, 9, false);
    assert!(third.same_point(&ninth));
    let origin_y = axis_segment(0, 1, true);
    let vertical_third = parameter(origin_y, 1, 3, true);
    assert_eq!(
        third.compare_point_lexicographically(&vertical_third),
        Ordering::Greater
    );
    assert_eq!(
        vertical_third.compare_point_lexicographically(&third),
        Ordering::Less
    );
    let v = s((0., 1.), (0., 2.));
    let half_y = parameter(origin_y, 1, 2, true);
    let y_one = point_on_segment_witness(v, v.start()).unwrap().parameter();
    assert_eq!(
        half_y.compare_point_lexicographically(&y_one),
        Ordering::Less
    );
}

#[test]
fn transverse_span_pairs_match_independent_rational_oracle() {
    let first = axis_segment(0, 1, false);
    let ratios_a = [(0, 1), (1, 3), (2, 3), (1, 1)];
    let ratios_b = [(0, 1), (1, 4), (1, 2), (3, 4), (1, 1)];
    let mut cases = 0;
    for rise in 1..=8 {
        for fall in 1..=8 {
            let second = s((0., rise as f64), (1., -(fall as f64)));
            let crossing_first = proper_crossing_witness(first, second).unwrap().parameter();
            let crossing_second = proper_crossing_witness(second, first).unwrap().parameter();
            let t = Fraction {
                n: i128::from(rise),
                d: i128::from(rise + fall),
            };
            for i in 0..ratios_a.len() {
                for j in i + 1..ratios_a.len() {
                    let (an, ad) = ratios_a[i];
                    let (bn, bd) = ratios_a[j];
                    let a = SegmentSpan2::try_new(
                        parameter(first, an, ad, false),
                        parameter(first, bn, bd, false),
                    )
                    .unwrap();
                    for k in 0..ratios_b.len() {
                        for l in k + 1..ratios_b.len() {
                            let cut = |(n, d): (i64, i64)| {
                                if n == 0 {
                                    return point_on_segment_witness(second, second.start())
                                        .unwrap()
                                        .parameter();
                                }
                                if n == d {
                                    return point_on_segment_witness(second, second.end())
                                        .unwrap()
                                        .parameter();
                                }
                                let x = n as f64 / d as f64;
                                proper_crossing_witness(second, s((x, -100.), (x, 100.)))
                                    .unwrap()
                                    .parameter()
                            };
                            let b =
                                SegmentSpan2::try_new(cut(ratios_b[k]), cut(ratios_b[l])).unwrap();
                            let as_fraction = |(n, d)| Fraction { n, d };
                            let lower_a = t.cmp(as_fraction((i128::from(an), i128::from(ad))));
                            let upper_a = t.cmp(as_fraction((i128::from(bn), i128::from(bd))));
                            let lower_b = t.cmp(as_fraction((
                                i128::from(ratios_b[k].0),
                                i128::from(ratios_b[k].1),
                            )));
                            let upper_b = t.cmp(as_fraction((
                                i128::from(ratios_b[l].0),
                                i128::from(ratios_b[l].1),
                            )));
                            let expected = if lower_a == Ordering::Less
                                || upper_a == Ordering::Greater
                                || lower_b == Ordering::Less
                                || upper_b == Ordering::Greater
                            {
                                SpanIntersectionKind2::Disjoint
                            } else if [lower_a, upper_a, lower_b, upper_b]
                                .contains(&Ordering::Equal)
                            {
                                SpanIntersectionKind2::BoundaryTouch
                            } else {
                                SpanIntersectionKind2::ProperCrossing
                            };
                            let mut budget = SpanQueryBudget::new(1);
                            let event = a.intersect_span(&b, &mut budget).unwrap();
                            assert_eq!(event.kind(), expected, "case {cases}");
                            compare_located_and_source_spans(&a, &b, expected);
                            if expected != SpanIntersectionKind2::Disjoint {
                                assert!(same(
                                    event.first_boundary().unwrap().parameter(),
                                    crossing_first
                                ));
                                assert!(same(event.point_on_second().unwrap(), crossing_second));
                                assert!(crossing_first.same_point(&crossing_second));
                            }
                            cases += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cases, 3840);
}
