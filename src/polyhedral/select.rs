// SPDX-License-Identifier: Apache-2.0
//! Partition convex faces by supporting planes; select oriented material transitions.

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

fn partition(
    mut piece: Piece,
    source: Plane3h,
    cutter: Plane3h,
    next: &mut Vec<Piece>,
    work: &mut PolyhedralWork<'_>,
) -> R<()> {
    let mut negative = false;
    let mut positive = false;
    for &p in &piece.vertices {
        work.charge(1)?;
        let sign = cutter.eval(p)?.sign();
        negative |= sign < 0;
        positive |= sign > 0;
    }

    // A relative-interior point of a convex polygon can be expressed as a
    // strictly positive combination of its vertices. A weak one-sided sign
    // with any strict vertex therefore determines its open fragment.
    if !(negative && positive) {
        if positive {
            piece.other = Sides::OUTSIDE;
        } else if !negative {
            // Both planes contain the entire open 2D patch. Admission
            // reduces integer coefficients to primitive oriented tuples,
            // so these are the only two ways their supports can agree.
            if source == cutter {
                piece.other.outward = false;
            } else if source == cutter.reversed() {
                piece.other.inward = false;
            } else {
                return Err(PolyhedralError::InvalidBoundary);
            }
        }
        return append(next, piece, work);
    }

    // Both open halfspaces contain vertices. Clipping a convex polygon at
    // one line introduces at most two additional boundary points.
    let bound = piece
        .vertices
        .len()
        .checked_add(2)
        .ok_or(PolyhedralError::ResourceLimit)?;
    let mut inside = Vec::new();
    let mut outside = Vec::new();
    work.reserve(&mut inside, bound)?;
    work.reserve(&mut outside, bound)?;
    let mut prev = *piece
        .vertices
        .last()
        .ok_or(PolyhedralError::InvalidBoundary)?;
    let mut prev_value = cutter.eval(prev)?;
    for point in piece.vertices {
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
    append(
        next,
        Piece {
            vertices: inside,
            other: piece.other,
        },
        work,
    )?;
    append(
        next,
        Piece {
            vertices: outside,
            other: Sides::OUTSIDE,
        },
        work,
    )
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
            work.charge(1)?;
            let sides = piece.other;
            // Exactly one opposing material side means both solids have a
            // face at this open patch; the A pass owns that shared patch.
            if !source_is_a && sides.inward != sides.outward {
                continue;
            }
            let (inner, outer) = if source_is_a {
                (
                    op.accepts(true, sides.inward),
                    op.accepts(false, sides.outward),
                )
            } else {
                (
                    op.accepts(sides.inward, true),
                    op.accepts(sides.outward, false),
                )
            };
            if inner == outer {
                continue;
            }
            let mut facet = Facet {
                plane: face.plane,
                vertices: piece.vertices,
            };
            if outer {
                work.charge(facet.vertices.len())?;
                facet.vertices.reverse();
                facet.plane = facet.plane.reversed();
            }
            append(facets, facet, work)?;
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
