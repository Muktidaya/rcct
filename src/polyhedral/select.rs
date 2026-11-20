// SPDX-License-Identifier: Apache-2.0
//! Partition convex faces by supporting planes; select oriented material transitions.

use std::cmp::Ordering;

use super::exact::{P, mix};
use super::{ConvexSolid, Facet, Operation3, Plane3h, PolyhedralError, PolyhedralWork, R};

/// Membership of the opposing solid immediately inside and outside the
/// source face. Each processed halfspace can only remove possible material.
#[derive(Clone, Copy)]
struct Sides {
    inward: bool,
    outward: bool,
}

impl Sides {
    const OUTSIDE: Self = Self {
        inward: false,
        outward: false,
    };
}

struct Piece {
    vertices: Vec<P>,
    other: Sides,
}

fn append<T>(out: &mut Vec<T>, item: T, work: &mut PolyhedralWork<'_>) -> R<()> {
    work.reserve(out, 1)?;
    out.push(item);
    Ok(())
}

fn partition_polygon(
    vertices: Vec<P>,
    cutter: Plane3h,
    work: &mut PolyhedralWork<'_>,
    mut emit: impl FnMut(Vec<P>, Ordering, &mut PolyhedralWork<'_>) -> R<()>,
) -> R<()> {
    let mut negative = false;
    let mut positive = false;
    for &p in &vertices {
        work.charge(1)?;
        let sign = cutter.eval(p)?.sign();
        negative |= sign < 0;
        positive |= sign > 0;
    }

    // A relative-interior point of a convex polygon can be expressed as a
    // strictly positive combination of its vertices. A weak one-sided sign
    // with any strict vertex therefore determines its open fragment.
    if !(negative && positive) {
        return emit(vertices, positive.cmp(&negative), work);
    }

    // Both open halfspaces contain vertices. Clipping a convex polygon at
    // one line introduces at most two additional boundary points.
    let bound = vertices
        .len()
        .checked_add(2)
        .ok_or(PolyhedralError::ResourceLimit)?;
    let mut inside = Vec::new();
    let mut outside = Vec::new();
    work.reserve(&mut inside, bound)?;
    work.reserve(&mut outside, bound)?;
    let mut prev = *vertices.last().ok_or(PolyhedralError::InvalidBoundary)?;
    let mut prev_value = cutter.eval(prev)?;
    for point in vertices {
        work.charge(1)?;
        let value = cutter.eval(point)?;
        if prev_value.sign() * value.sign() < 0 {
            let t = prev_value.div(prev_value.sub(value)?)?;
            let intersection = mix(prev, point, t)?;
            inside.push(intersection);
            outside.push(intersection);
        }
        if value.sign() <= 0 {
            inside.push(point);
        }
        if value.sign() >= 0 {
            outside.push(point);
        }
        prev = point;
        prev_value = value;
    }
    emit(inside, Ordering::Less, work)?;
    emit(outside, Ordering::Greater, work)
}

/// Appends the full-dimensional parts of the convex patch on either side of the cutter.
pub(super) fn split_polygon(
    vertices: Vec<P>,
    cutter: Plane3h,
    next: &mut Vec<Vec<P>>,
    work: &mut PolyhedralWork<'_>,
) -> R<()> {
    partition_polygon(vertices, cutter, work, |vertices, _, work| {
        append(next, vertices, work)
    })
}

fn partition(
    piece: Piece,
    source: Plane3h,
    cutter: Plane3h,
    next: &mut Vec<Piece>,
    work: &mut PolyhedralWork<'_>,
) -> R<()> {
    partition_polygon(piece.vertices, cutter, work, |vertices, side, work| {
        let mut other = piece.other;
        match side {
            Ordering::Less => {}
            Ordering::Greater => other = Sides::OUTSIDE,
            // Admission uses primitive oriented coefficients, so coplanar
            // supports agree up to sign.
            Ordering::Equal if source == cutter => other.outward = false,
            Ordering::Equal if source == cutter.reversed() => other.inward = false,
            Ordering::Equal => return Err(PolyhedralError::InvalidBoundary),
        }
        append(next, Piece { vertices, other }, work)
    })
}

/// The opposing material labels are ordered (inward, outward) relative to the source face.
pub(super) fn emit_transition(
    vertices: Vec<P>,
    plane: Plane3h,
    other: (bool, bool),
    source_is_a: bool,
    op: Operation3,
    facets: &mut Vec<Facet>,
    work: &mut PolyhedralWork<'_>,
) -> R<()> {
    work.charge(1)?;
    let (inward, outward) = other;
    // When both solids have a boundary here, A owns the shared open patch.
    if !source_is_a && inward != outward {
        return Ok(());
    }
    let (inner, outer) = if source_is_a {
        (op.accepts(true, inward), op.accepts(false, outward))
    } else {
        (op.accepts(inward, true), op.accepts(outward, false))
    };
    if inner == outer {
        return Ok(());
    }
    let mut facet = Facet { plane, vertices };
    if outer {
        work.charge(facet.vertices.len())?;
        facet.vertices.reverse();
        facet.plane = facet.plane.reversed();
    }
    append(facets, facet, work)
}

fn select_source(
    source: &ConvexSolid,
    other: &ConvexSolid,
    source_is_a: bool,
    op: Operation3,
    facets: &mut Vec<Facet>,
    work: &mut PolyhedralWork<'_>,
) -> R<()> {
    for face in &source.faces {
        work.charge(face.vertices.len())?;
        let mut vertices = Vec::new();
        work.reserve(&mut vertices, face.vertices.len())?;
        vertices.extend_from_slice(&face.vertices);
        let initially_possible = !other.is_empty();
        let mut pieces = Vec::new();
        append(
            &mut pieces,
            Piece {
                vertices,
                other: Sides {
                    inward: initially_possible,
                    outward: initially_possible,
                },
            },
            work,
        )?;
        for &plane in &other.planes {
            let mut next = Vec::new();
            for piece in pieces {
                partition(piece, face.plane, plane, &mut next, work)?;
            }
            pieces = next;
        }
        for piece in pieces {
            emit_transition(
                piece.vertices,
                face.plane,
                (piece.other.inward, piece.other.outward),
                source_is_a,
                op,
                facets,
                work,
            )?;
        }
    }
    Ok(())
}

pub(super) fn select(
    a: &ConvexSolid,
    b: &ConvexSolid,
    op: Operation3,
    work: &mut PolyhedralWork<'_>,
) -> R<Vec<Facet>> {
    let mut facets = Vec::new();
    select_source(a, b, true, op, &mut facets, work)?;
    select_source(b, a, false, op, &mut facets, work)?;
    Ok(facets)
}
