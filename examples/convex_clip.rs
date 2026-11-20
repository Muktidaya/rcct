// SPDX-License-Identifier: Apache-2.0

//! Clip the unit cube by an oblique plane and inspect the owned result.

use rcct::polyhedral::{
    ConvexSolid, Operation3, Plane3h, PolyhedralError, PolyhedralWork, boolean3,
};

fn main() -> Result<(), PolyhedralError> {
    let mut work = PolyhedralWork::new(2_000_000, 16 * 1024 * 1024);
    // Each row denotes ax + by + cz + d <= 0.
    let cube_planes = [
        [-1, 0, 0, 0],
        [1, 0, 0, -1],
        [0, -1, 0, 0],
        [0, 1, 0, -1],
        [0, 0, -1, 0],
        [0, 0, 1, -1],
    ]
    .map(Plane3h::from_coefficients)
    .into_iter()
    .collect::<Result<Vec<_>, _>>()?;
    let cube = ConvexSolid::from_halfspaces(&cube_planes, &mut work)?;
    let cutter_planes = [
        Plane3h::from_coefficients([-1, 0, 0, 0])?,
        Plane3h::from_coefficients([0, -1, 0, 0])?,
        Plane3h::from_coefficients([0, 0, -1, 0])?,
        Plane3h::from_coefficients([1, 1, 1, -1])?,
    ];
    let tetrahedron = ConvexSolid::from_halfspaces(&cutter_planes, &mut work)?;
    let result = boolean3(&cube, &tetrahedron, Operation3::Intersection, &mut work)?;
    assert_eq!(result.volume(), (1, 6));
    println!("Volume 1/6, {} boundary faces", result.face_count());
    Ok(())
}
