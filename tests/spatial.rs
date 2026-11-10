// SPDX-License-Identifier: Apache-2.0
use rcct::gp::{Point3, UnitDirection3, Vector3};
use rcct::spatial::{
    Aabb3, FinitePoint3, Line3, Plane3, Segment3, SpatialError3, SpatialEvaluationError3,
};

fn p(x: f64, y: f64, z: f64) -> FinitePoint3 {
    FinitePoint3::try_new(Point3::new(x, y, z)).unwrap()
}
fn axis(x: f64, y: f64, z: f64) -> UnitDirection3 {
    UnitDirection3::try_from(Vector3::new(x, y, z)).unwrap()
}

#[test]
fn checked_points_and_segments_cover_extremes() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for point in [
            Point3::new(bad, 0., 0.),
            Point3::new(0., bad, 0.),
            Point3::new(0., 0., bad),
        ] {
            assert_eq!(
                FinitePoint3::try_new(point),
                Err(SpatialError3::NonFinitePoint)
            );
        }
    }
    let tiny = f64::from_bits(1);
    assert_eq!(
        Segment3::try_new(p(0., 0., 0.), p(-0., 0., -0.)),
        Err(SpatialError3::DegenerateSegment)
    );
    for (a, b) in [
        (p(1., 0., 0.), p(0., 1., 1.)),
        (p(0., 1., 0.), p(1., 0., 1.)),
        (p(0., 0., 1.), p(1., 1., 0.)),
    ] {
        assert_eq!(Aabb3::try_new(a, b), Err(SpatialError3::InvertedBounds));
    }
    let segment = Segment3::try_new(p(0., 0., 0.), p(tiny, 0., 0.)).unwrap();
    assert!(segment.bounds().contains(p(tiny, 0., 0.)));
    assert!(Aabb3::from_points(p(-f64::MAX, 0., 0.), p(f64::MAX, 0., 0.)).contains(p(0., 0., 0.)));
    assert_eq!(
        Aabb3::try_new(p(1., 0., 0.), p(0., 1., 1.)),
        Err(SpatialError3::InvertedBounds)
    );
}

#[test]
fn closed_box_contacts_and_integer_oracle() {
    let a = Aabb3::from_points(p(0., 0., 0.), p(2., 2., 2.));
    for (lo, hi, expected_lo, expected_hi) in [
        ((2., 0., 0.), (3., 2., 2.), (2., 0., 0.), (2., 2., 2.)),
        ((2., 2., 0.), (3., 3., 2.), (2., 2., 0.), (2., 2., 2.)),
        ((2., 2., 2.), (3., 3., 3.), (2., 2., 2.), (2., 2., 2.)),
    ] {
        let b = Aabb3::from_points(p(lo.0, lo.1, lo.2), p(hi.0, hi.1, hi.2));
        let expected = Aabb3::try_new(
            p(expected_lo.0, expected_lo.1, expected_lo.2),
            p(expected_hi.0, expected_hi.1, expected_hi.2),
        )
        .unwrap();
        assert_eq!(a.intersection(b), Some(expected));
        assert_eq!(a.intersection(b), b.intersection(a));
    }
    assert_eq!(
        a.intersection(Aabb3::from_points(p(3., 0., 0.), p(4., 1., 1.))),
        None
    );
    for x in -3..=3 {
        for y in -3..=3 {
            for z in -3..=3 {
                let point = p(x as f64, y as f64, z as f64);
                assert_eq!(
                    a.contains(point),
                    (0..=2).contains(&x) && (0..=2).contains(&y) && (0..=2).contains(&z)
                );
                let b = Aabb3::from_points(point, point);
                assert!(a.union(b).contains(point));
                assert_eq!(a.intersection(b).is_some(), a.contains(point));
            }
        }
    }
}

#[test]
fn rounded_line_and_plane_evaluations_are_explicitly_fallible() {
    let line = Line3::new(p(1., 2., 3.), axis(1., 0., 0.));
    assert_eq!(line.try_evaluate(2.), Ok(p(3., 2., 3.)));
    assert_eq!(
        line.try_evaluate(f64::NAN),
        Err(SpatialEvaluationError3::NonFiniteParameter)
    );
    for parameter in [f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            line.try_evaluate(parameter),
            Err(SpatialEvaluationError3::NonFiniteParameter)
        );
    }
    assert_eq!(
        Line3::new(p(f64::MAX, 0., 0.), axis(1., 0., 0.)).try_evaluate(f64::MAX),
        Err(SpatialEvaluationError3::NonFiniteResult)
    );
    let plane = Plane3::new(p(1., 2., 3.), axis(1., 0., 0.));
    assert_eq!(plane.try_signed_offset(p(4., 9., 9.)), Ok(3.));
    assert_eq!(
        Plane3::new(p(1., 2., 3.), axis(-1., 0., 0.)).try_signed_offset(p(4., 9., 9.)),
        Ok(-3.)
    );
    // Exact offset is zero, but an unrelated displacement component overflows.
    assert_eq!(
        Plane3::new(p(-f64::MAX, 0., 0.), axis(0., 1., 0.)).try_signed_offset(p(f64::MAX, 0., 0.)),
        Err(SpatialEvaluationError3::NonFiniteResult)
    );
    assert_eq!(
        Plane3::new(p(0., 0., 0.), axis(1., 1., -1.)).try_signed_offset(p(
            f64::MAX,
            f64::MAX,
            f64::MAX
        )),
        Err(SpatialEvaluationError3::NonFiniteResult)
    );
    let tiny = f64::from_bits(1);
    assert_eq!(
        Plane3::new(p(0., 0., 0.), axis(1., 0., 0.)).try_signed_offset(p(tiny, 0., 0.)),
        Ok(tiny)
    );
    // A nonzero mathematical offset can round down to zero.
    assert_eq!(
        Plane3::new(p(0., 0., 0.), axis(1., 4., 0.)).try_signed_offset(p(tiny, 0., 0.)),
        Ok(0.)
    );
}
