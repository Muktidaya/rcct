// SPDX-License-Identifier: Apache-2.0
//! P14 checks that relisting a solid's faces or choosing new loop origins
//! preserves its Boolean results, including the location of each difference.
#[path = "support/p14_boundary.rs"]
mod boundary;

use boundary::assert_boundary;
use rcct::polyhedral::{
    ConvexSolid, IndexedFace, Operation3, Plane3h, PolyhedralWork, SolidBoundary,
    boolean_prismatic3, boolean3,
};

fn work() -> PolyhedralWork<'static> {
    PolyhedralWork::new(100_000_000, 128 * 1024 * 1024)
}

// Unit-height prisms in (u,v,w), sheared into world coordinates by
// (x,y,z) = (u + v/2,v,w). Viewed from above in the original frame:
//
//       u=0   1   2
// v=2         +---+
//             | B |
// v=1     +---+---+
//         | A |A&B|
// v=0     +---+---+
struct Prism {
    u: [i64; 2],
    v: [i64; 2],
}

const A: Prism = Prism {
    u: [0, 2],
    v: [0, 1],
};
const B: Prism = Prism {
    u: [1, 2],
    v: [0, 2],
};

impl Prism {
    fn planes(&self) -> [Plane3h; 6] {
        [
            [-2, 1, 0, 2 * self.u[0]],
            [2, -1, 0, -2 * self.u[1]],
            [0, -1, 0, self.v[0]],
            [0, 1, 0, -self.v[1]],
            [0, 0, -1, 0],
            [0, 0, 1, -1],
        ]
        .map(|row| Plane3h::from_coefficients(row).unwrap())
    }

    fn convex(&self, reorder: usize) -> ConvexSolid {
        let mut planes = self.planes();
        permute(&mut planes, reorder);
        ConvexSolid::from_halfspaces(&planes, &mut work()).unwrap()
    }

    fn indexed(&self, reorder: usize) -> SolidBoundary {
        let [u0, u1] = self.u.map(i128::from);
        let [v0, v1] = self.v.map(i128::from);
        let mut points = Vec::new();
        for w in [0, 1] {
            for [u, v] in [[u0, v0], [u1, v0], [u1, v1], [u0, v1]] {
                points.push([(2 * u + v, 2), (v, 1), (w, 1)]);
            }
        }
        let [left, right, front, back, bottom, top] = self.planes();
        // The diagonal in the top cap is a tessellation seam. It contributes
        // no exterior outline on the top plane.
        let mut patches = [
            (left, vec![0, 4, 7, 3]),
            (right, vec![1, 2, 6, 5]),
            (front, vec![0, 1, 5, 4]),
            (back, vec![3, 7, 6, 2]),
            (bottom, vec![0, 3, 2, 1]),
            (top, vec![4, 5, 6]),
            (top, vec![4, 6, 7]),
        ];
        if reorder != 0 {
            for (index, (_, ids)) in patches.iter_mut().enumerate() {
                let offset = 1 + (index + reorder) % (ids.len() - 1);
                ids.rotate_left(offset);
            }
            permute(&mut patches, reorder);
        }
        let faces = patches.each_ref().map(|(plane, vertices)| IndexedFace {
            plane: *plane,
            vertices,
        });
        let solid = SolidBoundary::from_indexed_faces(&points, &faces, &mut work()).unwrap();
        for (index, face) in faces.iter().enumerate() {
            assert_eq!(solid.face_plane(index), Some(face.plane));
            assert_eq!(solid.face_vertices(index), Some(face.vertices));
        }
        solid
    }
}

fn permute<T>(items: &mut [T], offset: usize) {
    if offset != 0 {
        items.reverse();
        items.rotate_left(offset);
    }
}

fn check_orders<T>(
    variants_a: [T; 2],
    variants_b: [T; 2],
    apply: impl Fn(&T, &T, Operation3) -> SolidBoundary,
) {
    let union = [[0, 0], [1, 0], [1, 1]];
    let intersection = [[1, 0]];
    let only_a = [[0, 0]];
    let only_b = [[1, 1]];
    for a in &variants_a {
        for b in &variants_b {
            for (left, right, difference) in [(a, b, &only_a), (b, a, &only_b)] {
                assert_boundary(&apply(left, right, Operation3::Union), &union);
                assert_boundary(&apply(left, right, Operation3::Intersection), &intersection);
                assert_boundary(&apply(left, right, Operation3::Difference), difference);
            }
        }
    }
}

#[test]
fn permuted_halfspaces_preserve_oriented_boolean_boundaries() {
    check_orders(
        [A.convex(0), A.convex(1)],
        [B.convex(0), B.convex(2)],
        |a, b, op| boolean3(a, b, op, &mut work()).unwrap(),
    );
}

#[test]
fn permuted_indexed_faces_and_rotated_loops_preserve_oriented_boolean_boundaries() {
    check_orders(
        [A.indexed(0), A.indexed(1)],
        [B.indexed(0), B.indexed(2)],
        |a, b, op| boolean_prismatic3(a, b, op, &mut work()).unwrap(),
    );
}
