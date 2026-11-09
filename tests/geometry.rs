// SPDX-License-Identifier: Apache-2.0

use rcct::gp::{DirectionError, Point3, UnitDirection3, Vector3};

#[test]
fn affine_operations_preserve_point_vector_distinction() {
    let p = Point3::new(1.0, 2.0, 3.0);
    let v = Vector3::new(3.0, 4.0, 0.0);
    let q = p + v;
    assert_eq!(q, Point3::new(4.0, 6.0, 3.0));
    assert_eq!(q - p, v);
    assert_eq!(q - v, p);
    assert_eq!(p.distance(q), 5.0);
    assert_eq!(q.distance(p), 5.0);
    assert_eq!(p.distance(p), 0.0);
    assert_eq!(Point3::default(), Point3::ORIGIN);
    assert_eq!(Vector3::default(), Vector3::ZERO);
}

#[test]
fn vector_arithmetic_and_right_handed_products() {
    let x = Vector3::new(1.0, 0.0, 0.0);
    let y = Vector3::new(0.0, 1.0, 0.0);
    let z = Vector3::new(0.0, 0.0, 1.0);
    assert_eq!(x.cross(y), z);
    assert_eq!(y.cross(x), -z);
    assert_eq!(x.cross(x), Vector3::ZERO);
    assert_eq!(x.dot(y), 0.0);
    assert_eq!(x.dot(x), 1.0);
    assert_eq!(x + y - y, x);
    assert_eq!(x * 2.0, 2.0 * x);
    assert_eq!((x * 2.0) / 2.0, x);
    assert_eq!(Vector3::new(2.0, 3.0, 6.0).norm(), 7.0);
}

#[test]
fn norm_avoids_unnecessary_overflow_and_underflow() {
    for magnitude in [1e-200, 1e200] {
        let v = Vector3::new(3.0 * magnitude, 4.0 * magnitude, 0.0);
        assert!((v.norm() / magnitude - 5.0).abs() < 1e-14);
    }
    assert_eq!(Vector3::new(f64::MAX, 0.0, 0.0).norm(), f64::MAX);
    assert!(Vector3::new(f64::MAX, f64::MAX, 0.0).norm().is_infinite());
    assert_eq!(
        Vector3::new(f64::from_bits(1), 0.0, 0.0).norm(),
        f64::from_bits(1)
    );
}

#[test]
fn unchecked_primitives_preserve_ieee_semantics() {
    assert!(Point3::ORIGIN.is_finite());
    assert!(Vector3::ZERO.is_finite());
    assert!(!Point3::new(0.0, f64::NAN, 0.0).is_finite());
    assert!(!Vector3::new(0.0, 0.0, f64::INFINITY).is_finite());
    assert!(Vector3::new(f64::NAN, 0.0, 0.0).norm().is_nan());
    assert!(
        Vector3::new(f64::NAN, f64::INFINITY, 0.0)
            .norm()
            .is_infinite()
    );
    assert!(
        Vector3::new(f64::NAN, 0.0, f64::INFINITY)
            .norm()
            .is_infinite()
    );
    // The first hypot can overflow before the NaN z component is evaluated.
    assert!(
        Vector3::new(f64::MAX, f64::MAX, f64::NAN)
            .norm()
            .is_infinite()
    );
    assert!((Vector3::ZERO / 0.0).x.is_nan());
    assert!((Vector3::new(1.0, 0.0, 0.0) / 0.0).x.is_infinite());
    assert!(
        Point3::new(-f64::MAX, 0.0, 0.0)
            .distance(Point3::new(f64::MAX, 0.0, 0.0))
            .is_infinite()
    );
    assert_ne!(
        Point3::new(f64::NAN, 0.0, 0.0),
        Point3::new(f64::NAN, 0.0, 0.0)
    );
}

#[test]
fn directions_reject_zero_and_every_nonfinite_coordinate() {
    for zero in [0.0, -0.0] {
        assert_eq!(
            UnitDirection3::try_from(Vector3::new(zero, -zero, zero)),
            Err(DirectionError::ZeroVector)
        );
    }
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for v in [
            Vector3::new(bad, 0.0, 0.0),
            Vector3::new(0.0, bad, 0.0),
            Vector3::new(0.0, 0.0, bad),
        ] {
            assert_eq!(UnitDirection3::try_from(v), Err(DirectionError::NonFinite));
        }
    }
}

#[test]
fn directions_normalize_extreme_finite_inputs() {
    for magnitude in [
        f64::from_bits(1),
        f64::MIN_POSITIVE,
        1e-200,
        1.0,
        1e200,
        f64::MAX,
    ] {
        for v in [
            Vector3::new(magnitude, 0.0, 0.0),
            Vector3::new(0.0, -magnitude, 0.0),
            Vector3::new(0.0, 0.0, magnitude),
            Vector3::new(magnitude, -magnitude, magnitude),
        ] {
            let unit = UnitDirection3::try_from(v).unwrap().as_vector();
            assert!(unit.is_finite());
            assert!((unit.norm() - 1.0).abs() <= 4.0 * f64::EPSILON);
            for (a, b) in [(v.x, unit.x), (v.y, unit.y), (v.z, unit.z)] {
                assert_eq!(a.signum(), b.signum());
            }
        }
    }
    let mixed = UnitDirection3::try_from(Vector3::new(f64::MAX, f64::from_bits(1), 0.0)).unwrap();
    assert_eq!(mixed.as_vector(), Vector3::new(1.0, 0.0, 0.0));
}

#[test]
fn normalization_preserves_orientation_and_ignores_positive_scale() {
    let v = Vector3::new(3.0, 4.0, 0.0);
    let a = UnitDirection3::try_from(v).unwrap().as_vector();
    assert!((a.x - 0.6).abs() < 1e-15);
    assert!((a.y - 0.8).abs() < 1e-15);
    assert_eq!(a, UnitDirection3::try_from(v * 2.0).unwrap().as_vector());
    assert_eq!(-a, UnitDirection3::try_from(-v).unwrap().as_vector());
}

#[test]
fn direction_errors_are_standard_errors() {
    fn is_error(_: &dyn std::error::Error) {}
    for error in [DirectionError::ZeroVector, DirectionError::NonFinite] {
        is_error(&error);
        assert!(!error.to_string().is_empty());
    }
}
