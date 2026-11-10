// SPDX-License-Identifier: Apache-2.0
use rcct::planar::{
    ConstructedPredicateBudget, ConstructedPredicateError, ExactPredicatePoint2, Orientation2,
    Point2, Segment2, orient_constructed2d, proper_crossing_witness,
};

fn p(x: f64, y: f64) -> Point2 {
    Point2::try_new(x, y).unwrap()
}
fn s(a: (f64, f64), b: (f64, f64)) -> Segment2 {
    Segment2::try_new(p(a.0, a.1), p(b.0, b.1)).unwrap()
}

#[test]
fn budget_failure_is_atomic_and_success_is_charged() {
    let cost = ConstructedPredicateBudget::orientation_cost();
    let a = ExactPredicatePoint2::Input(p(0.0, 0.0));
    let b = ExactPredicatePoint2::Input(p(1.0, 0.0));
    let c = ExactPredicatePoint2::Input(p(0.0, 1.0));
    let mut insufficient = ConstructedPredicateBudget::new(cost - 1);
    assert_eq!(
        orient_constructed2d(a, b, c, &mut insufficient),
        Err(ConstructedPredicateError::WorkLimitExceeded)
    );
    assert_eq!(insufficient.remaining(), cost - 1);
    let mut budget = ConstructedPredicateBudget::new(cost);
    assert_eq!(
        orient_constructed2d(a, b, c, &mut budget),
        Ok(Orientation2::Counterclockwise)
    );
    assert_eq!(budget.remaining(), 0);
    assert_eq!(
        orient_constructed2d(a, b, c, &mut budget),
        Err(ConstructedPredicateError::WorkLimitExceeded)
    );
    assert_eq!(budget.remaining(), 0);
}

#[test]
fn equivalent_constructed_points_remain_exactly_collinear() {
    let a = s((0.0, 0.0), (1.0, 1.0));
    let b = s((0.0, 1.0), (1.0, -1.0));
    let x = proper_crossing_witness(a, b).unwrap().exact_point();
    let y = proper_crossing_witness(b, a).unwrap().exact_point();
    let snapshot = x.clone();
    let mut insufficient = ConstructedPredicateBudget::new(0);
    assert_eq!(
        orient_constructed2d(
            ExactPredicatePoint2::Crossing(&x),
            ExactPredicatePoint2::Crossing(&y),
            ExactPredicatePoint2::Input(p(0.0, 0.0)),
            &mut insufficient
        ),
        Err(ConstructedPredicateError::WorkLimitExceeded)
    );
    assert_eq!(insufficient.remaining(), 0);
    assert!(x.same_point(&snapshot));
    let mut budget =
        ConstructedPredicateBudget::new(ConstructedPredicateBudget::orientation_cost());
    assert_eq!(
        orient_constructed2d(
            ExactPredicatePoint2::Crossing(&x),
            ExactPredicatePoint2::Crossing(&y),
            ExactPredicatePoint2::Input(p(f64::MAX, -f64::MAX)),
            &mut budget
        ),
        Ok(Orientation2::Collinear)
    );
}

#[test]
fn constructed_orientation_distinguishes_overlapping_enclosures_and_extremes() {
    let base = s((0.0, 0.0), (1.0, 0.0));
    let exact_third = proper_crossing_witness(base, s((0.0, 1.0), (1.0, -2.0)))
        .unwrap()
        .exact_point();
    let rounded = f64::from_bits(0x3fd5_5555_5555_5555);
    let nearby = proper_crossing_witness(base, s((rounded, -1.0), (rounded, 1.0)))
        .unwrap()
        .exact_point();
    assert_eq!(
        exact_third.enclosure().x().lower(),
        nearby.enclosure().x().lower()
    );
    let cost = ConstructedPredicateBudget::orientation_cost();
    let mut budget = ConstructedPredicateBudget::new(cost * 2);
    assert_eq!(
        orient_constructed2d(
            ExactPredicatePoint2::Crossing(&exact_third),
            ExactPredicatePoint2::Crossing(&nearby),
            ExactPredicatePoint2::Input(p(0.0, 1.0)),
            &mut budget
        ),
        Ok(Orientation2::Clockwise)
    );
    let m = f64::MAX;
    assert_eq!(
        orient_constructed2d(
            ExactPredicatePoint2::Input(p(-m, -m)),
            ExactPredicatePoint2::Input(p(m, -m)),
            ExactPredicatePoint2::Input(p(-m, m)),
            &mut budget
        ),
        Ok(Orientation2::Counterclockwise)
    );
    assert_eq!(budget.remaining(), 0);
}

#[test]
fn mixed_constructed_orientations_match_fraction_oracle() {
    let mut count = 0;
    for line in include_str!("fixtures/constructed_orientation.tsv")
        .lines()
        .filter(|line| !line.starts_with('#'))
    {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 28);
        let block = |index: usize| {
            let offset = index * 9;
            let v: Vec<f64> = fields[offset + 1..offset + 9]
                .iter()
                .map(|s| f64::from_bits(u64::from_str_radix(s, 16).unwrap()))
                .collect();
            let raw = p(v[0], v[1]);
            let exact = proper_crossing_witness(
                s((v[0], v[1]), (v[2], v[3])),
                s((v[4], v[5]), (v[6], v[7])),
            )
            .unwrap()
            .exact_point();
            (raw, exact)
        };
        let points = [block(0), block(1), block(2)];
        let select = |index: usize| {
            if fields[index * 9] == "I" {
                ExactPredicatePoint2::Input(points[index].0)
            } else {
                ExactPredicatePoint2::Crossing(&points[index].1)
            }
        };
        let expected = match fields[27] {
            "-1" => Orientation2::Clockwise,
            "0" => Orientation2::Collinear,
            "1" => Orientation2::Counterclockwise,
            _ => panic!("bad sign"),
        };
        let mut budget =
            ConstructedPredicateBudget::new(ConstructedPredicateBudget::orientation_cost());
        assert_eq!(
            orient_constructed2d(select(0), select(1), select(2), &mut budget),
            Ok(expected),
            "case {count}"
        );
        assert_eq!(budget.remaining(), 0);
        count += 1;
    }
    assert_eq!(count, 256);
}
