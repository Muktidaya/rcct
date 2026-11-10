// SPDX-License-Identifier: Apache-2.0
use rcct::planar::{
    LocatedPoint2, LocatedSpan2, Point2, Segment2, SegmentParameter2, SegmentSpan2,
    SpanLocationBudget, SpanLocationError, SpanQueryBudget, SpanSplitBudget, SpanSplitError,
    point_on_segment_witness, proper_crossing_witness,
};
use std::cmp::Ordering;

fn p(x: i64, y: i64) -> Point2 {
    Point2::try_new(x as f64, y as f64).unwrap()
}
fn s(a: (i64, i64), b: (i64, i64)) -> Segment2 {
    Segment2::try_new(p(a.0, a.1), p(b.0, b.1)).unwrap()
}
fn constructed(a: (i64, i64), b: (i64, i64), n: i64, d: i64) -> SegmentParameter2 {
    let source = s(a, b);
    if n == 0 {
        return point_on_segment_witness(source, source.start())
            .unwrap()
            .parameter();
    }
    if n == d {
        return point_on_segment_witness(source, source.end())
            .unwrap()
            .parameter();
    }
    let normal = (a.1 - b.1, b.0 - a.0);
    let cutter = s(
        (a.0 + normal.0 * n, a.1 + normal.1 * n),
        (b.0 - normal.0 * (d - n), b.1 - normal.1 * (d - n)),
    );
    proper_crossing_witness(source, cutter).unwrap().parameter()
}

#[test]
fn independent_rational_incidence_oracle() {
    let ratios = [(0, 1), (1, 5), (1, 3), (1, 2), (2, 3), (4, 5), (1, 1)];
    let mut state = 0x83f1_09a5_u64;
    let mut rand = |n: usize| {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((state >> 32) as usize) % n
    };
    for case in 0..2400 {
        let a = (rand(11) as i64 - 5, rand(11) as i64 - 5);
        let mut b = (rand(11) as i64 - 5, rand(11) as i64 - 5);
        if a == b {
            b.0 += 1;
        }
        let delta = (b.0 - a.0, b.1 - a.1);
        let (u, v) = match case % 5 {
            0 => (a, b),
            1 => (b, a),
            2 => (
                (a.0 - delta.0, a.1 - delta.1),
                (a.0 + 2 * delta.0, a.1 + 2 * delta.1),
            ),
            3 => (
                (a.0 - delta.1, a.1 + delta.0),
                (b.0 - delta.1, b.1 + delta.0),
            ),
            _ => (
                (rand(15) as i64 - 7, rand(15) as i64 - 7),
                (rand(15) as i64 - 7, rand(15) as i64 - 7),
            ),
        };
        let v = if u == v { (v.0 + 1, v.1) } else { v };
        let (n, d) = ratios[rand(ratios.len())];
        let point = constructed(a, b, n, d);
        // Independent i128 determinant and closed bounds on exact n/d point.
        let px = i128::from(a.0 * (d - n) + b.0 * n);
        let py = i128::from(a.1 * (d - n) + b.1 * n);
        let (ux, uy, vx, vy, den) = (
            i128::from(u.0),
            i128::from(u.1),
            i128::from(v.0),
            i128::from(v.1),
            i128::from(d),
        );
        let cross = (vx - ux) * (py - uy * den) - (vy - uy) * (px - ux * den);
        let expected = cross == 0
            && ux.min(vx) * den <= px
            && px <= ux.max(vx) * den
            && uy.min(vy) * den <= py
            && py <= uy.max(vy) * den;
        let mut budget = SpanLocationBudget::new(1);
        let located = LocatedPoint2::locate(s(u, v), point, &mut budget).unwrap();
        assert_eq!(
            located.is_some(),
            expected,
            "case {case}: {a:?} {b:?} {u:?} {v:?} {n}/{d}"
        );
        assert_eq!(budget.remaining(), 0);
        if let Some(located) = located {
            assert!(located.retained_parameter().same_point(&point));
        }
    }
}

#[test]
fn overlap_boundaries_can_be_located_and_split_on_both_supports() {
    let a = s((0, 0), (1, 0));
    let b = s((3, 0), (0, 0));
    let third = constructed((0, 0), (1, 0), 1, 3);
    let two_thirds = constructed((0, 0), (1, 0), 2, 3);
    let a_span = SegmentSpan2::try_new(third, two_thirds).unwrap();
    let b_span = SegmentSpan2::whole(b);
    let mut query = SpanQueryBudget::new(1);
    let overlap = a_span.intersect_span(&b_span, &mut query).unwrap();
    let first = overlap.first_boundary().unwrap().parameter();
    let last = overlap.last_boundary().unwrap().parameter();
    let mut locate = SpanLocationBudget::new(2);
    let first_on_b = LocatedPoint2::locate(b, first, &mut locate)
        .unwrap()
        .unwrap();
    let last_on_b = LocatedPoint2::locate(b, last, &mut locate)
        .unwrap()
        .unwrap();
    assert_eq!(
        last_on_b.compare_on_target(&first_on_b),
        Some(Ordering::Less)
    );
    assert!(first_on_b.retained_parameter().segment() == a);
    let mut split = SpanSplitBudget::new(2);
    let (_, right) = LocatedSpan2::whole(b)
        .split_at(last_on_b, &mut split)
        .unwrap();
    let (middle, _) = right.split_at(first_on_b, &mut split).unwrap();
    assert!(middle.start().retained_parameter().same_point(&two_thirds));
    assert!(middle.end().retained_parameter().same_point(&third));
    let vertical = Segment2::try_new(
        Point2::try_new(0.5, -1.).unwrap(),
        Point2::try_new(0.5, 1.).unwrap(),
    )
    .unwrap();
    let mut query = SpanQueryBudget::new(1);
    let crossing = middle
        .intersect_span(&LocatedSpan2::whole(vertical), &mut query)
        .unwrap();
    assert_eq!(
        crossing.kind(),
        rcct::planar::SpanIntersectionKind2::ProperCrossing
    );
    let first_point = crossing.first_boundary().unwrap().point();
    assert!(
        first_point
            .retained_parameter()
            .same_point(&crossing.point_on_second().unwrap().retained_parameter())
    );
    assert_eq!(locate.remaining(), 0);
    assert_eq!(split.remaining(), 0);
}

#[test]
fn minimum_subnormal_offset_is_not_snapped_to_target() {
    let tiny = f64::from_bits(1);
    let target = Segment2::try_new(
        Point2::try_new(0., 0.).unwrap(),
        Point2::try_new(1., 0.).unwrap(),
    )
    .unwrap();
    let source = Segment2::try_new(
        Point2::try_new(0., tiny).unwrap(),
        Point2::try_new(1., tiny).unwrap(),
    )
    .unwrap();
    let cutter = Segment2::try_new(
        Point2::try_new(0., 1.).unwrap(),
        Point2::try_new(1., -2.).unwrap(),
    )
    .unwrap();
    let point = proper_crossing_witness(source, cutter).unwrap().parameter();
    let mut budget = SpanLocationBudget::new(1);
    assert!(
        LocatedPoint2::locate(target, point, &mut budget)
            .unwrap()
            .is_none()
    );
    assert_eq!(budget.remaining(), 0);
}

#[test]
fn extreme_scale_locations_preserve_exact_incidence() {
    for (a, b) in [(-f64::MAX, f64::MAX), (0.0, f64::from_bits(1))] {
        let source = Segment2::try_new(
            Point2::try_new(a, 0.).unwrap(),
            Point2::try_new(b, 0.).unwrap(),
        )
        .unwrap();
        let reverse = Segment2::try_new(source.end(), source.start()).unwrap();
        let cutter = Segment2::try_new(
            Point2::try_new(a, 1.).unwrap(),
            Point2::try_new(b, -2.).unwrap(),
        )
        .unwrap();
        let point = proper_crossing_witness(source, cutter).unwrap().parameter();
        let mut budget = SpanLocationBudget::new(2);
        let located = LocatedPoint2::locate(reverse, point, &mut budget)
            .unwrap()
            .unwrap();
        assert!(point.same_point(&located.retained_parameter()));
        let offset = Segment2::try_new(
            Point2::try_new(a, f64::from_bits(1)).unwrap(),
            Point2::try_new(b, f64::from_bits(1)).unwrap(),
        )
        .unwrap();
        assert!(
            LocatedPoint2::locate(offset, point, &mut budget)
                .unwrap()
                .is_none()
        );
        assert_eq!(budget.remaining(), 0);
    }
}

#[test]
fn repeated_transfer_preserves_origin_and_failures_are_atomic() {
    let source = s((0, 0), (1, 0));
    let point = constructed((0, 0), (1, 0), 1, 3);
    let mut retained = point;
    let mut budget = SpanLocationBudget::new(24);
    for k in 1..=20 {
        let target = if k % 2 == 0 {
            s((-k, 0), (k, 0))
        } else {
            s((k, 0), (-k, 0))
        };
        let location = LocatedPoint2::locate(target, retained, &mut budget)
            .unwrap()
            .unwrap();
        assert!(location.retained_parameter().segment() == source);
        assert!(location.retained_parameter().same_point(&point));
        retained = location.retained_parameter();
    }
    assert!(
        LocatedPoint2::locate(s((0, 1), (1, 1)), point, &mut budget)
            .unwrap()
            .is_none()
    );
    assert!(
        LocatedPoint2::locate(s((1, 0), (2, 0)), point, &mut budget)
            .unwrap()
            .is_none()
    );
    assert_eq!(budget.remaining(), 2);
    let target = s((0, 0), (2, 0));
    let cut = LocatedPoint2::locate(target, point, &mut budget)
        .unwrap()
        .unwrap();
    let span = LocatedSpan2::whole(target);
    let mut split = SpanSplitBudget::new(0);
    assert!(matches!(
        span.split_at(cut, &mut split),
        Err(SpanSplitError::SplitLimitExceeded)
    ));
    assert_eq!(split.remaining(), 0);
    assert!(
        span.start().retained_parameter().same_point(
            &point_on_segment_witness(target, target.start())
                .unwrap()
                .parameter()
        )
    );
    let foreign = LocatedPoint2::locate(source, point, &mut budget)
        .unwrap()
        .unwrap();
    assert!(matches!(
        span.split_at(foreign, &mut split),
        Err(SpanSplitError::DifferentSource)
    ));
    assert!(matches!(
        LocatedPoint2::locate(target, point, &mut budget),
        Err(SpanLocationError::LocationLimitExceeded)
    ));
    assert_eq!(budget.remaining(), 0);
}
