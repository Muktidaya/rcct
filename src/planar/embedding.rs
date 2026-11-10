// SPDX-License-Identifier: Apache-2.0

//! Exact radial order and directed boundary walks for an arrangement graph.

use super::{Arrangement2, Orientation2, Segment2, exact_cross};
use std::cmp::Ordering;
use std::fmt;

/// Caller-chosen logical limits for embedding one arrangement graph.
///
/// These are item and selected-comparison counts, not exact memory, total
/// instruction, allocator or time budgets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EmbeddingLimits2 {
    /// Maximum number of input graph vertices.
    pub max_vertices: usize,
    /// Maximum directed half-edges, exactly twice the graph edge count.
    pub max_half_edges: usize,
    /// Maximum source-to-edge records scanned for representative directions.
    pub max_edge_uses: usize,
    /// Maximum sorting and duplicate-ray comparison calls.
    pub max_comparisons: usize,
    /// Maximum distinct `next`-permutation boundary walks.
    pub max_boundary_walks: usize,
    /// Maximum half-edges visited while partitioning boundary walks.
    pub max_walk_steps: usize,
}

/// A logical embedding resource whose cap was exceeded.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EmbeddingResource2 {
    /// Graph vertices.
    Vertices,
    /// Directed half-edges.
    HalfEdges,
    /// Source-to-edge records scanned for representative directions.
    EdgeUses,
    /// Sorting and duplicate-ray comparisons.
    Comparisons,
    /// Boundary walks.
    BoundaryWalks,
    /// Visited half-edges.
    WalkSteps,
}

/// Embedding failure. No partial embedding is returned.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EmbeddingError2 {
    /// A caller-provided cap was exceeded.
    LimitExceeded(EmbeddingResource2),
    /// A fallible reservation was refused.
    AllocationFailed,
    /// A graph or embedding invariant required by this construction failed.
    InvariantViolation,
}
impl fmt::Display for EmbeddingError2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LimitExceeded(resource) => write!(f, "embedding limit exceeded: {resource:?}"),
            Self::AllocationFailed => f.write_str("embedding allocation failed"),
            Self::InvariantViolation => f.write_str("embedding invariant failed"),
        }
    }
}
impl std::error::Error for EmbeddingError2 {}

/// One directed side of an arrangement edge.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HalfEdge2 {
    origin: usize,
    destination: usize,
    edge: usize,
    twin: usize,
    next: usize,
}
impl HalfEdge2 {
    /// Returns the origin vertex index.
    pub const fn origin(&self) -> usize {
        self.origin
    }
    /// Returns the destination vertex index.
    pub const fn destination(&self) -> usize {
        self.destination
    }
    /// Returns the underlying undirected edge index.
    pub const fn edge(&self) -> usize {
        self.edge
    }
    /// Returns the reverse directed half-edge index.
    pub const fn twin(&self) -> usize {
        self.twin
    }
    /// Returns the next directed half-edge on the same left-side boundary walk.
    pub const fn next(&self) -> usize {
        self.next
    }
}

/// Radial order and boundary walks tied to one immutable arrangement graph.
///
/// Outgoing half-edges are counterclockwise from the positive x direction.
/// `next` selects the clockwise predecessor of the incoming edge's twin at its
/// destination. Walks partition the directed half-edges, but are not themselves
/// faces: disconnected components, holes, and bridges need additional treatment.
pub struct ArrangementEmbedding2<'a> {
    arrangement: &'a Arrangement2,
    half_edges: Vec<HalfEdge2>,
    rotation: Vec<usize>,
    rotation_offsets: Vec<usize>,
    walk_edges: Vec<usize>,
    walk_offsets: Vec<usize>,
    representatives: Vec<Option<(usize, bool)>>,
}
impl<'a> ArrangementEmbedding2<'a> {
    /// Returns the graph this embedding borrows.
    pub const fn arrangement(&self) -> &'a Arrangement2 {
        self.arrangement
    }
    /// Returns directed half-edges; indices are local to this embedding.
    pub fn half_edges(&self) -> &[HalfEdge2] {
        &self.half_edges
    }
    /// Returns outgoing half-edge indices in counterclockwise order for a vertex.
    pub fn outgoing(&self, vertex: usize) -> Option<&[usize]> {
        let next = vertex.checked_add(1)?;
        let start = *self.rotation_offsets.get(vertex)?;
        let end = *self.rotation_offsets.get(next)?;
        Some(&self.rotation[start..end])
    }
    /// Returns the number of directed boundary walks.
    pub fn boundary_walk_count(&self) -> usize {
        self.walk_offsets.len() - 1
    }
    /// Returns the half-edge indices of a closed `next` walk.
    pub fn boundary_walk(&self, walk: usize) -> Option<&[usize]> {
        let next = walk.checked_add(1)?;
        let start = *self.walk_offsets.get(walk)?;
        let end = *self.walk_offsets.get(next)?;
        Some(&self.walk_edges[start..end])
    }
    pub(super) fn directed_support(&self, h: usize) -> Option<(Segment2, bool)> {
        let half = self.half_edges.get(h)?;
        let (source, forward) = self.representatives.get(half.edge).copied().flatten()?;
        Some((
            *self.arrangement.sources().get(source)?,
            forward == h.is_multiple_of(2),
        ))
    }
}

fn reserved<T>(capacity: usize) -> Result<Vec<T>, EmbeddingError2> {
    let mut result = Vec::new();
    result
        .try_reserve_exact(capacity)
        .map_err(|_| EmbeddingError2::AllocationFailed)?;
    Ok(result)
}
fn charge(remaining: &mut usize) -> Result<(), EmbeddingError2> {
    if *remaining == 0 {
        return Err(EmbeddingError2::LimitExceeded(
            EmbeddingResource2::Comparisons,
        ));
    }
    *remaining -= 1;
    Ok(())
}

// A directed half-edge follows or reverses a raw support. No coordinate
// subtraction in f64 is used, including when the displacement would overflow.
fn direction_signs(segment: Segment2, along: bool) -> (Ordering, Ordering) {
    let cmp = |a: f64, b: f64| a.partial_cmp(&b).expect("finite checked points");
    let mut x = cmp(segment.end().x(), segment.start().x());
    let mut y = cmp(segment.end().y(), segment.start().y());
    if !along {
        x = x.reverse();
        y = y.reverse();
    }
    (x, y)
}
fn upper_half(segment: Segment2, along: bool) -> bool {
    let (x, y) = direction_signs(segment, along);
    y == Ordering::Greater || (y == Ordering::Equal && x == Ordering::Greater)
}
fn compare_rays(a: (Segment2, bool), b: (Segment2, bool)) -> Ordering {
    match (upper_half(a.0, a.1), upper_half(b.0, b.1)) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        _ => direction_cross(a, b).reverse(),
    }
}

fn direction_cross(a: (Segment2, bool), b: (Segment2, bool)) -> Ordering {
    let mut cross = exact_cross(a.0.end(), a.0.start(), b.0.end(), b.0.start()).sign();
    if a.1 != b.1 {
        cross = cross.reverse();
    }
    cross
}

/// Builds a bounded half-edge rotation and closed boundary-walk partition.
///
/// Ray order is exact for original finite binary64 source directions, with no
/// rounded vector subtraction. A private draft is returned only on success;
/// the borrowed graph is unchanged on all recoverable errors. Faces, grouping
/// walks into regions, and numeric winding/area are not computed. Count limits
/// do not bound exact allocated bytes, total instructions, or process aborts.
pub fn embed_arrangement2(
    arrangement: &Arrangement2,
    limits: EmbeddingLimits2,
) -> Result<ArrangementEmbedding2<'_>, EmbeddingError2> {
    let vertices = arrangement.vertices().len();
    if vertices > limits.max_vertices {
        return Err(EmbeddingError2::LimitExceeded(EmbeddingResource2::Vertices));
    }
    let edge_count = arrangement.edges().len();
    if arrangement.edge_uses().len() > limits.max_edge_uses {
        return Err(EmbeddingError2::LimitExceeded(EmbeddingResource2::EdgeUses));
    }
    let count = edge_count
        .checked_mul(2)
        .ok_or(EmbeddingError2::LimitExceeded(
            EmbeddingResource2::HalfEdges,
        ))?;
    if count > limits.max_half_edges {
        return Err(EmbeddingError2::LimitExceeded(
            EmbeddingResource2::HalfEdges,
        ));
    }
    if count > limits.max_walk_steps {
        return Err(EmbeddingError2::LimitExceeded(
            EmbeddingResource2::WalkSteps,
        ));
    }
    let offset_count = vertices
        .checked_add(1)
        .ok_or(EmbeddingError2::LimitExceeded(EmbeddingResource2::Vertices))?;
    let walk_capacity = count.min(limits.max_boundary_walks).checked_add(1).ok_or(
        EmbeddingError2::LimitExceeded(EmbeddingResource2::BoundaryWalks),
    )?;

    let mut representatives = reserved(edge_count)?;
    representatives.resize(edge_count, None::<(usize, bool)>);
    for use_ in arrangement.edge_uses() {
        let slot = representatives
            .get_mut(use_.edge())
            .ok_or(EmbeddingError2::InvariantViolation)?;
        if use_.source() >= arrangement.sources().len() {
            return Err(EmbeddingError2::InvariantViolation);
        }
        if slot.is_none() {
            *slot = Some((use_.source(), use_.forward()));
        }
    }
    let mut half_edges = reserved(count)?;
    for (edge, item) in arrangement.edges().iter().enumerate() {
        if representatives[edge].is_none() {
            return Err(EmbeddingError2::InvariantViolation);
        }
        let [a, b] = item.vertices();
        if a >= vertices || b >= vertices || a >= b {
            return Err(EmbeddingError2::InvariantViolation);
        }
        let h = edge * 2;
        half_edges.push(HalfEdge2 {
            origin: a,
            destination: b,
            edge,
            twin: h + 1,
            next: usize::MAX,
        });
        half_edges.push(HalfEdge2 {
            origin: b,
            destination: a,
            edge,
            twin: h,
            next: usize::MAX,
        });
    }
    let ray = |h: usize| {
        let (source, forward) =
            representatives[half_edges[h].edge].expect("edge representative validated");
        (
            arrangement.sources()[source],
            forward == h.is_multiple_of(2),
        )
    };
    let mut rotation = reserved(count)?;
    rotation.extend(0..count);
    let mut comparisons = limits.max_comparisons;
    for i in 1..count {
        let mut j = i;
        while j > 0 {
            charge(&mut comparisons)?;
            let (a, b) = (rotation[j - 1], rotation[j]);
            let ordering = match half_edges[a].origin.cmp(&half_edges[b].origin) {
                Ordering::Equal => compare_rays(ray(a), ray(b)),
                other => other,
            };
            if ordering != Ordering::Greater {
                break;
            }
            rotation.swap(j - 1, j);
            j -= 1;
        }
    }
    for pair in rotation.windows(2) {
        if half_edges[pair[0]].origin == half_edges[pair[1]].origin {
            charge(&mut comparisons)?;
            if compare_rays(ray(pair[0]), ray(pair[1])) == Ordering::Equal {
                return Err(EmbeddingError2::InvariantViolation);
            }
        }
    }
    let mut rotation_offsets = reserved(offset_count)?;
    rotation_offsets.resize(offset_count, 0_usize);
    for &h in &rotation {
        rotation_offsets[half_edges[h].origin + 1] += 1;
    }
    for vertex in 0..vertices {
        rotation_offsets[vertex + 1] += rotation_offsets[vertex];
    }
    for vertex in 0..vertices {
        let (start, end) = (rotation_offsets[vertex], rotation_offsets[vertex + 1]);
        for index in start..end {
            let outgoing = rotation[index];
            let clockwise = rotation[if index == start { end - 1 } else { index - 1 }];
            let incoming = half_edges[outgoing].twin;
            half_edges[incoming].next = clockwise;
        }
    }
    let mut visited = reserved(count)?;
    visited.resize(count, false);
    let mut walk_edges = reserved(count)?;
    let mut walk_offsets = reserved(walk_capacity)?;
    walk_offsets.push(0);
    for start in 0..count {
        if visited[start] {
            continue;
        }
        if walk_offsets.len() > limits.max_boundary_walks {
            return Err(EmbeddingError2::LimitExceeded(
                EmbeddingResource2::BoundaryWalks,
            ));
        }
        let mut current = start;
        loop {
            if current >= count {
                return Err(EmbeddingError2::InvariantViolation);
            }
            if visited[current] {
                if current != start {
                    return Err(EmbeddingError2::InvariantViolation);
                }
                break;
            }
            visited[current] = true;
            walk_edges.push(current);
            current = half_edges[current].next;
        }
        walk_offsets.push(walk_edges.len());
    }
    Ok(ArrangementEmbedding2 {
        arrangement,
        half_edges,
        rotation,
        rotation_offsets,
        walk_edges,
        walk_offsets,
        representatives,
    })
}

/// Logical caps for classifying one simple boundary walk's orientation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WalkOrientationLimits2 {
    /// Maximum half-edges in the selected walk.
    pub max_steps: usize,
    /// Maximum repeated-vertex and exact extremum comparison calls.
    pub max_comparisons: usize,
}

/// A limit exceeded while classifying a walk.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WalkOrientationResource2 {
    /// Half-edges in the selected walk.
    Steps,
    /// Repeated-vertex and exact extremum comparisons.
    Comparisons,
}

/// Failure to classify a simple closed walk.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WalkOrientationError2 {
    /// The walk index is outside this embedding.
    UnknownWalk,
    /// The walk has fewer than three edges or repeats a vertex.
    NotSimple,
    /// A caller-provided resource cap was exceeded.
    LimitExceeded(WalkOrientationResource2),
    /// A fallible reservation was refused.
    AllocationFailed,
    /// An invariant required for a simple walk did not hold.
    InvariantViolation,
}
impl fmt::Display for WalkOrientationError2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownWalk => f.write_str("boundary walk index is out of range"),
            Self::NotSimple => f.write_str("boundary walk is not simple"),
            Self::LimitExceeded(resource) => {
                write!(f, "walk orientation limit exceeded: {resource:?}")
            }
            Self::AllocationFailed => f.write_str("walk orientation allocation failed"),
            Self::InvariantViolation => f.write_str("walk orientation invariant failed"),
        }
    }
}
impl std::error::Error for WalkOrientationError2 {}

/// Classifies the orientation of a simple closed boundary walk exactly.
///
/// Repeated-vertex and short walks are rejected, so bridges and self-touching
/// boundary walks do not receive a polygon orientation. The lexicographically
/// least vertex is a strict convex corner of a simple embedded straight-edge
/// polygon; its exact incoming/outgoing turn determines orientation. This does
/// not assign a face, group holes, compute area, or give a point winding number.
/// Limits count selected operations, not exact time or memory. Inputs are
/// unchanged on errors; allocation is requested fallibly.
pub fn simple_walk_orientation2(
    embedding: &ArrangementEmbedding2<'_>,
    walk: usize,
    limits: WalkOrientationLimits2,
) -> Result<Orientation2, WalkOrientationError2> {
    let walk = embedding
        .boundary_walk(walk)
        .ok_or(WalkOrientationError2::UnknownWalk)?;
    simple_cycle_orientation2(embedding, walk, limits)
}

// The caller must establish that the supplied half-edges form a closed,
// continuous walk before using this helper on anything but a stored walk.
pub(super) fn simple_cycle_orientation2(
    embedding: &ArrangementEmbedding2<'_>,
    walk: &[usize],
    limits: WalkOrientationLimits2,
) -> Result<Orientation2, WalkOrientationError2> {
    if walk.len() < 3 {
        return Err(WalkOrientationError2::NotSimple);
    }
    if walk.len() > limits.max_steps {
        return Err(WalkOrientationError2::LimitExceeded(
            WalkOrientationResource2::Steps,
        ));
    }
    let mut seen = Vec::new();
    seen.try_reserve_exact(walk.len())
        .map_err(|_| WalkOrientationError2::AllocationFailed)?;
    let mut remaining = limits.max_comparisons;
    let mut extremum = 0;
    for (index, &h) in walk.iter().enumerate() {
        let vertex = embedding.half_edges[h].origin;
        for &old in &seen {
            if remaining == 0 {
                return Err(WalkOrientationError2::LimitExceeded(
                    WalkOrientationResource2::Comparisons,
                ));
            }
            remaining -= 1;
            if old == vertex {
                return Err(WalkOrientationError2::NotSimple);
            }
        }
        seen.push(vertex);
        if index != 0 {
            if remaining == 0 {
                return Err(WalkOrientationError2::LimitExceeded(
                    WalkOrientationResource2::Comparisons,
                ));
            }
            remaining -= 1;
            let first = embedding.arrangement.vertices()[vertex].point();
            let second_vertex = embedding.half_edges[walk[extremum]].origin;
            let second = embedding.arrangement.vertices()[second_vertex].point();
            if first.compare_point_lexicographically(&second) == Ordering::Less {
                extremum = index;
            }
        }
    }
    let direction = |h: usize| {
        let (source, forward) = embedding.representatives[embedding.half_edges[h].edge]
            .expect("validated edge representative");
        (
            embedding.arrangement.sources()[source],
            forward == h.is_multiple_of(2),
        )
    };
    let incoming = walk[if extremum == 0 {
        walk.len() - 1
    } else {
        extremum - 1
    }];
    let outgoing = walk[extremum];
    match direction_cross(direction(incoming), direction(outgoing)) {
        Ordering::Less => Ok(Orientation2::Clockwise),
        Ordering::Greater => Ok(Orientation2::Counterclockwise),
        Ordering::Equal => Err(WalkOrientationError2::InvariantViolation),
    }
}
