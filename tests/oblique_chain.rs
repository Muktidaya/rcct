// SPDX-License-Identifier: Apache-2.0
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::AtomicBool;

use rcct::polyhedral::{
    ConvexSolid, IndexedFace, Operation3, Plane3h, PolyhedralError, PolyhedralWork, SolidBoundary,
    boolean_prismatic3, boolean3,
};

fn work() -> PolyhedralWork<'static> {
    PolyhedralWork::new(100_000_000, 128 * 1024 * 1024)
}

fn convex(rows: &[[i64; 4]]) -> ConvexSolid {
    let planes: Vec<_> = rows
        .iter()
        .map(|&row| Plane3h::from_coefficients(row).unwrap())
        .collect();
    ConvexSolid::from_halfspaces(&planes, &mut work()).unwrap()
}

// Apply the determinant-one shear (u,v,w) -> (u+v/2,v,w) to a unit-height
// box. The supplied footprint bounds are numerators over the same denominator.
fn sheared_box(lo: [i64; 2], hi: [i64; 2], den: i64) -> ConvexSolid {
    convex(&[
        [-2 * den, den, 0, 2 * lo[0]],
        [2 * den, -den, 0, -2 * hi[0]],
        [0, -den, 0, lo[1]],
        [0, den, 0, -hi[1]],
        [0, 0, -1, 0],
        [0, 0, 1, -1],
    ])
}

fn boundary(solid: &ConvexSolid) -> SolidBoundary {
    boolean3(solid, &ConvexSolid::empty(), Operation3::Union, &mut work()).unwrap()
}

fn sheared_l() -> SolidBoundary {
    boolean3(
        &sheared_box([0, 0], [2, 1], 1),
        &sheared_box([0, 0], [1, 2], 1),
        Operation3::Union,
        &mut work(),
    )
    .unwrap()
}

fn apply(a: &SolidBoundary, b: &SolidBoundary, op: Operation3) -> SolidBoundary {
    boolean_prismatic3(a, b, op, &mut work()).unwrap()
}

fn cut_l(l: &SolidBoundary) -> SolidBoundary {
    let cutter = boundary(&sheared_box([1, 0], [6, 6], 3));
    apply(l, &cutter, Operation3::Intersection)
}

fn check_indexed_boundary(solid: &SolidBoundary) {
    let points: Vec<_> = (0..solid.vertex_count())
        .map(|i| solid.vertex(i).unwrap())
        .collect();
    let faces: Vec<_> = (0..solid.face_count())
        .map(|i| IndexedFace {
            plane: solid.face_plane(i).unwrap(),
            vertices: solid.face_vertices(i).unwrap(),
        })
        .collect();
    let admitted = SolidBoundary::from_indexed_faces(&points, &faces, &mut work()).unwrap();
    assert_eq!(admitted.volume(), solid.volume());
    assert_eq!(admitted.shell_count(), 1);
}

// Compare the complete oriented outline of the z=1 cap, independent of its
// tessellation. Opposite uses cancel on internal seams. The remaining edges
// must form exactly one cycle; collinear subdivision vertices can be skipped.
fn assert_top_outline(solid: &SolidBoundary, corners: &[[i128; 2]], denominator: i128) {
    let mut edges = BTreeSet::new();
    for i in 0..solid.face_count() {
        if solid.face_plane(i).unwrap().coefficients() == [0, 0, 1, -1] {
            let ids = solid.face_vertices(i).unwrap();
            for (&a, &b) in ids.iter().zip(ids.iter().cycle().skip(1)) {
                if !edges.remove(&(b, a)) {
                    assert!(edges.insert((a, b)));
                }
            }
        }
    }
    let edge_count = edges.len();
    let mut successors: BTreeMap<_, _> = edges.into_iter().collect();
    assert_eq!(successors.len(), edge_count);
    let start = *successors.keys().next().expect("nonempty top cap");
    let mut id = start;
    let mut ring = Vec::new();
    loop {
        let [(x, xd), (y, yd), _] = solid.vertex(id).unwrap();
        ring.push([x * yd, y * xd, xd * yd]);
        id = successors.remove(&id).expect("closed cap outline");
        if id == start {
            break;
        }
    }
    assert!(successors.is_empty(), "unexpected additional cap outline");
    let mut turns = Vec::new();
    for i in 0..ring.len() {
        let [a, b, c] = [i, i + 1, i + 2].map(|j| ring[j % ring.len()]);
        // Signed doubled triangle area times the positive homogeneous scales.
        let turn = a[0] * (b[1] * c[2] - b[2] * c[1])
            + a[1] * (b[2] * c[0] - b[0] * c[2])
            + a[2] * (b[0] * c[1] - b[1] * c[0]);
        if turn != 0 {
            turns.push(b);
        }
    }
    let matches = |point: [i128; 3], corner: [i128; 2]| {
        point[0] * denominator == corner[0] * point[2]
            && point[1] * denominator == corner[1] * point[2]
    };
    assert_eq!(turns.len(), corners.len());
    let first = turns
        .iter()
        .position(|&p| matches(p, corners[0]))
        .expect("first expected corner appears on boundary");
    for (i, &corner) in corners.iter().enumerate() {
        assert!(matches(turns[(first + i) % turns.len()], corner));
    }
}

#[test]
fn sheared_nonconvex_result_retains_the_entire_notch_after_a_second_cut() {
    let l = sheared_l();
    assert_eq!(l.volume(), (3, 1));
    assert_eq!(
        l.as_convex(&mut work()).unwrap_err(),
        PolyhedralError::UnsupportedNonconvex
    );
    let clipped = cut_l(&l);
    assert_eq!(clipped.volume(), (7, 3));
    // The sheared notch corner is (3/2,1); the adjacent cap segments go
    // west from (5/2,1) and northeast to (2,2).
    assert_top_outline(
        &clipped,
        &[[2, 0], [12, 0], [15, 6], [9, 6], [12, 12], [8, 12]],
        6,
    );
    check_indexed_boundary(&clipped);
}

#[test]
fn independently_angled_cutter_accepts_the_previous_result_directly() {
    let clipped = cut_l(&sheared_l());
    // The new wall 2x+y=3 is u+v=3/2, distinct from either wall family of
    // the sheared boxes. The retained part in (u,v) coordinates is a right
    // triangle with legs 3/2-1/3 = 7/6. Its height is one.
    let cutter = boundary(&convex(&[
        [-1, 0, 0, 0],
        [0, -1, 0, 0],
        [2, 1, 0, -3],
        [0, 0, -1, 0],
        [0, 0, 1, -1],
    ]));
    let triangle = apply(&clipped, &cutter, Operation3::Intersection);
    assert_eq!(triangle.volume(), (49, 72));
    assert_top_outline(&triangle, &[[4, 0], [18, 0], [11, 14]], 12);
    check_indexed_boundary(&triangle);
    let remainder = apply(&clipped, &cutter, Operation3::Difference);
    assert_eq!(remainder.volume(), (119, 72));
}

#[test]
fn coplanar_notch_contact_cancels_internal_faces_and_edge_contact_is_typed() {
    let l = sheared_l();
    let notch = boundary(&sheared_box([1, 1], [2, 2], 1));
    assert!(apply(&l, &notch, Operation3::Intersection).is_empty());
    let filled = apply(&l, &notch, Operation3::Union);
    assert_eq!(filled.volume(), (4, 1));
    assert_top_outline(&filled, &[[0, 0], [2, 0], [3, 2], [1, 2]], 1);
    check_indexed_boundary(&filled);
    let edge_contact = boundary(&sheared_box([2, 1], [3, 2], 1));
    assert!(apply(&l, &edge_contact, Operation3::Intersection).is_empty());
    assert_eq!(
        boolean_prismatic3(&l, &edge_contact, Operation3::Union, &mut work()).unwrap_err(),
        PolyhedralError::NonManifoldResult
    );
}

#[test]
fn staggered_heights_preserve_a_cap_with_four_radial_seams() {
    let lower = boundary(&sheared_box([0, 0], [1, 1], 1));
    let mut points: Vec<_> = (0..lower.vertex_count())
        .map(|i| lower.vertex(i).unwrap())
        .collect();
    let center = points.len();
    points.push([(3, 4), (1, 2), (1, 1)]);
    let mut faces: Vec<_> = (0..lower.face_count())
        .map(|i| IndexedFace {
            plane: lower.face_plane(i).unwrap(),
            vertices: lower.face_vertices(i).unwrap(),
        })
        .collect();
    let top = faces
        .iter()
        .position(|face| face.plane.coefficients() == [0, 0, 1, -1])
        .unwrap();
    let cap = faces.remove(top);
    assert_eq!(cap.vertices.len(), 4);
    let triangles: Vec<_> = cap
        .vertices
        .iter()
        .zip(cap.vertices.iter().cycle().skip(1))
        .map(|(&a, &b)| [a, b, center])
        .collect();
    faces.extend(triangles.iter().map(|ids| IndexedFace {
        plane: cap.plane,
        vertices: ids,
    }));
    let subdivided = SolidBoundary::from_indexed_faces(&points, &faces, &mut work()).unwrap();
    let upper = boundary(&convex(&[
        [-2, 1, 0, 0],
        [2, -1, 0, -2],
        [0, -1, 0, 0],
        [0, 1, 0, -1],
        [0, 0, -2, 1],
        [0, 0, 2, -3],
    ]));
    // The full sheared footprint has area 1. The overlap extends from the
    // upper operand's lower cap at 1/2 to the lower operand's split cap at 1.
    for (a, b) in [(&subdivided, &upper), (&upper, &subdivided)] {
        let overlap = apply(a, b, Operation3::Intersection);
        assert_eq!(overlap.volume(), (1, 2));
        assert!(
            (0..overlap.face_count())
                .any(|i| { overlap.face_plane(i).unwrap().coefficients() == [0, 0, -2, 1] })
        );
        assert_top_outline(&overlap, &[[0, 0], [2, 0], [3, 2], [1, 2]], 2);
        check_indexed_boundary(&overlap);
        assert!(apply(&overlap, &upper, Operation3::Difference).is_empty());
    }
}

#[test]
fn tilted_face_and_resource_failures_return_errors() {
    let tilted = boundary(&convex(&[
        [-1, 0, 0, 0],
        [0, -1, 0, 0],
        [0, 0, -1, 0],
        [1, 1, 1, -1],
    ]));
    let empty = SolidBoundary::empty();
    for (a, b) in [(&tilted, &empty), (&empty, &tilted)] {
        assert_eq!(
            boolean_prismatic3(a, b, Operation3::Intersection, &mut work()).unwrap_err(),
            PolyhedralError::UnsupportedFaceGeometry
        );
    }
    let l = sheared_l();
    for mut budget in [
        PolyhedralWork::new(0, usize::MAX),
        PolyhedralWork::new(usize::MAX, 0),
    ] {
        assert_eq!(
            boolean_prismatic3(&l, &l, Operation3::Union, &mut budget).unwrap_err(),
            PolyhedralError::ResourceLimit
        );
    }
    let stop = AtomicBool::new(true);
    assert_eq!(
        boolean_prismatic3(
            &l,
            &l,
            Operation3::Union,
            &mut work().with_cancellation(&stop)
        )
        .unwrap_err(),
        PolyhedralError::Cancelled
    );
}
