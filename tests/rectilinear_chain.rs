// SPDX-License-Identifier: Apache-2.0
use std::collections::BTreeSet;
use std::sync::atomic::AtomicBool;

use rcct::polyhedral::{
    ConvexSolid, IndexedFace, Operation3, Plane3h, PolyhedralError, PolyhedralWork, SolidBoundary,
    boolean_rectilinear3, boolean3,
};

fn work() -> PolyhedralWork<'static> {
    PolyhedralWork::new(100_000_000, 128 * 1024 * 1024)
}

fn admit(rows: &[[i64; 4]]) -> ConvexSolid {
    let planes: Vec<_> = rows
        .iter()
        .map(|&r| Plane3h::from_coefficients(r).unwrap())
        .collect();
    ConvexSolid::from_halfspaces(&planes, &mut work()).unwrap()
}

// Bounds are integer numerators over the same positive denominator.
fn cuboid(lo: [i64; 3], hi: [i64; 3], den: i64) -> ConvexSolid {
    admit(&[
        [-den, 0, 0, lo[0]],
        [den, 0, 0, -hi[0]],
        [0, -den, 0, lo[1]],
        [0, den, 0, -hi[1]],
        [0, 0, -den, lo[2]],
        [0, 0, den, -hi[2]],
    ])
}

fn boundary(operand: &ConvexSolid) -> SolidBoundary {
    boolean3(
        operand,
        &ConvexSolid::empty(),
        Operation3::Union,
        &mut work(),
    )
    .unwrap()
}

fn apply(a: &SolidBoundary, b: &SolidBoundary, op: Operation3) -> SolidBoundary {
    boolean_rectilinear3(a, b, op, &mut work()).unwrap()
}

// Measure the oriented z=1 cap in each of the four unit squares of [0,2]².
// These fixtures lie on a thirds grid, so integer calculations give ninths
// of area. Accumulating overlaps permits any conforming rectangle subdivision.
fn cap_areas_in_ninths(solid: &SolidBoundary) -> [i128; 4] {
    let mut areas = [0; 4];
    for face in 0..solid.face_count() {
        let points: Vec<_> = solid
            .face_vertices(face)
            .unwrap()
            .iter()
            .map(|&id| {
                solid.vertex(id).unwrap().map(|(n, d)| {
                    assert_eq!((3 * n) % d, 0);
                    (3 * n) / d
                })
            })
            .collect();
        if !points.iter().all(|p| p[2] == 3) {
            continue;
        }
        let xmin = points.iter().map(|p| p[0]).min().unwrap();
        let xmax = points.iter().map(|p| p[0]).max().unwrap();
        let ymin = points.iter().map(|p| p[1]).min().unwrap();
        let ymax = points.iter().map(|p| p[1]).max().unwrap();
        let twice_area: i128 = points
            .iter()
            .zip(points.iter().cycle().skip(1))
            .map(|(a, b)| a[0] * b[1] - a[1] * b[0])
            .sum();
        assert_eq!(twice_area, 2 * (xmax - xmin) * (ymax - ymin));
        for (index, area) in areas.iter_mut().enumerate() {
            let x = 3 * (index % 2) as i128;
            let y = 3 * (index / 2) as i128;
            let width = (xmax.min(x + 3) - xmin.max(x)).max(0);
            let height = (ymax.min(y + 3) - ymin.max(y)).max(0);
            *area += width * height;
        }
    }
    areas
}

#[test]
fn nonconvex_union_chains_into_exact_cut_and_recombination() {
    let bar = cuboid([0; 3], [2, 1, 1], 1);
    let upright = cuboid([0; 3], [1, 2, 1], 1);
    let l_shape = boolean3(&bar, &upright, Operation3::Union, &mut work()).unwrap();
    assert_eq!(l_shape.volume(), (3, 1));
    assert_eq!(
        l_shape.as_convex(&mut work()).unwrap_err(),
        PolyhedralError::UnsupportedNonconvex
    );

    // x >= 1/3 removes a strip of width 1/3 and height 2 from the L.
    // The empty upper-right quadrant must stay empty after intersection.
    let cutter = boundary(&cuboid([1, 0, 0], [6, 6, 3], 3));
    let clipped = apply(&l_shape, &cutter, Operation3::Intersection);
    let swapped = apply(&cutter, &l_shape, Operation3::Intersection);
    for result in [&clipped, &swapped] {
        assert_eq!(result.volume(), (7, 3));
        assert_eq!(result.shell_count(), 1);
        assert_eq!(cap_areas_in_ninths(result), [6, 9, 6, 0]);
    }
    let strip = apply(&l_shape, &cutter, Operation3::Difference);
    let notch = apply(&cutter, &l_shape, Operation3::Difference);
    assert_eq!(strip.volume(), (2, 3));
    assert_eq!(cap_areas_in_ninths(&strip), [3, 0, 3, 0]);
    assert_eq!(notch.volume(), (1, 1));
    assert_eq!(cap_areas_in_ninths(&notch), [0, 0, 0, 9]);
    let reunited = apply(&clipped, &notch, Operation3::Union);
    assert_eq!(reunited.volume(), (10, 3));
    assert_eq!(cap_areas_in_ninths(&reunited), [6, 9, 6, 9]);
    assert!(reunited.as_convex(&mut work()).is_ok());
}

#[test]
fn slicing_a_cavity_opens_a_through_hole() {
    let outer = cuboid([0; 3], [3; 3], 1);
    let inner = cuboid([1; 3], [2; 3], 1);
    let hollow = boolean3(&outer, &inner, Operation3::Difference, &mut work()).unwrap();
    assert_eq!((hollow.volume(), hollow.shell_count()), ((26, 1), 2));
    let slab = boundary(&cuboid([0, 0, 5], [12, 12, 7], 4));
    let ring = apply(&hollow, &slab, Operation3::Intersection);
    // Area (3²-1²) times thickness 1/2; one connected genus-one boundary.
    assert_eq!((ring.volume(), ring.shell_count()), ((4, 1), 1));
    let mut edges = BTreeSet::new();
    for face in 0..ring.face_count() {
        let ids = ring.face_vertices(face).unwrap();
        for (&a, &b) in ids.iter().zip(ids.iter().cycle().skip(1)) {
            edges.insert((a.min(b), a.max(b)));
        }
    }
    assert_eq!(ring.vertex_count() + ring.face_count(), edges.len());
}

#[test]
fn unsupported_faces_remain_errors_even_for_empty_intersection() {
    let tetrahedron = boundary(&admit(&[
        [-1, 0, 0, 0],
        [0, -1, 0, 0],
        [0, 0, -1, 0],
        [1, 1, 1, -1],
    ]));
    let cube = boundary(&cuboid([0; 3], [1; 3], 1));
    let points: Vec<_> = (0..cube.vertex_count())
        .map(|i| cube.vertex(i).unwrap())
        .collect();
    let mut faces: Vec<_> = (0..cube.face_count())
        .map(|i| IndexedFace {
            plane: cube.face_plane(i).unwrap(),
            vertices: cube.face_vertices(i).unwrap(),
        })
        .collect();
    // Keep all support planes axis-aligned but split one square along a diagonal.
    let first = faces.remove(0);
    let [a, b, c, d] = *first.vertices else {
        panic!("expected a square cube face")
    };
    let triangles = [[a, b, c], [a, c, d]];
    faces.extend(triangles.iter().map(|v| IndexedFace {
        plane: first.plane,
        vertices: v,
    }));
    let triangulated = SolidBoundary::from_indexed_faces(&points, &faces, &mut work()).unwrap();
    let empty = SolidBoundary::empty();
    for unsupported in [&tetrahedron, &triangulated] {
        for (a, b) in [(unsupported, &empty), (&empty, unsupported)] {
            assert_eq!(
                boolean_rectilinear3(a, b, Operation3::Intersection, &mut work()).unwrap_err(),
                PolyhedralError::UnsupportedFaceGeometry
            );
        }
    }
}

#[test]
fn contact_and_resource_failures_do_not_publish_a_partial_solid() {
    let a = boundary(&cuboid([0; 3], [1; 3], 1));
    for lo in [[1, 1, 0], [1, 1, 1]] {
        let touching = boundary(&cuboid(lo, lo.map(|v| v + 1), 1));
        assert!(apply(&a, &touching, Operation3::Intersection).is_empty());
        assert_eq!(
            boolean_rectilinear3(&a, &touching, Operation3::Union, &mut work()).unwrap_err(),
            PolyhedralError::NonManifoldResult
        );
    }
    let stop = AtomicBool::new(true);
    for mut budget in [
        PolyhedralWork::new(0, usize::MAX),
        PolyhedralWork::new(usize::MAX, 0),
    ] {
        assert_eq!(
            boolean_rectilinear3(&a, &a, Operation3::Union, &mut budget).unwrap_err(),
            PolyhedralError::ResourceLimit
        );
    }
    assert_eq!(
        boolean_rectilinear3(
            &a,
            &a,
            Operation3::Union,
            &mut work().with_cancellation(&stop)
        )
        .unwrap_err(),
        PolyhedralError::Cancelled
    );
    assert_eq!(apply(&a, &a, Operation3::Union).volume(), (1, 1));
}
