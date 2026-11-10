// SPDX-License-Identifier: Apache-2.0
use rcct::gp::Point3;
use rcct::spatial::{CurveEvaluationError3, FinitePoint3, QuadraticBezier3};

fn point(x: f64, y: f64, z: f64) -> FinitePoint3 {
    FinitePoint3::try_new(Point3::new(x, y, z)).unwrap()
}

#[test]
fn parameter_validation_and_endpoint_bits() {
    let start = point(-0.0, f64::MAX, -f64::MAX);
    let end = point(0.0, -f64::MAX, f64::MAX);
    let curve = QuadraticBezier3::new(start, point(1.0, 2.0, 3.0), end);
    assert_eq!(curve.controls(), [start, point(1.0, 2.0, 3.0), end]);
    assert_eq!(
        curve.try_evaluate(-0.0).unwrap().get().x.to_bits(),
        (-0.0f64).to_bits()
    );
    assert_eq!(curve.try_evaluate(0.0), Ok(start));
    assert_eq!(
        curve.try_evaluate(1.0).unwrap().get().x.to_bits(),
        0.0f64.to_bits()
    );
    assert_eq!(curve.try_evaluate(1.0), Ok(end));
    for t in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            curve.try_evaluate(t),
            Err(CurveEvaluationError3::NonFiniteParameter)
        );
    }
    for t in [-f64::from_bits(1), f64::from_bits(1.0f64.to_bits() + 1)] {
        assert_eq!(
            curve.try_evaluate(t),
            Err(CurveEvaluationError3::OutOfRangeParameter)
        );
    }
}

#[test]
fn dyadic_polynomial_oracle_on_small_integer_controls() {
    // Independent Bernstein polynomial: all intermediates here are small
    // dyadics, so both the stated de Casteljau order and oracle are exact.
    for a in -4i32..=4 {
        for b in -4i32..=4 {
            for c in -4i32..=4 {
                let curve = QuadraticBezier3::new(
                    point(a as f64, 2.0 * a as f64, 1.0),
                    point(b as f64, 2.0 * b as f64, 1.0),
                    point(c as f64, 2.0 * c as f64, 1.0),
                );
                for q in 0i32..=4 {
                    let numerator = (4 - q) * (4 - q) * a + 2 * (4 - q) * q * b + q * q * c;
                    let x = numerator as f64 / 16.0;
                    let result = curve.try_evaluate(q as f64 / 4.0).unwrap();
                    assert_eq!(result.get(), Point3::new(x, 2.0 * x, 1.0));
                    assert!(curve.control_bounds().contains(result));
                }
            }
        }
    }
}

#[test]
fn degenerate_subnormal_and_extreme_controls() {
    let tiny = f64::from_bits(1);
    let repeated = point(tiny, 0.0, -tiny);
    let constant = QuadraticBezier3::new(repeated, repeated, repeated);
    assert_eq!(constant.try_evaluate(tiny), Ok(repeated));
    // Both half-subnormal products round to zero at the first level.
    let rounded = constant.try_evaluate(0.5).unwrap();
    assert_eq!(rounded.get().x, 0.0);
    assert!(!constant.control_bounds().contains(rounded));
    let negative_tiny = point(-tiny, 0.0, 0.0);
    let negative = QuadraticBezier3::new(negative_tiny, negative_tiny, negative_tiny);
    assert_eq!(
        negative.try_evaluate(0.5).unwrap().get().x.to_bits(),
        (-0.0f64).to_bits()
    );
    let negative_zero = point(-0.0, 0.0, 0.0);
    let zero_curve = QuadraticBezier3::new(negative_zero, negative_zero, negative_zero);
    assert_eq!(
        zero_curve.try_evaluate(0.5).unwrap().get().x.to_bits(),
        (-0.0f64).to_bits()
    );
    assert_eq!(constant.control_bounds().lower(), repeated);
    assert_eq!(constant.control_bounds().upper(), repeated);
    let extreme = QuadraticBezier3::new(
        point(f64::MAX, 0.0, 0.0),
        point(-f64::MAX, 0.0, 0.0),
        point(f64::MAX, 0.0, 0.0),
    );
    assert_eq!(extreme.try_evaluate(0.5), Ok(point(0.0, 0.0, 0.0)));
    let bounds = extreme.control_bounds();
    assert_eq!(bounds.lower().get().x, -f64::MAX);
    assert_eq!(bounds.upper().get().x, f64::MAX);
}

#[test]
fn independent_rational_rounded_operation_fixtures() {
    let text = include_str!("fixtures/bezier3.tsv");
    let mut count = 0usize;
    for line in text.lines().filter(|line| !line.starts_with('#')) {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 5);
        let parse = |s: &str| f64::from_bits(u64::from_str_radix(s, 16).unwrap());
        let curve = QuadraticBezier3::new(
            point(parse(fields[0]), 0.0, 0.0),
            point(parse(fields[1]), 0.0, 0.0),
            point(parse(fields[2]), 0.0, 0.0),
        );
        let result = curve.try_evaluate(parse(fields[3]));
        if fields[4] == "error" {
            assert_eq!(
                result,
                Err(CurveEvaluationError3::NonFiniteResult),
                "{line}"
            );
        } else {
            assert_eq!(
                result.unwrap().get().x.to_bits(),
                u64::from_str_radix(fields[4], 16).unwrap(),
                "{line}"
            );
        }
        count += 1;
    }
    assert_eq!(count, 262);
}
