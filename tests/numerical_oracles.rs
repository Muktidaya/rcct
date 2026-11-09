// SPDX-License-Identifier: Apache-2.0

use rcct::gp::{DirectionError, Point3, UnitDirection3, Vector3};

// The oracle never invokes rcct, hypot or a binary64 square root. See the
// generator and fixtures/README.md for provenance and operation-specific bounds.
#[test]
fn finite_operations_match_independent_real_arithmetic_oracles() {
    let mut counts = [0; 5];
    for line in include_str!("fixtures/numerical.tsv").lines() {
        if line.starts_with('#') {
            continue;
        }
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 14);
        let numbers: Vec<_> = fields[2..]
            .iter()
            .map(|s| f64::from_bits(u64::from_str_radix(s, 16).unwrap()))
            .collect();
        let a = Vector3::new(numbers[0], numbers[1], numbers[2]);
        let b = Vector3::new(numbers[3], numbers[4], numbers[5]);
        let (index, actual) = match fields[0] {
            "norm" => (0, vec![a.norm()]),
            "normalize" => {
                let u = UnitDirection3::try_from(a).unwrap().as_vector();
                (1, vec![u.x, u.y, u.z])
            }
            "dot" => (2, vec![a.dot(b)]),
            "cross" => {
                let c = a.cross(b);
                (3, vec![c.x, c.y, c.z])
            }
            "distance" => (
                4,
                vec![Point3::new(a.x, a.y, a.z).distance(Point3::new(b.x, b.y, b.z))],
            ),
            op => panic!("unknown fixture operation {op}"),
        };
        counts[index] += 1;
        for (component, value) in actual.into_iter().enumerate() {
            let expected = numbers[6 + component];
            let budget = numbers[9 + component];
            assert!(expected.is_finite() && budget.is_finite() && budget >= 0.0);
            assert!(
                value.is_finite() && (value - expected).abs() <= budget,
                "{} {} component {component}: actual={value:e}, expected={expected:e}, budget={budget:e}",
                fields[0],
                fields[1]
            );
        }
    }
    assert_eq!(counts, [21, 21, 27, 27, 32]);
}

#[test]
fn finite_intermediate_overflow_is_not_an_exact_predicate() {
    // The exact dot product is zero, but products overflow before cancellation.
    let a = Vector3::new(f64::MAX, f64::MAX, 0.0);
    assert!(a.dot(Vector3::new(2.0, -2.0, 0.0)).is_nan());
    // Exactly parallel finite vectors can have NaN cross components.
    assert!(a.cross(a).z.is_nan());
    assert!(a.norm().is_infinite());
    assert!(UnitDirection3::try_from(a).unwrap().as_vector().is_finite());
    assert!(
        Point3::new(-f64::MAX, 0.0, 0.0)
            .distance(Point3::new(f64::MAX, 0.0, 0.0))
            .is_infinite()
    );
    // Underflow is permitted; no nonzero exact-predicate promise is made.
    let tiny = Vector3::new(f64::from_bits(1), 0.0, 0.0);
    assert_eq!(tiny.dot(tiny), 0.0);
}

#[test]
fn exceptional_values_are_classified_separately_from_accuracy() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for a in [
            Vector3::new(bad, 0.0, 0.0),
            Vector3::new(0.0, bad, 0.0),
            Vector3::new(0.0, 0.0, bad),
        ] {
            assert_eq!(UnitDirection3::try_from(a), Err(DirectionError::NonFinite));
            assert!(a.dot(Vector3::ZERO).is_nan());
            let cross = a.cross(Vector3::ZERO);
            assert!(cross.x.is_nan() || cross.y.is_nan() || cross.z.is_nan());
            if bad.is_nan() {
                assert!(a.norm().is_nan());
            } else {
                assert_eq!(a.norm(), f64::INFINITY);
            }
        }
    }
    // Infinity dominates NaN in hypot, including an intermediate infinity.
    for a in [
        Vector3::new(f64::INFINITY, f64::NAN, 0.0),
        Vector3::new(f64::NAN, 0.0, f64::NEG_INFINITY),
        Vector3::new(f64::MAX, f64::MAX, f64::NAN),
    ] {
        assert_eq!(a.norm(), f64::INFINITY);
    }
    let p = Point3::new(f64::INFINITY, 0.0, 0.0);
    assert!(p.distance(p).is_nan());
}

#[test]
fn signed_zero_and_subnormal_direction_semantics_are_explicit() {
    for signs in 0..8 {
        let zero = |bit| if signs & (1 << bit) == 0 { 0.0 } else { -0.0 };
        let a = Vector3::new(zero(0), zero(1), zero(2));
        assert_eq!(a.norm().to_bits(), 0.0f64.to_bits());
        assert_eq!(UnitDirection3::try_from(a), Err(DirectionError::ZeroVector));
    }
    for scale in [f64::from_bits(1), f64::MIN_POSITIVE, 1.0, f64::MAX] {
        let a = Vector3::new(scale, -0.0, 0.0);
        let u = UnitDirection3::try_from(a).unwrap().as_vector();
        assert_eq!(u.x, 1.0);
        assert_eq!(u.y.to_bits(), (-0.0f64).to_bits());
        assert_eq!(u.z.to_bits(), 0.0f64.to_bits());
    }
    let u = UnitDirection3::try_from(Vector3::new(f64::MAX, -f64::from_bits(1), 0.0))
        .unwrap()
        .as_vector();
    assert_eq!(u.y.to_bits(), (-0.0f64).to_bits());
}

#[test]
fn componentwise_arithmetic_preserves_ieee_classification_and_zero_signs() {
    let zeros = Vector3::new(0.0, -0.0, 0.0);
    for result in [-zeros, zeros * -1.0, -1.0 * zeros, zeros / -1.0] {
        assert_eq!(result.x.to_bits(), (-0.0f64).to_bits());
        assert_eq!(result.y.to_bits(), 0.0f64.to_bits());
        assert_eq!(result.z.to_bits(), (-0.0f64).to_bits());
    }
    for divisor in [0.0, -0.0] {
        let result = Vector3::new(1.0, -1.0, 0.0) / divisor;
        assert!(result.x.is_infinite() && result.y.is_infinite());
        assert_eq!(result.x.is_sign_negative(), divisor.is_sign_negative());
        assert_ne!(result.y.is_sign_negative(), divisor.is_sign_negative());
        assert!(result.z.is_nan());
    }
    let extreme = Vector3::new(f64::MAX, -f64::MAX, f64::from_bits(1));
    let overflow = extreme * 2.0;
    assert_eq!(overflow.x, f64::INFINITY);
    assert_eq!(overflow.y, f64::NEG_INFINITY);
    assert_eq!(overflow.z.to_bits(), 2);
    for underflow in [extreme * 0.5, extreme / 2.0] {
        assert_eq!(underflow.z.to_bits(), 0);
    }
    let overflow_sum = extreme + extreme;
    let overflow_difference = extreme - -extreme;
    assert_eq!(overflow_sum, overflow);
    assert_eq!(overflow_difference, overflow);

    // Rotate exceptional operands through each axis. Assert classifications,
    // not NaN payloads or signs, which are not portable arithmetic contracts.
    for axis in 0..3 {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut coordinates = [0.0; 3];
            coordinates[axis] = bad;
            let v = Vector3::new(coordinates[0], coordinates[1], coordinates[2]);
            let p = Point3::new(v.x, v.y, v.z);
            let component = |v: Vector3| [v.x, v.y, v.z][axis];
            let classify = |actual: f64, expected: f64| {
                if expected.is_nan() {
                    assert!(actual.is_nan());
                } else {
                    assert_eq!(actual, expected);
                }
            };
            for actual in [v + Vector3::ZERO, v - Vector3::ZERO, p - Point3::ORIGIN] {
                classify(component(actual), bad);
            }
            for actual in [p + Vector3::ZERO, p - Vector3::ZERO, Point3::ORIGIN + v] {
                classify([actual.x, actual.y, actual.z][axis], bad);
            }
            let opposite = Point3::ORIGIN - v;
            classify([opposite.x, opposite.y, opposite.z][axis], -bad);
            classify(component(Vector3::ZERO - v), -bad);
            assert!(component(v - v).is_nan());
            assert!(component(p - p).is_nan());
            assert!(component(v * 0.0).is_nan());
        }
    }
}
