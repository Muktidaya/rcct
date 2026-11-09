// SPDX-License-Identifier: Apache-2.0

// Frozen v0.1.0 public surface smoke test. tools/check_package.py compiles and
// runs this identical source as a separate consumer of both v0.1.0 and the
// extracted candidate package. This is source compatibility evidence, not an
// exhaustive semver check or a promise of bit-identical floating-point results.
use rcct::gp::{DirectionError, Point3, UnitDirection3, Vector3};
use std::fmt::Debug;

const POINT: Point3 = Point3::new(1.0, 2.0, 3.0);
const VECTOR: Vector3 = Vector3::new(1.0, 0.0, 0.0);
const ORIGIN: Point3 = Point3::ORIGIN;
const ZERO: Vector3 = Vector3::ZERO;

// A const function body checks the const-call contract even though constructing
// a checked direction is deliberately not a const operation in v0.1.0.
const fn components(direction: UnitDirection3) -> Vector3 {
    direction.as_vector()
}

fn value_traits<T: Clone + Copy + Debug + PartialEq>(value: T) {
    let copied = value;
    assert_eq!(value, copied);
    assert!(!format!("{value:?}").is_empty());
}

#[test]
fn public_fields_constructors_methods_and_operators() {
    let mut p = Point3 {
        x: 1.0,
        y: 2.0,
        z: 3.0,
    };
    let mut v = Vector3 {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    };
    p.x = POINT.x;
    p.y = POINT.y;
    p.z = POINT.z;
    v.x = VECTOR.x;
    v.y = VECTOR.y;
    v.z = VECTOR.z;
    value_traits(p);
    value_traits(v);
    assert_eq!(Point3::default(), ORIGIN);
    assert_eq!(Vector3::default(), ZERO);
    assert!(p.is_finite());
    assert!(v.is_finite());
    let translated: Point3 = p + v;
    let restored: Point3 = translated - v;
    let delta: Vector3 = translated - p;
    assert_eq!(restored, p);
    assert_eq!(delta, v);
    assert_eq!(p.distance(translated), 1.0);
    let sum: Vector3 = v + v;
    let difference: Vector3 = sum - v;
    let negative: Vector3 = -v;
    let right_scaled: Vector3 = v * 2.0_f64;
    let left_scaled: Vector3 = 2.0_f64 * v;
    let quotient: Vector3 = sum / 2.0_f64;
    assert_eq!(difference, v);
    assert_eq!(negative + v, ZERO);
    assert_eq!(sum, right_scaled);
    assert_eq!(sum, left_scaled);
    assert_eq!(quotient, v);
    assert_eq!(v.dot(v), 1.0);
    assert_eq!(
        v.cross(Vector3::new(0.0, 1.0, 0.0)),
        Vector3::new(0.0, 0.0, 1.0)
    );
    assert_eq!(v.norm(), 1.0);
}

#[test]
fn checked_direction_and_exhaustive_error_surface() {
    let result: Result<UnitDirection3, DirectionError> = VECTOR.try_into();
    let direction = result.unwrap();
    value_traits(direction);
    assert_eq!(components(direction), VECTOR);
    assert_eq!(
        UnitDirection3::try_from(ZERO),
        Err(DirectionError::ZeroVector)
    );
    assert_eq!(
        UnitDirection3::try_from(Vector3::new(f64::NAN, 0.0, 0.0)),
        Err(DirectionError::NonFinite)
    );
    fn error_traits<T: std::error::Error + Eq>(_: T) {}
    for error in [DirectionError::NonFinite, DirectionError::ZeroVector] {
        value_traits(error);
        error_traits(error);
        let expected = match error {
            DirectionError::NonFinite => "a direction requires finite components",
            DirectionError::ZeroVector => "a zero vector has no direction",
        };
        assert_eq!(error.to_string(), expected);
        let erased: &dyn std::error::Error = &error;
        assert!(erased.source().is_none());
    }
}
