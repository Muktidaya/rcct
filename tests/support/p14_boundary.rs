// SPDX-License-Identifier: Apache-2.0
use std::collections::{BTreeMap, BTreeSet};

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
    a.checked_mul(b).expect("small P14 fixture product")
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

fn sheared([u, v, w]: Triple) -> Point {
    [(2 * u + v, 2), (v, 1), (w, 1)]
}

fn expected_outline(cells: &[[i32; 2]]) -> Outline {
    let voxels: BTreeSet<_> = cells
        .iter()
        .map(|&[u, v]| [i128::from(u), i128::from(v), 0])
        .collect();
    assert_eq!(voxels.len(), cells.len(), "duplicate expected cell");
    let mut outline = Outline::new();
    for &voxel in &voxels {
        for axis in 0..3 {
            for sign in [-1, 1] {
                let mut neighbor = voxel;
                neighbor[axis] += sign;
                if voxels.contains(&neighbor) {
                    continue;
                }
                let mut corner = voxel;
                if sign > 0 {
                    corner[axis] += 1;
                }
                let level = corner[axis];
                let plane = match axis {
                    0 => [2, -1, 0, -2 * level], // 2x - y = 2u
                    1 => [0, 1, 0, -level],
                    _ => [0, 0, 1, -level],
                }
                .map(|n| sign * n);
                let mut face = [[0, 0], [1, 0], [1, 1], [0, 1]].map(|[s, t]| {
                    let mut p = corner;
                    p[(axis + 1) % 3] += s;
                    p[(axis + 2) % 3] += t;
                    sheared(p)
                });
                if sign < 0 {
                    face.reverse();
                }
                record_loop(&mut outline, plane, &face);
            }
        }
    }
    outline.retain(|_, count| *count != 0);
    outline
}

/// Checks both caps and all walls of the unit-height sheared cells. On each
/// oriented support, conforming internal seams cancel and collinear edge
/// subdivisions telescope. The remaining outline determines the bounded
/// oriented face region independently of the returned patch and vertex IDs.
pub fn assert_boundary(result: &SolidBoundary, cells: &[[i32; 2]]) {
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
    assert_eq!(
        actual,
        expected_outline(cells),
        "oriented boundary of {cells:?}"
    );
    assert_eq!(result.volume(), (cells.len() as i128, 1));
}
