// SPDX-License-Identifier: Apache-2.0

use crate::polyhedral::exact::{P, Q};
use crate::polyhedral::{Plane3h, PolyhedralError, PolyhedralWork, R, SolidBoundary};
use std::cmp::Ordering;

pub(super) struct Patch {
    pub(super) plane: Plane3h,
    pub(super) axis: usize,
    pub(super) lo: P,
    pub(super) hi: P,
}

fn insert_level(levels: &mut Vec<Q>, value: Q, work: &mut PolyhedralWork<'_>) -> R<()> {
    let mut index = levels.len();
    for (i, &level) in levels.iter().enumerate() {
        work.charge(1)?;
        match value.cmp_checked(level)? {
            Ordering::Equal => return Ok(()),
            Ordering::Less => {
                index = i;
                break;
            }
            Ordering::Greater => {}
        }
    }
    work.charge(levels.len() - index)?;
    work.reserve(levels, 1)?;
    levels.insert(index, value);
    Ok(())
}

pub(super) fn rectangles(
    body: &SolidBoundary,
    levels: &mut [Vec<Q>; 3],
    work: &mut PolyhedralWork<'_>,
) -> R<Vec<Patch>> {
    let mut patches = Vec::new();
    work.reserve(&mut patches, body.faces.len())?;
    for face in &body.faces {
        work.charge(1)?;
        // Plane3h admission already excludes a zero normal.
        let axis = match face.plane.coefficients() {
            [_, 0, 0, _] => 0,
            [0, _, 0, _] => 1,
            [0, 0, _, _] => 2,
            _ => return Err(PolyhedralError::UnsupportedFaceGeometry),
        };
        let u = (axis + 1) % 3;
        let v = (axis + 2) % 3;
        let mut previous = body.vertices[*face
            .indices
            .last()
            .ok_or(PolyhedralError::InvalidBoundary)?];
        let mut lo = previous;
        let mut hi = previous;
        for &index in &face.indices {
            work.charge(1)?;
            let point = body.vertices[index];
            if point[u] != previous[u] && point[v] != previous[v] {
                return Err(PolyhedralError::UnsupportedFaceGeometry);
            }
            for i in [u, v] {
                if point[i].cmp_checked(lo[i])? == Ordering::Less {
                    lo[i] = point[i];
                }
                if point[i].cmp_checked(hi[i])? == Ordering::Greater {
                    hi[i] = point[i];
                }
            }
            previous = point;
        }

        // A validated convex face with only these two edge directions fills
        // its rectangle. Intermediate collinear vertices add no material event.
        for (i, axis_levels) in levels.iter_mut().enumerate() {
            insert_level(axis_levels, lo[i], work)?;
            if lo[i] != hi[i] {
                insert_level(axis_levels, hi[i], work)?;
            }
        }
        patches.push(Patch {
            plane: face.plane,
            axis,
            lo,
            hi,
        });
    }
    Ok(patches)
}
