// SPDX-License-Identifier: Apache-2.0
use std::collections::BTreeMap;

use rcct::polyhedral::SolidBoundary;

type Point = [(i128, i128); 3];
type Plane = [i128; 4];
type Triple = [i128; 3];

// The key identifies an oriented face support, an unoriented line in that
// support's chart, and a point on the line. A directed segment contributes
// +1 at its start and -1 at its end. These counts determine the directed
// coverage on a finite line from its zero value outside the endpoints.
type Outline = BTreeMap<(Plane, Triple, Triple), i32>;

fn product(a: i128, b: i128) -> i128 {
    a.checked_mul(b).expect("small boundary fixture product")
}

fn primitive(value: Triple) -> Triple {
    let mut divisor = 0;
    for n in value {
        let mut r = n.checked_abs().unwrap();
        while r != 0 {
            (divisor, r) = (r, divisor % r);
        }
    }
    assert_ne!(divisor, 0, "degenerate homogeneous point or line");
    value.map(|v| v / divisor)
}

fn project(plane: Plane, point: Point) -> Triple {
    let [(x, xd), (y, yd), (z, zd)] = point;
    let h = [
        product(x, product(yd, zd)),
        product(y, product(xd, zd)),
        product(z, product(xd, yd)),
        product(xd, product(yd, zd)),
    ];
    let residual = plane
        .into_iter()
        .zip(h)
        .fold(0i128, |sum, (a, b)| sum.checked_add(product(a, b)).unwrap());
    assert_eq!(residual, 0, "face vertex off its declared support");
    let axis = plane[..3].iter().position(|&n| n != 0).unwrap();
    primitive([h[(axis + 1) % 3], h[(axis + 2) % 3], h[3]])
}

fn line_through(a: Triple, b: Triple) -> Triple {
    // Homogeneous cross product gives the line through two rational points.
    let line = primitive(std::array::from_fn(|i| {
        let (j, k) = ((i + 1) % 3, (i + 2) % 3);
        product(a[j], b[k])
            .checked_sub(product(a[k], b[j]))
            .unwrap()
    }));
    if line < [0; 3] {
        line.map(|n| -n)
    } else {
        line
    }
}

fn record_loop(outline: &mut Outline, plane: Plane, points: &[Point]) {
    assert!(points.len() >= 3);
    let mut from = project(plane, *points.last().unwrap());
    for &point in points {
        let to = project(plane, point);
        let line = line_through(from, to);
        *outline.entry((plane, line, from)).or_default() += 1;
        *outline.entry((plane, line, to)).or_default() -= 1;
        from = to;
    }
}

/// Compare the exact finite oriented region on every support. Coplanar seams
/// cancel, and splitting an edge leaves the same directed line coverage.
pub fn assert_oriented_faces(
    result: &SolidBoundary,
    expected_faces: impl IntoIterator<Item = (Plane, Vec<Point>)>,
) {
    let mut actual = Outline::new();
    for face in 0..result.face_count() {
        let plane = result.face_plane(face).unwrap().coefficients();
        let points: Vec<_> = result
            .face_vertices(face)
            .unwrap()
            .iter()
            .map(|&id| result.vertex(id).unwrap())
            .collect();
        record_loop(&mut actual, plane, &points);
    }
    actual.retain(|_, count| *count != 0);
    let mut expected = Outline::new();
    for (plane, points) in expected_faces {
        record_loop(&mut expected, plane, &points);
    }
    expected.retain(|_, count| *count != 0);
    assert_eq!(actual, expected, "oriented face regions on every support");
}
