// SPDX-License-Identifier: Apache-2.0
use rcct::planar::{
    Point2, ProperCrossing2, Segment2, endpoint_touch_witness, overlap_witness,
    proper_crossing_witness,
};
use std::cmp::Ordering;

fn point(x: f64, y: f64) -> Point2 {
    Point2::try_new(x, y).unwrap()
}
fn segment(a: (f64, f64), b: (f64, f64)) -> Segment2 {
    Segment2::try_new(point(a.0, a.1), point(b.0, b.1)).unwrap()
}
fn witness(u: f64, v: f64) -> ProperCrossing2 {
    proper_crossing_witness(
        segment((0.0, 0.0), (1.0, 0.0)),
        segment((0.0, u), (1.0, -v)),
    )
    .unwrap()
}

#[test]
fn orders_rational_parameters_and_detects_equal_noncanonical_fractions() {
    let third = witness(1.0, 2.0);
    let same_third = witness(2.0, 4.0);
    let half = witness(1.0, 1.0);
    assert_eq!(third.compare_on_first(&same_third), Some(Ordering::Equal));
    assert_eq!(third.compare_on_first(&half), Some(Ordering::Less));
    assert_eq!(half.compare_on_first(&third), Some(Ordering::Greater));
    assert_eq!(third.first_segment(), segment((0.0, 0.0), (1.0, 0.0)));
    assert_eq!(third.second_segment(), segment((0.0, 1.0), (1.0, -2.0)));
    let reversed =
        proper_crossing_witness(segment((1.0, 0.0), (0.0, 0.0)), third.second_segment()).unwrap();
    assert_eq!(third.compare_on_first(&reversed), None);
}

#[test]
fn proper_witness_excludes_other_intersection_classes() {
    let base = segment((0.0, 0.0), (1.0, 0.0));
    assert!(proper_crossing_witness(base, segment((2.0, 1.0), (2.0, -1.0))).is_none());
    assert!(proper_crossing_witness(base, segment((1.0, 0.0), (1.0, 1.0))).is_none());
    assert!(proper_crossing_witness(base, segment((0.5, 0.0), (2.0, 0.0))).is_none());
}

#[test]
fn compares_extreme_parameters_without_rounding() {
    let tiny = f64::from_bits(1);
    let max = f64::MAX;
    let before = witness(tiny, max);
    let after = witness(max, tiny);
    assert_eq!(before.compare_on_first(&after), Some(Ordering::Less));
    let near_max = f64::from_bits(max.to_bits() - 1);
    assert_eq!(
        witness(near_max, max).compare_on_first(&witness(max, near_max)),
        Some(Ordering::Less)
    );
}

#[test]
fn orders_512_wide_exponent_cases_against_fraction_oracle() {
    let mut count = 0;
    for line in include_str!("fixtures/crossing_order.tsv")
        .lines()
        .filter(|line| !line.starts_with('#'))
    {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 5);
        let v: Vec<_> = fields[..4]
            .iter()
            .map(|s| f64::from_bits(u64::from_str_radix(s, 16).unwrap()))
            .collect();
        let expected = match fields[4] {
            "-1" => Ordering::Less,
            "0" => Ordering::Equal,
            "1" => Ordering::Greater,
            _ => panic!("invalid sign"),
        };
        assert_eq!(
            witness(v[0], v[1]).compare_on_first(&witness(v[2], v[3])),
            Some(expected),
            "case {count}"
        );
        count += 1;
    }
    assert_eq!(count, 512);
}

#[test]
fn orders_general_integer_crossings_against_fraction_oracle() {
    let mut count = 0;
    for line in include_str!("fixtures/crossing_order_general.tsv")
        .lines()
        .filter(|line| !line.starts_with('#'))
    {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 13);
        let values: Vec<f64> = fields[..12]
            .iter()
            .map(|s| s.parse::<i32>().unwrap() as f64)
            .collect();
        let a = segment((values[0], values[1]), (values[2], values[3]));
        let b = segment((values[4], values[5]), (values[6], values[7]));
        let c = segment((values[8], values[9]), (values[10], values[11]));
        let first = proper_crossing_witness(a, b).unwrap();
        let second = proper_crossing_witness(a, c).unwrap();
        let expected = match fields[12] {
            "-1" => Ordering::Less,
            "0" => Ordering::Equal,
            "1" => Ordering::Greater,
            _ => panic!("invalid sign"),
        };
        assert_eq!(
            first.compare_on_first(&second),
            Some(expected),
            "case {count}"
        );
        count += 1;
    }
    assert_eq!(count, 256);
}

#[test]
fn endpoint_parameters_order_against_proper_crossings_at_extreme_scale() {
    let max = f64::MAX;
    let tiny = f64::from_bits(1);
    let first = segment((-max, 0.0), (max, 0.0));
    let middle = proper_crossing_witness(first, segment((0.0, -1.0), (0.0, 1.0))).unwrap();
    let touch = endpoint_touch_witness(first, segment((tiny, 0.0), (tiny, 1.0))).unwrap();
    assert_eq!(touch.point(), point(tiny, 0.0));
    assert_eq!(
        touch.parameter().compare_on_first(&middle.parameter()),
        Some(Ordering::Greater)
    );
    let reversed = segment((max, 0.0), (-max, 0.0));
    let reverse_touch =
        endpoint_touch_witness(reversed, segment((tiny, 0.0), (tiny, 1.0))).unwrap();
    assert_eq!(
        touch
            .parameter()
            .compare_on_first(&reverse_touch.parameter()),
        None
    );
}

#[test]
fn collinear_single_endpoint_contact_preserves_source_point() {
    let first = segment((0.0, 0.0), (1.0, 0.0));
    let touch = endpoint_touch_witness(first, segment((1.0, -0.0), (2.0, 0.0))).unwrap();
    assert_eq!(touch.point(), point(1.0, 0.0));
    assert_eq!(touch.first_segment(), first);
    assert_eq!(touch.second_segment(), segment((1.0, -0.0), (2.0, 0.0)));
    assert!(endpoint_touch_witness(first, segment((0.5, 0.0), (2.0, 0.0))).is_none());
}

#[test]
fn overlap_boundaries_are_exact_and_follow_first_segment_direction() {
    let max = f64::MAX;
    let tiny = f64::from_bits(1);
    let first = segment((-max, 0.0), (max, 0.0));
    let second = segment((max, -0.0), (tiny, 0.0));
    let overlap = overlap_witness(first, second).unwrap();
    assert_eq!(overlap.first_segment(), first);
    assert_eq!(overlap.second_segment(), second);
    assert_eq!(overlap.start_point(), point(tiny, 0.0));
    assert_eq!(overlap.end_point(), point(max, 0.0));
    let touch = endpoint_touch_witness(first, segment((tiny, 0.0), (tiny, 1.0))).unwrap();
    let crossing = proper_crossing_witness(first, segment((0.0, -1.0), (0.0, 1.0))).unwrap();
    assert_eq!(
        overlap
            .start_parameter()
            .compare_on_first(&touch.parameter()),
        Some(Ordering::Equal)
    );
    assert_eq!(
        overlap
            .start_parameter()
            .compare_on_first(&crossing.parameter()),
        Some(Ordering::Greater)
    );
    assert_eq!(
        overlap
            .start_parameter()
            .compare_on_first(&overlap.end_parameter()),
        Some(Ordering::Less)
    );
    let reverse = overlap_witness(segment((max, 0.0), (-max, 0.0)), second).unwrap();
    assert_eq!(reverse.start_point(), point(max, 0.0));
    assert_eq!(reverse.end_point(), point(tiny, 0.0));
    assert_eq!(
        overlap
            .start_parameter()
            .compare_on_first(&reverse.end_parameter()),
        None
    );
    assert!(overlap_witness(first, segment((max, 0.0), (max, 1.0))).is_none());
}
