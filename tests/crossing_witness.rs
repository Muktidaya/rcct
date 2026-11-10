// SPDX-License-Identifier: Apache-2.0
use rcct::planar::{
    Point2, ProperCrossing2, Segment2, endpoint_touch_witness, overlap_witness,
    point_on_segment_witness, proper_crossing_witness,
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
    let point_witness = point_on_segment_witness(first, point(tiny, 0.0)).unwrap();
    assert_eq!(
        point_witness
            .parameter()
            .compare_on_first(&touch.parameter()),
        Some(Ordering::Equal)
    );
    assert!(point_on_segment_witness(first, point(tiny, tiny)).is_none());
    assert_eq!(touch.point(), point(tiny, 0.0));
    let tight = touch.parameter().enclosure();
    assert_eq!(tight.lower(), 0.5);
    assert_eq!(tight.upper().to_bits(), 0.5_f64.to_bits() + 1);
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

#[test]
fn parameter_enclosures_are_tight_for_exact_and_extreme_values() {
    let third = witness(1.0, 2.0).parameter().enclosure();
    assert_eq!(third.lower().to_bits(), 0x3fd5_5555_5555_5555);
    assert_eq!(third.upper().to_bits(), 0x3fd5_5555_5555_5556);
    let half = witness(1.0, 1.0).parameter().enclosure();
    assert_eq!((half.lower(), half.upper()), (0.5, 0.5));
    let tiny = f64::from_bits(1);
    let near_zero = witness(tiny, f64::MAX).parameter().enclosure();
    assert_eq!(near_zero.lower().to_bits(), 0);
    assert_eq!(near_zero.upper().to_bits(), 1);
    let near_one = witness(f64::MAX, tiny).parameter().enclosure();
    assert_eq!(near_one.lower().to_bits(), 1.0_f64.to_bits() - 1);
    assert_eq!(near_one.upper(), 1.0);
    let base = segment((0.0, 0.0), (1.0, 0.0));
    let zero = endpoint_touch_witness(base, segment((0.0, 0.0), (0.0, 1.0)))
        .unwrap()
        .parameter()
        .enclosure();
    let one = endpoint_touch_witness(base, segment((1.0, 0.0), (1.0, 1.0)))
        .unwrap()
        .parameter()
        .enclosure();
    assert_eq!((zero.lower(), zero.upper()), (0.0, 0.0));
    assert_eq!((one.lower(), one.upper()), (1.0, 1.0));
}

#[test]
fn parameter_enclosures_match_independent_fraction_oracle() {
    let mut count = 0;
    for line in include_str!("fixtures/parameter_enclosures.tsv")
        .lines()
        .filter(|line| !line.starts_with('#'))
    {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 4);
        let values: Vec<u64> = fields
            .iter()
            .map(|s| u64::from_str_radix(s, 16).unwrap())
            .collect();
        let result = witness(f64::from_bits(values[0]), f64::from_bits(values[1]))
            .parameter()
            .enclosure();
        assert_eq!(
            (result.lower().to_bits(), result.upper().to_bits()),
            (values[2], values[3]),
            "case {count}"
        );
        assert!(values[3] - values[2] <= 1);
        count += 1;
    }
    assert_eq!(count, 772);
}

#[test]
fn general_crossing_enclosures_match_independent_fraction_oracle() {
    let mut count = 0;
    for line in include_str!("fixtures/parameter_enclosures_general.tsv")
        .lines()
        .filter(|line| !line.starts_with('#'))
    {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 10);
        let v: Vec<f64> = fields[..8]
            .iter()
            .map(|s| s.parse::<i32>().unwrap() as f64)
            .collect();
        let first = segment((v[0], v[1]), (v[2], v[3]));
        let second = segment((v[4], v[5]), (v[6], v[7]));
        let enclosure = proper_crossing_witness(first, second)
            .unwrap()
            .parameter()
            .enclosure();
        let expected = (
            u64::from_str_radix(fields[8], 16).unwrap(),
            u64::from_str_radix(fields[9], 16).unwrap(),
        );
        assert_eq!(
            (enclosure.lower().to_bits(), enclosure.upper().to_bits()),
            expected,
            "case {count}"
        );
        count += 1;
    }
    assert_eq!(count, 256);
}

#[test]
fn crossing_coordinates_are_tightly_enclosed_without_rounding_parameter_first() {
    let third = proper_crossing_witness(
        segment((0.0, 0.0), (1.0, 1.0)),
        segment((0.0, 1.0), (1.0, -1.0)),
    )
    .unwrap()
    .point_enclosure();
    for c in [third.x(), third.y()] {
        assert_eq!(c.lower().to_bits(), 0x3fd5_5555_5555_5555);
        assert_eq!(c.upper().to_bits(), 0x3fd5_5555_5555_5556);
    }
    let tiny = f64::from_bits(1);
    let exact_tiny = proper_crossing_witness(
        segment((-f64::MAX, 0.0), (f64::MAX, 0.0)),
        segment((tiny, -1.0), (tiny, 1.0)),
    )
    .unwrap()
    .point_enclosure();
    assert_eq!(exact_tiny.x().lower().to_bits(), 1);
    assert_eq!(exact_tiny.x().upper().to_bits(), 1);
    assert_eq!(exact_tiny.y().lower().to_bits(), 0);
    assert_eq!(exact_tiny.y().upper().to_bits(), 0);
    let negative_half_subnormal = proper_crossing_witness(
        segment((0.0, 0.0), (-tiny, 1.0)),
        segment((-1.0, 0.5), (1.0, 0.5)),
    )
    .unwrap()
    .point_enclosure();
    assert_eq!(
        negative_half_subnormal.x().lower().to_bits(),
        (-tiny).to_bits()
    );
    assert_eq!(
        negative_half_subnormal.x().upper().to_bits(),
        (-0.0_f64).to_bits()
    );
    let positive_half_subnormal = proper_crossing_witness(
        segment((0.0, 0.0), (tiny, 1.0)),
        segment((-1.0, 0.5), (1.0, 0.5)),
    )
    .unwrap()
    .point_enclosure();
    assert_eq!(positive_half_subnormal.x().lower().to_bits(), 0);
    assert_eq!(positive_half_subnormal.x().upper().to_bits(), 1);
}

#[test]
fn crossing_coordinate_enclosures_match_independent_fraction_oracle() {
    let mut count = 0;
    for line in include_str!("fixtures/crossing_coordinate_enclosures.tsv")
        .lines()
        .filter(|line| !line.starts_with('#'))
    {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 12);
        let values: Vec<u64> = fields
            .iter()
            .map(|s| u64::from_str_radix(s, 16).unwrap())
            .collect();
        let v: Vec<f64> = values[..8].iter().map(|&b| f64::from_bits(b)).collect();
        let a = segment((v[0], v[1]), (v[2], v[3]));
        let b = segment((v[4], v[5]), (v[6], v[7]));
        let enclosure = proper_crossing_witness(a, b).unwrap().point_enclosure();
        let swapped = proper_crossing_witness(b, a).unwrap().point_enclosure();
        let reversed = proper_crossing_witness(segment((v[2], v[3]), (v[0], v[1])), b)
            .unwrap()
            .point_enclosure();
        let bit_bounds = |e: rcct::planar::CrossingEnclosure2| {
            (
                e.x().lower().to_bits(),
                e.x().upper().to_bits(),
                e.y().lower().to_bits(),
                e.y().upper().to_bits(),
            )
        };
        assert_eq!(
            bit_bounds(swapped),
            bit_bounds(enclosure),
            "swapped case {count}"
        );
        assert_eq!(
            bit_bounds(reversed),
            bit_bounds(enclosure),
            "reversed case {count}"
        );
        assert_eq!(
            (
                enclosure.x().lower().to_bits(),
                enclosure.x().upper().to_bits(),
                enclosure.y().lower().to_bits(),
                enclosure.y().upper().to_bits()
            ),
            (values[8], values[9], values[10], values[11]),
            "case {count}"
        );
        count += 1;
    }
    assert_eq!(count, 512);
}
