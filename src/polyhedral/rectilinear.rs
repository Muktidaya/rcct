// SPDX-License-Identifier: Apache-2.0

use super::exact::{P, Q};
use super::{Facet, Operation3, Plane3h, PolyhedralError, PolyhedralWork, R, SolidBoundary};
use std::cmp::Ordering;

mod input;
use input::{Patch, rectangles};

/// Applies a regularized Boolean operation directly to two owned solid boundaries
/// whose patches are rectangles parallel to the coordinate planes.
///
/// An operand may be nonconvex, have a cavity, or contain disjoint components.
/// Collinear subdivision vertices on a rectangular patch are allowed. Both
/// operands are checked, including in operations involving an empty boundary.
/// An oblique support or an edge diagonal to the coordinate axes yields
/// [`PolyhedralError::UnsupportedFaceGeometry`].
///
/// The result owns its oriented rectangular patches and can be passed to this
/// function again. Exact coordinate levels subdivide those patches, so a face
/// count can depend on operand subdivisions. Only volume cells contribute
/// material. Edge- or point-contact unions that would have a nonmanifold
/// boundary return a typed error. Work, storage, cancellation and arithmetic
/// limits apply through the final aggregate boundary check.
pub fn boolean_rectilinear3(
    a: &SolidBoundary,
    b: &SolidBoundary,
    op: Operation3,
    work: &mut PolyhedralWork<'_>,
) -> R<SolidBoundary> {
    work.charge(1)?;
    let mut levels = [Vec::new(), Vec::new(), Vec::new()];
    let patches = [
        rectangles(a, &mut levels, work)?,
        rectangles(b, &mut levels, work)?,
    ];
    let mut faces = Vec::new();
    for axis in 0..3 {
        for u in levels[(axis + 1) % 3].windows(2) {
            for v in levels[(axis + 2) % 3].windows(2) {
                work.charge(1)?;
                let column = Column {
                    axis,
                    u: [u[0], u[1]],
                    v: [v[0], v[1]],
                };
                column.sweep(&levels[axis], &patches, op, &mut faces, work)?;
            }
        }
    }
    SolidBoundary::from_facets(faces, work)
}

// The open rectangle between two successive transverse coordinates has
// constant coverage on every input patch. Sweep its whole interior at once:
// no offset, rounded sample, or constructed midpoint is needed.
struct Column {
    axis: usize,
    u: [Q; 2],
    v: [Q; 2],
}

impl Column {
    fn sweep(
        &self,
        levels: &[Q],
        patches: &[Vec<Patch>; 2],
        op: Operation3,
        faces: &mut Vec<Facet>,
        work: &mut PolyhedralWork<'_>,
    ) -> R<()> {
        let mut inside = [false; 2];
        for &level in levels {
            work.charge(1)?;
            let before = op.accepts(inside[0], inside[1]);
            let mut support = None;
            for (operand, state) in patches.iter().zip(&mut inside) {
                if let Some(plane) = self.crossing(operand, level, work)? {
                    let enters = plane.coefficients()[self.axis] < 0;
                    if enters == *state {
                        return Err(PolyhedralError::InvalidBoundary);
                    }
                    *state = enters;
                    support = Some(plane);
                }
            }
            // Apply coincident events from both operands before deciding if
            // this tile separates two material states (including difference).
            let after = op.accepts(inside[0], inside[1]);
            if before != after {
                let plane = support.ok_or(PolyhedralError::InvalidBoundary)?;
                let face = self.face(level, plane, before, work)?;
                work.reserve(faces, 1)?;
                faces.push(face);
            }
        }
        if inside != [false; 2] {
            return Err(PolyhedralError::InvalidBoundary);
        }
        Ok(())
    }

    fn crossing(
        &self,
        patches: &[Patch],
        level: Q,
        work: &mut PolyhedralWork<'_>,
    ) -> R<Option<Plane3h>> {
        let u = (self.axis + 1) % 3;
        let v = (self.axis + 2) % 3;
        let mut hit = None;
        for patch in patches {
            work.charge(1)?;
            if patch.axis != self.axis || patch.lo[self.axis] != level {
                continue;
            }
            if patch.lo[u].cmp_checked(self.u[0])? == Ordering::Greater
                || patch.hi[u].cmp_checked(self.u[1])? == Ordering::Less
                || patch.lo[v].cmp_checked(self.v[0])? == Ordering::Greater
                || patch.hi[v].cmp_checked(self.v[1])? == Ordering::Less
            {
                continue;
            }
            // A validated boundary has no overlapping patch interiors.
            if hit.replace(patch.plane).is_some() {
                return Err(PolyhedralError::InvalidBoundary);
            }
        }
        Ok(hit)
    }

    fn face(
        &self,
        level: Q,
        support: Plane3h,
        positive_normal: bool,
        work: &mut PolyhedralWork<'_>,
    ) -> R<Facet> {
        let u = (self.axis + 1) % 3;
        let v = (self.axis + 2) % 3;
        let mut vertices = Vec::new();
        work.reserve(&mut vertices, 4)?;
        for (i, j) in [(0, 0), (1, 0), (1, 1), (0, 1)] {
            let mut point: P = [Q::ZERO; 3];
            point[self.axis] = level;
            point[u] = self.u[i];
            point[v] = self.v[j];
            vertices.push(point);
        }
        // With cyclic coordinate order, u cross v points along the positive
        // sweep axis. Entering material exposes the negative side.
        if !positive_normal {
            vertices.reverse();
        }
        let plane = if (support.coefficients()[self.axis] > 0) == positive_normal {
            support
        } else {
            support.reversed()
        };
        Ok(Facet { plane, vertices })
    }
}
