// SPDX-License-Identifier: Apache-2.0
//! Exact convex readmission of computed boundaries, including a subdivided box and Empty.

#[path = "support/cuboid.rs"]
mod cuboid;
#[path = "support/oriented_boundary.rs"]
mod oriented_boundary;

use cuboid::Cuboid;
use oriented_boundary::assert_oriented_faces;
use rcct::polyhedral::{
    ConvexSolid, Operation3, PolyhedralWork, SolidBoundary, boolean_rectilinear3, boolean3,
};

fn work() -> PolyhedralWork<'static> {
    PolyhedralWork::new(100_000_000, 128 * 1024 * 1024)
}

fn convex_boolean(a: &ConvexSolid, b: &ConvexSolid, op: Operation3) -> SolidBoundary {
    boolean3(a, b, op, &mut work()).unwrap()
}

fn assert_box(result: &SolidBoundary, expected: &Cuboid, volume: (i128, i128)) {
    assert_eq!(result.volume(), volume);
    assert_eq!(result.shell_count(), 1);
    assert_oriented_faces(result, expected.faces());
}

#[test]
fn subdivided_convex_intersection_normalizes_and_enters_a_second_boolean() {
    // The adjacent boxes form [0,1] × [0,2] × [0,1] with a seam at x=1/3.
    let left = Cuboid::scaled([0, 0, 0], [1, 6, 3], 3).solid();
    let right = Cuboid::scaled([1, 0, 0], [3, 6, 3], 3).solid();
    let joined = convex_boolean(&left, &right, Operation3::Union);
    let cutter = Cuboid::scaled([-1, 3, 0], [2, 9, 3], 3).solid();
    let cutter_boundary = convex_boolean(&cutter, &ConvexSolid::empty(), Operation3::Union);
    let overlap = boolean_rectilinear3(
        &joined,
        &cutter_boundary,
        Operation3::Intersection,
        &mut work(),
    )
    .unwrap();
    let expected = Cuboid::scaled([0, 3, 0], [2, 6, 3], 3);
    assert_box(&overlap, &expected, (2, 3));

    // The *computed* top cap has an internal edge with two opposite uses.
    // Count those uses directly before asking convex admission to remove it.
    let seam_start = [(1, 3), (1, 1), (1, 1)];
    let seam_end = [(1, 3), (2, 1), (1, 1)];
    let mut uses = [0, 0];
    for face in 0..overlap.face_count() {
        if overlap.face_plane(face).unwrap().coefficients() == [0, 0, 1, -1] {
            let ids = overlap.face_vertices(face).unwrap();
            for (&i, &j) in ids.iter().zip(ids.iter().cycle().skip(1)) {
                let endpoints = [overlap.vertex(i).unwrap(), overlap.vertex(j).unwrap()];
                if endpoints == [seam_start, seam_end] {
                    uses[0] += 1;
                } else if endpoints == [seam_end, seam_start] {
                    uses[1] += 1;
                }
            }
        }
    }
    assert_eq!(uses, [1, 1]);

    let convex = overlap.as_convex(&mut work()).unwrap();
    let normalized = convex_boolean(&convex, &ConvexSolid::empty(), Operation3::Union);
    assert_box(&normalized, &expected, (2, 3));
    assert_eq!((normalized.face_count(), normalized.vertex_count()), (6, 8));

    // Subtract the left portion. The exterior plane x=2/3 survives reuse.
    let remainder = convex_boolean(&convex, &left, Operation3::Difference);
    assert_box(&remainder, &Cuboid::scaled([1, 3, 0], [2, 6, 3], 3), (1, 3));
}

#[test]
fn computed_empty_intersection_is_readmitted_and_reused_in_union() {
    let a = Cuboid::new([0, 0, 0], [1, 1, 1]).solid();
    let expected = Cuboid::scaled([4, 0, 0], [5, 3, 3], 3);
    let b = expected.solid();
    let empty = convex_boolean(&a, &b, Operation3::Intersection);
    assert!(empty.is_empty());
    assert_eq!(empty.volume(), (0, 1));
    assert_eq!(
        (
            empty.shell_count(),
            empty.face_count(),
            empty.vertex_count()
        ),
        (0, 0, 0)
    );
    let admitted = empty.as_convex(&mut work()).unwrap();
    assert!(admitted.is_empty());
    let restored = convex_boolean(&admitted, &b, Operation3::Union);
    assert_box(&restored, &expected, (1, 3));
}
