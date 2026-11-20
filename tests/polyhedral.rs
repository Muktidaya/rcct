// SPDX-License-Identifier: Apache-2.0
use std::collections::BTreeMap;

use rcct::polyhedral::{
    ConvexSolid, Operation3, Plane3h, PolyhedralError, PolyhedralWork, SolidBoundary, boolean3,
};

fn work() -> PolyhedralWork<'static> {
    PolyhedralWork::new(100_000_000, 128 * 1024 * 1024)
}

fn box_planes(lo: [i64; 3], hi: [i64; 3]) -> [[i64; 4]; 6] {
    [
        [-1, 0, 0, lo[0]],
        [1, 0, 0, -hi[0]],
        [0, -1, 0, lo[1]],
        [0, 1, 0, -hi[1]],
        [0, 0, -1, lo[2]],
        [0, 0, 1, -hi[2]],
    ]
}

fn planes(coefficients: &[[i64; 4]]) -> Vec<Plane3h> {
    coefficients
        .iter()
        .map(|&p| Plane3h::from_coefficients(p).unwrap())
        .collect()
}

fn admit(coefficients: &[[i64; 4]]) -> ConvexSolid {
    ConvexSolid::from_halfspaces(&planes(coefficients), &mut work()).unwrap()
}

fn cube(lo: i64, hi: i64) -> ConvexSolid {
    admit(&box_planes([lo; 3], [hi; 3]))
}

fn apply(a: &ConvexSolid, b: &ConvexSolid, op: Operation3) -> SolidBoundary {
    boolean3(a, b, op, &mut work()).unwrap()
}

// Count uses of each returned edge independently, including subdivision edges.
fn check_boundary(result: &SolidBoundary, volume: (i128, i128), shells: usize) {
    assert_eq!(result.volume(), volume);
    assert_eq!(result.shell_count(), shells);
    assert_eq!(result.is_empty(), shells == 0);
    let mut edge_uses = BTreeMap::new();
    for face in 0..result.face_count() {
        let vertices = result.face_vertices(face).unwrap();
        assert!(vertices.len() >= 3);
        for (&a, &b) in vertices.iter().zip(vertices.iter().cycle().skip(1)) {
            assert!(result.vertex(a).is_some() && result.vertex(b).is_some());
            assert_ne!(a, b);
            let counts = edge_uses.entry((a.min(b), a.max(b))).or_insert([0, 0]);
            counts[usize::from(a < b)] += 1;
        }
    }
    assert!(edge_uses.values().all(|&counts| counts == [1, 1]));
    assert_eq!(edge_uses.is_empty(), shells == 0);
}

#[test]
fn equivalent_planes_and_explicit_empty_obey_set_identities() {
    let a = cube(0, 1);
    let mut scaled = box_planes([0; 3], [1; 3])
        .map(|p| p.map(|c| 3 * c))
        .to_vec();
    scaled.reverse();
    scaled.push([2, 0, 0, -2]); // A redundant copy of x <= 1.
    let b = admit(&scaled);
    for op in [Operation3::Union, Operation3::Intersection] {
        check_boundary(&apply(&a, &b, op), (1, 1), 1);
    }
    check_boundary(&apply(&a, &b, Operation3::Difference), (0, 1), 0);
    let empty = ConvexSolid::empty();
    check_boundary(&apply(&empty, &a, Operation3::Union), (1, 1), 1);
    check_boundary(&apply(&a, &empty, Operation3::Intersection), (0, 1), 0);
    check_boundary(&apply(&empty, &a, Operation3::Difference), (0, 1), 0);
    check_boundary(&apply(&a, &empty, Operation3::Difference), (1, 1), 1);
}

#[test]
fn overlapping_cubes_and_exact_convex_readmission() {
    let a = cube(0, 2);
    let b = cube(1, 3);
    let intersection = apply(&a, &b, Operation3::Intersection);
    check_boundary(&intersection, (1, 1), 1);
    let points: Vec<_> = (0..intersection.vertex_count())
        .map(|i| intersection.vertex(i).unwrap())
        .collect();
    for x in [1, 2] {
        for y in [1, 2] {
            for z in [1, 2] {
                assert!(points.contains(&[(x, 1), (y, 1), (z, 1)]));
            }
        }
    }
    let reused = intersection.as_convex(&mut work()).unwrap();
    check_boundary(&apply(&a, &reused, Operation3::Difference), (7, 1), 1);
    let union = apply(&a, &b, Operation3::Union);
    check_boundary(&union, (15, 1), 1);
    assert!(matches!(
        union.as_convex(&mut work()),
        Err(PolyhedralError::UnsupportedNonconvex)
    ));
    check_boundary(&apply(&a, &b, Operation3::Difference), (7, 1), 1);
}

#[test]
fn cavity_boundaries_have_opposite_material_orientation() {
    let result = apply(&cube(0, 3), &cube(1, 2), Operation3::Difference);
    check_boundary(&result, (26, 1), 2);
    let mut planes_seen = [[false; 4]; 3];
    for face in 0..result.face_count() {
        let points: Vec<_> = result
            .face_vertices(face)
            .unwrap()
            .iter()
            .map(|&i| {
                result.vertex(i).unwrap().map(|(num, den)| {
                    assert_eq!(den, 1);
                    num
                })
            })
            .collect();
        let axis = (0..3)
            .find(|&axis| points.iter().all(|p| p[axis] == points[0][axis]))
            .unwrap();
        let offset = usize::try_from(points[0][axis]).unwrap();
        planes_seen[axis][offset] = true;
        let (j, k) = ((axis + 1) % 3, (axis + 2) % 3);
        let area2: i128 = points
            .iter()
            .zip(points.iter().cycle().skip(1))
            .map(|(p, q)| p[j] * q[k] - q[j] * p[k])
            .sum();
        // Outward at 0 and 3; the material normal points into the cavity at 1 and 2.
        assert_eq!(area2.signum(), [-1, 1, -1, 1][offset]);
    }
    assert!(planes_seen.iter().flatten().all(|&seen| seen));
}

#[test]
fn regularized_contacts_distinguish_edges_vertices_and_faces() {
    let a = cube(0, 1);
    for lo in [[1, 1, 0], [1, 1, 1]] {
        let touching = admit(&box_planes(lo, lo.map(|n| n + 1)));
        check_boundary(&apply(&a, &touching, Operation3::Intersection), (0, 1), 0);
        assert!(matches!(
            boolean3(&a, &touching, Operation3::Union, &mut work()),
            Err(PolyhedralError::NonManifoldResult)
        ));
    }
    let adjacent = admit(&box_planes([1, 0, 0], [2, 1, 1]));
    check_boundary(&apply(&a, &adjacent, Operation3::Intersection), (0, 1), 0);
    let joined = apply(&a, &adjacent, Operation3::Union);
    check_boundary(&joined, (2, 1), 1);
    for i in 0..joined.face_count() {
        assert!(
            joined
                .face_vertices(i)
                .unwrap()
                .iter()
                .any(|&v| joined.vertex(v).unwrap()[0] != (1, 1))
        );
    }
}

#[test]
fn oblique_plane_and_rational_sliver_preserve_exact_geometry() {
    // Unit square sheared by x' = x+y: lower/upper x bounds become bounds on x'-y.
    let mut sheared = box_planes([0; 3], [1; 3]);
    sheared[0] = [-1, 1, 0, 0];
    sheared[1] = [1, -1, 0, -1];
    let points = admit(&sheared);
    let half_cube = apply(&cube(0, 1), &points, Operation3::Intersection);
    check_boundary(&half_cube, (1, 2), 1);
    // Only a slab of width 1/1024 overlaps; the other two dimensions each have length 1.
    let mut shifted = box_planes([0; 3], [2, 1, 1]);
    shifted[0] = [-1024, 0, 0, 1023];
    let overlap = apply(&cube(0, 1), &admit(&shifted), Operation3::Intersection);
    check_boundary(&overlap, (1, 1024), 1);
    assert!((0..overlap.vertex_count()).any(|v| overlap.vertex(v).unwrap()[0] == (1023, 1024)));

    let x = 1_i64 << 53;
    assert_eq!(x as f64, (x + 1) as f64);
    let large = admit(&box_planes([x - 1, 0, 0], [x + 1, 1, 1]));
    let offset = admit(&box_planes([x, 0, 0], [x + 2, 1, 1]));
    let thin = apply(&large, &offset, Operation3::Intersection);
    check_boundary(&thin, (1, 1), 1);
    for coordinate in [x, x + 1] {
        assert!(
            (0..thin.vertex_count()).any(|i| thin.vertex(i).unwrap()[0] == (coordinate.into(), 1))
        );
    }
}

#[test]
fn disconnected_and_partially_coincident_boundaries() {
    check_boundary(
        &apply(&cube(0, 1), &cube(2, 3), Operation3::Union),
        (2, 1),
        2,
    );
    let slab = admit(&box_planes([1, -1, -1], [2, 4, 4]));
    check_boundary(
        &apply(&cube(0, 3), &slab, Operation3::Difference),
        (18, 1),
        2,
    );
    let small = admit(&box_planes([2, 0, 0], [3, 1, 1]));
    let partial = apply(&cube(0, 2), &small, Operation3::Union);
    check_boundary(&partial, (9, 1), 1);
}

#[test]
fn invalid_unbounded_and_exhausted_are_distinct_from_empty() {
    assert!(matches!(
        Plane3h::from_coefficients([0, 0, 0, 1]),
        Err(PolyhedralError::InvalidPlane)
    ));
    let p = planes(&box_planes([0; 3], [1; 3]));
    assert!(matches!(
        ConvexSolid::from_halfspaces(&p[..5], &mut work()),
        Err(PolyhedralError::UnboundedOperand)
    ));
    let flat = planes(&box_planes([0; 3], [0, 1, 1]));
    assert!(matches!(
        ConvexSolid::from_halfspaces(&flat, &mut work()),
        Err(PolyhedralError::InvalidOperand)
    ));
    let a = cube(0, 1);
    for (steps, bytes) in [(0, 1024 * 1024), (1_000_000, 0)] {
        let mut limited = PolyhedralWork::new(steps, bytes);
        assert!(matches!(
            boolean3(&a, &a, Operation3::Intersection, &mut limited),
            Err(PolyhedralError::ResourceLimit)
        ));
    }
    let cancel = std::sync::atomic::AtomicBool::new(true);
    let mut cancelled = PolyhedralWork::new(1_000_000, 1024 * 1024).with_cancellation(&cancel);
    assert!(matches!(
        boolean3(&a, &a, Operation3::Union, &mut cancelled),
        Err(PolyhedralError::Cancelled)
    ));
    check_boundary(&apply(&a, &a, Operation3::Intersection), (1, 1), 1);
}
