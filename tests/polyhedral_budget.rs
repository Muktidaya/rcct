// SPDX-License-Identifier: Apache-2.0
use std::collections::BTreeSet;

use rcct::polyhedral::{
    ConvexSolid, Operation3, Plane3h, PolyhedralError, PolyhedralWork, SolidBoundary, boolean3,
};

fn work() -> PolyhedralWork<'static> {
    PolyhedralWork::new(100_000_000, 128 * 1024 * 1024)
}

fn cube(lo: i64, hi: i64) -> ConvexSolid {
    let planes = [
        [-1, 0, 0, lo],
        [1, 0, 0, -hi],
        [0, -1, 0, lo],
        [0, 1, 0, -hi],
        [0, 0, -1, lo],
        [0, 0, 1, -hi],
    ]
    .map(|r| Plane3h::from_coefficients(r).unwrap());
    ConvexSolid::from_halfspaces(&planes, &mut work()).unwrap()
}

// Read the public boundary of an opaque convex operand independently of the
// intersection whose budget is under test.
fn boundary(solid: &ConvexSolid) -> SolidBoundary {
    boolean3(solid, &ConvexSolid::empty(), Operation3::Union, &mut work()).unwrap()
}

fn assert_unchanged(expected: &SolidBoundary, actual: &SolidBoundary) {
    assert_eq!(actual.volume(), expected.volume());
    assert_eq!(actual.shell_count(), expected.shell_count());
    assert_eq!(actual.vertex_count(), expected.vertex_count());
    assert_eq!(actual.face_count(), expected.face_count());
    for id in 0..expected.vertex_count() {
        assert_eq!(actual.vertex(id), expected.vertex(id));
    }
    for id in 0..expected.face_count() {
        assert_eq!(actual.face_plane(id), expected.face_plane(id));
        assert_eq!(actual.face_vertices(id), expected.face_vertices(id));
    }
}

// [0,2]^3 intersect [1,3]^3 is the closed unit cube [1,2]^3.
fn assert_overlap(result: &SolidBoundary) {
    assert_eq!(result.volume(), (1, 1));
    assert_eq!(result.shell_count(), 1);
    let actual: BTreeSet<_> = (0..result.vertex_count())
        .map(|id| result.vertex(id).unwrap())
        .collect();
    // Allow coplanar subdivisions. All vertices must lie inside the expected
    // cube, and the eight extremal corners must all be represented.
    for point in &actual {
        for &(n, d) in point {
            assert!(d > 0 && n >= d && n - d <= d);
        }
    }
    for x in [1, 2] {
        for y in [1, 2] {
            for z in [1, 2] {
                assert!(actual.contains(&[(x, 1), (y, 1), (z, 1)]));
            }
        }
    }
}

#[test]
fn one_short_boolean_budgets_retain_charges_and_allow_exact_retry() {
    let a = cube(0, 2);
    let b = cube(1, 3);
    let before_a = boundary(&a);
    let before_b = boundary(&b);
    let intersect =
        |meter: &mut PolyhedralWork<'_>| boolean3(&a, &b, Operation3::Intersection, meter);

    // Calibrate against this run. Allocation sizes and internal checkpoints
    // are implementation details, not fixed thresholds for other platforms.
    let mut reference = work();
    let (work_start, bytes_start) = reference.remaining();
    assert_overlap(&intersect(&mut reference).unwrap());
    let (work_left, bytes_left) = reference.remaining();
    let steps = work_start - work_left;
    let bytes = bytes_start - bytes_left;
    assert!(steps > 1 && bytes > 1);

    for (work_limit, byte_limit) in [(steps - 1, bytes), (steps, bytes - 1)] {
        let mut limited = PolyhedralWork::new(work_limit, byte_limit);
        assert_eq!(
            intersect(&mut limited).unwrap_err(),
            PolyhedralError::ResourceLimit
        );
        let (work_left, bytes_left) = limited.remaining();
        assert!(work_left < work_limit && bytes_left < byte_limit);
        assert_unchanged(&before_a, &boundary(&a));
        assert_unchanged(&before_b, &boundary(&b));

        let mut exact = PolyhedralWork::new(steps, bytes);
        assert_overlap(&intersect(&mut exact).unwrap());
        assert_eq!(exact.remaining(), (0, 0));
    }
}
