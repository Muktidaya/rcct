// SPDX-License-Identifier: Apache-2.0

use super::boundary::interior_sample;
use super::select::{emit_transition, split_polygon};
use super::{Facet, Operation3, Plane3h, PolyhedralError, PolyhedralWork, R, SolidBoundary};

/// Applies a regularized Boolean to validated boundaries with horizontal caps and vertical walls.
///
/// Every convex patch must have a support parallel to the z-axis or perpendicular
/// to it. Wall directions may differ between or within operands. The boundary
/// may be nonconvex and may include coplanar subdivisions. A successful result
/// owns its exact rational patches and can enter another call directly.
///
/// A sloped face on either operand returns [`PolyhedralError::UnsupportedFaceGeometry`],
/// including when its partner is empty. Edge-only or point-only unions that fail
/// the aggregate manifold check return [`PolyhedralError::NonManifoldResult`].
/// Checked arithmetic, work, storage and cancellation limits apply during
/// partitioning, classification and validation.
pub fn boolean_prismatic3(
    a: &SolidBoundary,
    b: &SolidBoundary,
    op: Operation3,
    work: &mut PolyhedralWork<'_>,
) -> R<SolidBoundary> {
    work.charge(1)?;
    let a_planes = support_planes(a, work)?;
    let b_planes = support_planes(b, work)?;
    let mut facets = Vec::new();
    select_boundary(a, b, &b_planes, true, op, &mut facets, work)?;
    select_boundary(b, a, &a_planes, false, op, &mut facets, work)?;
    SolidBoundary::from_facets(facets, work)
}

fn support_planes(solid: &SolidBoundary, work: &mut PolyhedralWork<'_>) -> R<Vec<Plane3h>> {
    let mut planes = Vec::new();
    for face in &solid.faces {
        work.charge(1)?;
        let [a, b, c, _] = face.plane.coefficients();
        if c != 0 && (a != 0 || b != 0) {
            return Err(PolyhedralError::UnsupportedFaceGeometry);
        }
        // For partitioning, opposite material normals define the same cut.
        work.charge(planes.len())?;
        let reversed = face.plane.reversed();
        if !planes.iter().any(|&p| p == face.plane || p == reversed) {
            work.reserve(&mut planes, 1)?;
            planes.push(face.plane);
        }
    }
    Ok(planes)
}

fn select_boundary(
    source: &SolidBoundary,
    other: &SolidBoundary,
    cutters: &[Plane3h],
    source_is_a: bool,
    op: Operation3,
    facets: &mut Vec<Facet>,
    work: &mut PolyhedralWork<'_>,
) -> R<()> {
    for face in &source.faces {
        let mut polygon = Vec::new();
        work.reserve(&mut polygon, face.indices.len())?;
        for &index in &face.indices {
            work.charge(1)?;
            polygon.push(source.vertices[index]);
        }
        let mut pieces = Vec::new();
        work.reserve(&mut pieces, 1)?;
        pieces.push(polygon);
        for &plane in cutters {
            work.charge(1)?;
            if face.plane == plane || face.plane == plane.reversed() {
                continue;
            }
            let mut next = Vec::new();
            for piece in pieces {
                split_polygon(piece, plane, &mut next, work)?;
            }
            pieces = next;
        }
        // Across an open fragment, opposing material can change only at an
        // opposing support cutting its interior. Artificial coplanar seams do
        // not change membership. Strict ray crossings therefore label the two
        // adjacent volumes even when the sample lies on a coplanar seam.
        for piece in pieces {
            let sides = if other.is_empty() {
                (false, false)
            } else {
                let point = interior_sample(face.plane, piece.iter().copied(), work)?;
                other.material_sides_at(point, face.plane, work)?
            };
            emit_transition(piece, face.plane, sides, source_is_a, op, facets, work)?;
        }
    }
    Ok(())
}
