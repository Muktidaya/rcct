// SPDX-License-Identifier: Apache-2.0
use rcct::gp::Point3;
use rcct::spatial::{
    FinitePoint3, Orientation3, RawPlane3, RawPlaneError3, Segment3, SegmentPlaneIntersection3,
};

fn p(x: f64, y: f64, z: f64) -> FinitePoint3 {
    FinitePoint3::try_new(Point3::new(x, y, z)).unwrap()
}
fn event_name(event: SegmentPlaneIntersection3) -> &'static str {
    match event {
        SegmentPlaneIntersection3::Disjoint => "disjoint",
        SegmentPlaneIntersection3::Start(_) => "start",
        SegmentPlaneIntersection3::End(_) => "end",
        SegmentPlaneIntersection3::Contained => "contained",
        SegmentPlaneIntersection3::ProperCrossing(_) => "crossing",
    }
}

#[test]
fn independent_fraction_oracle_and_reversals() {
    let mut count = 0;
    for row in include_str!("fixtures/segment_plane.tsv")
        .lines()
        .filter(|row| !row.starts_with('#'))
    {
        let fields: Vec<_> = row.split('\t').collect();
        assert_eq!(fields.len(), 16);
        let coordinate = |i: usize| f64::from_bits(u64::from_str_radix(fields[i], 16).unwrap());
        let point = |i: usize| p(coordinate(i), coordinate(i + 1), coordinate(i + 2));
        let [a, b, c, start, end] = [point(0), point(3), point(6), point(9), point(12)];
        let plane = RawPlane3::try_new(a, b, c);
        if fields[15] == "invalid" {
            assert!(
                matches!(plane, Err(RawPlaneError3::Collinear)),
                "row {count}"
            );
            count += 1;
            continue;
        }
        let plane = plane.unwrap();
        let reverse_plane = RawPlane3::try_new(a, c, b).unwrap();
        let segment = Segment3::try_new(start, end).unwrap();
        let reversed = Segment3::try_new(end, start).unwrap();
        let result = plane.intersect_segment(segment);
        assert_eq!(event_name(result), fields[15], "row {count}");
        assert_eq!(
            event_name(reverse_plane.intersect_segment(segment)),
            fields[15],
            "plane reversed row {count}"
        );
        let expected_reverse = match fields[15] {
            "start" => "end",
            "end" => "start",
            other => other,
        };
        assert_eq!(
            event_name(plane.intersect_segment(reversed)),
            expected_reverse,
            "segment reversed row {count}"
        );
        if let SegmentPlaneIntersection3::ProperCrossing(witness) = result {
            assert_eq!(witness.segment(), segment);
            assert_eq!(witness.plane().points(), [a, b, c]);
            assert!(matches!(
                (plane.side(start), plane.side(end)),
                (Orientation3::Positive, Orientation3::Negative)
                    | (Orientation3::Negative, Orientation3::Positive)
            ));
        }
        count += 1;
    }
    assert_eq!(count, 266);
}

#[test]
fn plane_validity_and_endpoint_bits() {
    let zero = p(0.0, 0.0, 0.0);
    let x = p(1.0, 0.0, 0.0);
    let y = p(0.0, 1.0, 0.0);
    assert!(matches!(
        RawPlane3::try_new(zero, x, p(2.0, 0.0, -0.0)),
        Err(RawPlaneError3::Collinear)
    ));
    assert!(matches!(
        RawPlane3::try_new(zero, zero, y),
        Err(RawPlaneError3::Collinear)
    ));
    // This plane has a degenerate xy projection but a nonzero xz projection.
    assert!(RawPlane3::try_new(zero, x, p(0.0, 0.0, 1.0)).is_ok());
    let plane = RawPlane3::try_new(zero, x, y).unwrap();
    let start = p(-0.0, 0.0, -0.0);
    let segment = Segment3::try_new(start, p(0.0, 0.0, 1.0)).unwrap();
    match plane.intersect_segment(segment) {
        SegmentPlaneIntersection3::Start(point) => {
            assert_eq!(point.get().x.to_bits(), (-0.0f64).to_bits());
            assert_eq!(point.get().z.to_bits(), (-0.0f64).to_bits());
        }
        other => panic!("unexpected event: {other:?}"),
    }
    let end = p(-0.0, -0.0, -0.0);
    let reverse = Segment3::try_new(p(0.0, 0.0, 1.0), end).unwrap();
    match plane.intersect_segment(reverse) {
        SegmentPlaneIntersection3::End(point) => {
            assert_eq!(point.get().x.to_bits(), end.get().x.to_bits());
            assert_eq!(point.get().y.to_bits(), end.get().y.to_bits());
            assert_eq!(point.get().z.to_bits(), end.get().z.to_bits());
        }
        other => panic!("unexpected event: {other:?}"),
    }
    assert_eq!(plane.points(), [zero, x, y]);
}
