// SPDX-License-Identifier: Apache-2.0

use crate::polyhedral::exact::{P, Q};
use crate::polyhedral::{
    ConvexSolid, Operation3, Plane3h, PolyhedralWork, SolidBoundary, boolean3,
};

fn work() -> PolyhedralWork<'static> {
    PolyhedralWork::new(100_000_000, 128 * 1024 * 1024)
}

pub(super) fn cube(lo: i64, hi: i64) -> ConvexSolid {
    let planes = [
        [-1, 0, 0, lo],
        [1, 0, 0, -hi],
        [0, -1, 0, lo],
        [0, 1, 0, -hi],
        [0, 0, -1, lo],
        [0, 0, 1, -hi],
    ]
    .map(|row| Plane3h::from_coefficients(row).unwrap());
    ConvexSolid::from_halfspaces(&planes, &mut work()).unwrap()
}

pub(super) fn boundary(solid: &ConvexSolid) -> SolidBoundary {
    boolean3(solid, &ConvexSolid::empty(), Operation3::Union, &mut work()).unwrap()
}

#[derive(Debug, Eq, PartialEq)]
pub(super) struct ConvexSnapshot {
    planes: Vec<Plane3h>,
    faces: Vec<(Plane3h, Vec<P>)>,
}

pub(super) fn convex_snapshot(solid: &ConvexSolid) -> ConvexSnapshot {
    ConvexSnapshot {
        planes: solid.planes.clone(),
        faces: solid
            .faces
            .iter()
            .map(|face| (face.plane, face.vertices.clone()))
            .collect(),
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(super) struct BoundarySnapshot {
    vertices: Vec<P>,
    faces: Vec<(Plane3h, Vec<usize>)>,
    volume: (i128, i128),
    shells: usize,
}

pub(super) fn boundary_snapshot(solid: &SolidBoundary) -> BoundarySnapshot {
    BoundarySnapshot {
        vertices: solid.vertices.clone(),
        faces: solid
            .faces
            .iter()
            .map(|face| (face.plane, face.indices.clone()))
            .collect(),
        volume: solid.volume(),
        shells: solid.shell_count(),
    }
}

// A = [0,2]³ and B = [1,3]³ overlap in one unit cube. A−B retains the
// cell at the origin, while the equal-volume B−A retains the far corner.
pub(super) fn assert_result(solid: &SolidBoundary, op: Operation3) {
    let volume = match op {
        Operation3::Union => 15,
        Operation3::Intersection => 1,
        Operation3::Difference => 7,
    };
    assert_eq!(solid.volume(), (volume, 1));
    assert_eq!(solid.shell_count(), 1);

    // Every face must lie on an integer grid plane. This makes material
    // constant in each open unit cell, regardless of coplanar subdivisions.
    for face in &solid.faces {
        let [a, b, c, d] = face.plane.coefficients();
        assert_eq!(a.abs() + b.abs() + c.abs(), 1);
        assert!((0..=3).contains(&(-d * (a + b + c))));
    }
    for x in 0..3 {
        for y in 0..3 {
            for z in 0..3 {
                let cell = [x, y, z];
                let in_a = cell.iter().all(|&i| i < 2);
                let in_b = cell.iter().all(|&i| i >= 1);
                let expected = match op {
                    Operation3::Union => in_a || in_b,
                    Operation3::Intersection => in_a && in_b,
                    Operation3::Difference => in_a && !in_b,
                };
                // Unequal offsets avoid sending a diagonal ray through the
                // grid corners, as could happen at the exact cell center.
                let sample = [(x, 2), (y, 3), (z, 5)]
                    .map(|(i, d)| Q::ratio(i128::from(d * i + 1), i128::from(d)).unwrap());
                // Both sides of a plane through this interior sample have
                // the same material. Read it from the result's indexed faces.
                let plane = Plane3h::from_coefficients([2, 0, 0, -(2 * x + 1)]).unwrap();
                let actual = solid.material_sides_at(sample, plane, &mut work()).unwrap();
                assert_eq!(actual, (expected, expected), "{op:?} in cell {cell:?}");
            }
        }
    }
}
