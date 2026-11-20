// SPDX-License-Identifier: Apache-2.0
use rcct::polyhedral::{ConvexSolid, Operation3, Plane3h, PolyhedralWork, boolean3};

#[path = "support/oriented_boundary.rs"]
mod oriented_boundary;

type Point = [i64; 3];
type Face = ([i64; 4], Vec<Point>);

#[derive(Clone, Copy)]
enum Frame {
    Base,
    Reflected,
}

impl Frame {
    // (x,y,z) = ((3u+v+1)/2, v−1/3, 1/4−w). Its linear determinant is −3/2.
    fn point(self, [u, v, w]: Point) -> [(i128, i128); 3] {
        match self {
            Self::Base => [u, v, w].map(|p| (i128::from(p), 1)),
            Self::Reflected => [
                (i128::from(3 * u + v + 1), 2),
                (i128::from(3 * v - 1), 3),
                (i128::from(1 - 4 * w), 4),
            ],
        }
    }

    // Substitute u=(2x−y−4/3)/3, v=y+1/3, w=1/4−z in the
    // halfspace. Multiplication by 36 clears all positive denominators.
    fn plane(self, [a, b, c, d]: [i64; 4]) -> Plane3h {
        let row = match self {
            Self::Base => [a, b, c, d],
            Self::Reflected => [
                24 * a,
                36 * b - 12 * a,
                -36 * c,
                -16 * a + 12 * b + 9 * c + 36 * d,
            ],
        };
        Plane3h::from_coefficients(row).unwrap()
    }

    fn admit(self, rows: &[[i64; 4]]) -> ConvexSolid {
        let planes: Vec<_> = rows.iter().map(|&row| self.plane(row)).collect();
        ConvexSolid::from_halfspaces(&planes, &mut work()).unwrap()
    }
}

fn work() -> PolyhedralWork<'static> {
    PolyhedralWork::new(100_000_000, 128 * 1024 * 1024)
}

// Cyclic (axis,j,k) order preserves orientation. Coordinates in the j,k chart
// below are counterclockwise as viewed from the positive end of the axis.
fn on_axis(axis: usize, level: i64, s: i64, t: i64) -> Point {
    let mut p = [0; 3];
    p[axis] = level;
    p[(axis + 1) % 3] = s;
    p[(axis + 2) % 3] = t;
    p
}

fn expected_faces(op: Operation3) -> Vec<Face> {
    // A is the unit cube. B is the positive-octant tetrahedron with coordinate
    // sum ≤ 2. C = A\B is the tetrahedral corner around (1,1,1).
    let mut faces = Vec::new();
    let diagonal = [1, 1, 1, -2];
    for axis in 0..3 {
        // At level 0, A's unit square lies inside B's triangle of side 2.
        let low: &[[i64; 2]] = match op {
            Operation3::Union => &[[0, 0], [2, 0], [0, 2]],
            Operation3::Intersection => &[[0, 0], [1, 0], [1, 1], [0, 1]],
            Operation3::Difference => &[],
        };
        if !low.is_empty() {
            let mut plane = [0; 4];
            plane[axis] = -1;
            let points = low
                .iter()
                .rev()
                .map(|&[s, t]| on_axis(axis, 0, s, t))
                .collect();
            faces.push((plane, points));
        }

        // At level 1, the line s+t=1 divides the square. Its lower triangle
        // bounds the intersection. The upper triangle bounds C and the union.
        let high = match op {
            Operation3::Intersection => [[0, 0], [1, 0], [0, 1]],
            Operation3::Union | Operation3::Difference => [[1, 0], [1, 1], [0, 1]],
        };
        let mut plane = [0, 0, 0, -1];
        plane[axis] = 1;
        faces.push((plane, high.map(|[s, t]| on_axis(axis, 1, s, t)).to_vec()));

        if op == Operation3::Union {
            // On B's diagonal face, the unit cube covers the central triangle.
            // Each remaining corner at one of B's three axis tips is exposed.
            faces.push((
                diagonal,
                vec![
                    on_axis(axis, 2, 0, 0),
                    on_axis(axis, 1, 1, 0),
                    on_axis(axis, 1, 0, 1),
                ],
            ));
        }
    }
    let mut center = vec![[0, 1, 1], [1, 0, 1], [1, 1, 0]];
    match op {
        Operation3::Union => {}
        Operation3::Intersection => faces.push((diagonal, center)),
        Operation3::Difference => {
            center.reverse();
            faces.push((diagonal.map(|c| -c), center));
        }
    }
    faces
}

#[test]
fn rational_affine_map_preserves_boolean_boundaries_and_scales_volume() {
    let cube = [
        [-1, 0, 0, 0],
        [1, 0, 0, -1],
        [0, -1, 0, 0],
        [0, 1, 0, -1],
        [0, 0, -1, 0],
        [0, 0, 1, -1],
    ];
    let simplex = [[-1, 0, 0, 0], [0, -1, 0, 0], [0, 0, -1, 0], [1, 1, 1, -2]];
    for frame in [Frame::Base, Frame::Reflected] {
        let (a, b) = (frame.admit(&cube), frame.admit(&simplex));
        // C has unit coordinate legs and volume 1/6. The intersection is
        // A minus C; the union joins B (volume 4/3) and C along their face.
        for (op, base_volume, mapped_volume) in [
            (Operation3::Union, (3, 2), (9, 4)),
            (Operation3::Intersection, (5, 6), (5, 4)),
            (Operation3::Difference, (1, 6), (1, 4)),
        ] {
            let result = boolean3(&a, &b, op, &mut work()).unwrap();
            let volume = match frame {
                Frame::Base => base_volume,
                Frame::Reflected => mapped_volume,
            };
            assert_eq!(result.volume(), volume);
            assert_eq!(result.shell_count(), 1);
            let expected = expected_faces(op).into_iter().map(|(plane, mut points)| {
                // A negative determinant reverses the winding of mapped loops.
                if matches!(frame, Frame::Reflected) {
                    points.reverse();
                }
                (
                    frame.plane(plane).coefficients(),
                    points.into_iter().map(|p| frame.point(p)).collect(),
                )
            });
            oriented_boundary::assert_oriented_faces(&result, expected);
        }
    }
}
