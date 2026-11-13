// SPDX-License-Identifier: Apache-2.0
use rcct::gp::Point3;
use rcct::spatial::{FinitePoint3, Orientation3, orient3d};

fn p(x: f64, y: f64, z: f64) -> FinitePoint3 {
    FinitePoint3::try_new(Point3::new(x, y, z)).unwrap()
}
fn inverse(orientation: Orientation3) -> Orientation3 {
    match orientation {
        Orientation3::Negative => Orientation3::Positive,
        Orientation3::Coplanar => Orientation3::Coplanar,
        Orientation3::Positive => Orientation3::Negative,
    }
}

#[test]
fn exact_raw_input_oracle_covers_all_finite_exponent_fields() {
    let mut count = 0usize;
    for row in include_str!("fixtures/orientation3.tsv")
        .lines()
        .filter(|row| !row.starts_with('#'))
    {
        let fields: Vec<_> = row.split('\t').collect();
        assert_eq!(fields.len(), 13);
        let coordinate = |i: usize| f64::from_bits(u64::from_str_radix(fields[i], 16).unwrap());
        let point = |i: usize| p(coordinate(i), coordinate(i + 1), coordinate(i + 2));
        let expected = match fields[12] {
            "positive" => Orientation3::Positive,
            "negative" => Orientation3::Negative,
            "coplanar" => Orientation3::Coplanar,
            other => panic!("unexpected orientation: {other}"),
        };
        let (a, b, c, d) = (point(0), point(3), point(6), point(9));
        assert_eq!(orient3d(a, b, c, d), expected, "fixture row {count}");
        assert_eq!(
            orient3d(a, c, b, d),
            inverse(expected),
            "swapped row {count}"
        );
        count += 1;
    }
    assert_eq!(count, 2352);
}

#[test]
fn orientation_convention_and_degenerate_inputs() {
    let origin = p(0.0, 0.0, 0.0);
    let x = p(1.0, 0.0, 0.0);
    let y = p(0.0, 1.0, 0.0);
    let z = p(0.0, 0.0, 1.0);
    assert_eq!(orient3d(origin, x, y, z), Orientation3::Positive);
    assert_eq!(orient3d(origin, y, x, z), Orientation3::Negative);
    assert_eq!(orient3d(origin, x, y, origin), Orientation3::Coplanar);
    assert_eq!(
        orient3d(origin, x, y, p(-0.0, 0.0, -0.0)),
        Orientation3::Coplanar
    );
    let tiny = f64::from_bits(1);
    assert_eq!(
        orient3d(
            origin,
            p(tiny, 0.0, 0.0),
            p(0.0, tiny, 0.0),
            p(0.0, 0.0, tiny)
        ),
        Orientation3::Positive
    );
}

#[test]
fn representable_translation_scaling_and_permutation_metamorphisms() {
    let transform = |point: [i32; 3], offset: [i32; 3], scale: i32| {
        p(
            f64::from(scale * point[0] + offset[0]),
            f64::from(scale * point[1] + offset[1]),
            f64::from(scale * point[2] + offset[2]),
        )
    };
    for seed in -4..=4 {
        let base = [[seed, 0, 0], [1, 2, -1], [-2, 1, 3], [0, -1, 2]];
        let [a, b, c, d] = base.map(|v| transform(v, [0, 0, 0], 1));
        let result = orient3d(a, b, c, d);
        assert_eq!(orient3d(b, c, a, d), result);
        assert_eq!(orient3d(b, a, c, d), inverse(result));
        for scale in [1, 2, 8] {
            let [u, v, w, x] = base.map(|p| transform(p, [13, -9, 7], scale));
            assert_eq!(orient3d(u, v, w, x), result);
        }
    }
}

#[test]
fn signed_products_and_extreme_exact_cancellation() {
    let zero = p(0.0, 0.0, 0.0);
    let b = p(0.0, 1.0, 0.0);
    let c = p(0.0, 0.0, -1.0);
    // The x component of b × c is negative. A negative x difference
    // produces a positive term; changing only that sign negates it.
    assert_eq!(
        orient3d(zero, b, c, p(-1.0, 0.0, 0.0)),
        Orientation3::Positive
    );
    assert_eq!(
        orient3d(zero, b, c, p(1.0, 0.0, 0.0)),
        Orientation3::Negative
    );
    let max = f64::MAX;
    // The fourth point equals c-b as an exact mathematical vector; every
    // coordinate is finite although intermediate floating products overflow.
    assert_eq!(
        orient3d(zero, p(max, max, 0.0), p(max, 0.0, max), p(0.0, -max, max)),
        Orientation3::Coplanar
    );
}
