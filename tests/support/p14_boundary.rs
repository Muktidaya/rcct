// SPDX-License-Identifier: Apache-2.0
use std::collections::BTreeSet;

use rcct::polyhedral::SolidBoundary;

#[path = "oriented_boundary.rs"]
mod oriented_boundary;

type Point = [(i128, i128); 3];
type Triple = [i128; 3];

fn sheared([u, v, w]: Triple) -> Point {
    [(2 * u + v, 2), (v, 1), (w, 1)]
}

fn expected_faces(cells: &[[i32; 2]]) -> Vec<([i128; 4], Vec<Point>)> {
    let voxels: BTreeSet<_> = cells
        .iter()
        .map(|&[u, v]| [i128::from(u), i128::from(v), 0])
        .collect();
    assert_eq!(voxels.len(), cells.len(), "duplicate expected cell");
    let mut faces = Vec::new();
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
                faces.push((plane, face.to_vec()));
            }
        }
    }
    faces
}

/// Checks both caps and all walls of the unit-height sheared cells. On each
/// oriented support, conforming internal seams cancel and collinear edge
/// subdivisions telescope. The remaining outline determines the bounded
/// oriented face region independently of the returned patch and vertex IDs.
pub fn assert_boundary(result: &SolidBoundary, cells: &[[i32; 2]]) {
    oriented_boundary::assert_oriented_faces(result, expected_faces(cells));
    assert_eq!(result.volume(), (cells.len() as i128, 1));
}
