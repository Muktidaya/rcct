// SPDX-License-Identifier: Apache-2.0

//! Bounded embedded graphs of original nonzero straight segments.

use super::{
    Segment2, SegmentIntersection, SegmentParameter2, SegmentPointEvent, endpoint_touch_witness,
    intersect_segments, overlap_witness, parameter_for_input_point, point_on_segment_witness,
    proper_crossing_witness,
};
use std::cmp::Ordering;
use std::fmt;

/// Independent caller-chosen caps for one arrangement build.
///
/// These cap logical item counts and selected comparison calls, not exact heap
/// bytes, all limb operations, allocator behavior, total instructions, or time.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArrangementLimits2 {
    /// Maximum number of original nonzero segments.
    pub max_segments: usize,
    /// Maximum unordered source pairs examined, including disjoint pairs.
    pub max_pairs: usize,
    /// Maximum endpoint and pair-event cut records before duplicate removal.
    pub max_cuts: usize,
    /// Maximum distinct exact point identities in the returned graph.
    pub max_vertices: usize,
    /// Maximum distinct undirected geometric edge pieces.
    pub max_edges: usize,
    /// Maximum source-to-edge traversal records.
    pub max_edge_uses: usize,
    /// Maximum sorting, deduplication, vertex-search, and edge-search comparisons.
    pub max_comparisons: usize,
}

/// The count whose configured limit prevented a build.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArrangementResource2 {
    /// Input segments.
    Segments,
    /// Unordered source pairs.
    Pairs,
    /// Cut records before deduplication.
    Cuts,
    /// Unique vertices.
    Vertices,
    /// Unique undirected edges.
    Edges,
    /// Source-edge traversal records.
    EdgeUses,
    /// Explicit sort, deduplication and graph-search comparisons.
    Comparisons,
}

/// A failed arrangement build. No partial graph is returned.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArrangementError2 {
    /// A caller-specified item or comparison cap was reached.
    LimitExceeded(ArrangementResource2),
    /// The allocator refused a fallible reservation.
    AllocationFailed,
    /// An internal relationship required by the construction did not hold.
    InvariantViolation,
}

impl fmt::Display for ArrangementError2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LimitExceeded(resource) => write!(f, "arrangement limit exceeded: {resource:?}"),
            Self::AllocationFailed => f.write_str("arrangement allocation failed"),
            Self::InvariantViolation => f.write_str("arrangement invariant failed"),
        }
    }
}
impl std::error::Error for ArrangementError2 {}

/// One shared exact point identity, retained through an original parameter.
#[derive(Clone, Copy)]
pub struct ArrangementVertex2 {
    point: SegmentParameter2,
}
impl ArrangementVertex2 {
    /// Returns one exact retained construction of this point.
    pub const fn point(&self) -> SegmentParameter2 {
        self.point
    }
}

/// One undirected geometric piece between distinct vertex indices.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArrangementEdge2 {
    vertices: [usize; 2],
}
impl ArrangementEdge2 {
    /// Returns endpoint indices in increasing index order, not geometric order.
    pub const fn vertices(&self) -> [usize; 2] {
        self.vertices
    }
}

/// One original source's directed traversal of an arrangement edge.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArrangementEdgeUse2 {
    source: usize,
    edge: usize,
    forward: bool,
}
impl ArrangementEdgeUse2 {
    /// Returns the original input segment index.
    pub const fn source(&self) -> usize {
        self.source
    }
    /// Returns the index of the shared undirected edge.
    pub const fn edge(&self) -> usize {
        self.edge
    }
    /// Whether source traversal follows the edge's stored endpoint order.
    pub const fn forward(&self) -> bool {
        self.forward
    }
}

/// An embedded straight-segment graph with exact shared vertices and provenance.
///
/// Indices and ordering are deterministic for the same ordered inputs, but are
/// not stable under input permutation. Coincident edge pieces are merged and
/// retain one use per source traversal. A separate half-edge embedding can be
/// built from this graph. Faces, isolated point inputs, arbitrary constructed
/// segments, and booleans are not provided.
pub struct Arrangement2 {
    sources: Vec<Segment2>,
    vertices: Vec<ArrangementVertex2>,
    edges: Vec<ArrangementEdge2>,
    uses: Vec<ArrangementEdgeUse2>,
}
impl Arrangement2 {
    /// Returns original source segments in input order.
    pub fn sources(&self) -> &[Segment2] {
        &self.sources
    }
    /// Returns unique exact point identities.
    pub fn vertices(&self) -> &[ArrangementVertex2] {
        &self.vertices
    }
    /// Returns unique undirected edge pieces.
    pub fn edges(&self) -> &[ArrangementEdge2] {
        &self.edges
    }
    /// Returns source traversals, grouped by source and in its traversal order.
    pub fn edge_uses(&self) -> &[ArrangementEdgeUse2] {
        &self.uses
    }
}

#[derive(Clone, Copy)]
struct Cut {
    source: usize,
    parameter: SegmentParameter2,
}

struct ComparisonBudget {
    remaining: usize,
}
impl ComparisonBudget {
    fn charge(&mut self) -> Result<(), ArrangementError2> {
        if self.remaining == 0 {
            return Err(ArrangementError2::LimitExceeded(
                ArrangementResource2::Comparisons,
            ));
        }
        self.remaining -= 1;
        Ok(())
    }
}

fn push_bounded<T>(
    items: &mut Vec<T>,
    value: T,
    max: usize,
    resource: ArrangementResource2,
) -> Result<(), ArrangementError2> {
    if items.len() >= max {
        return Err(ArrangementError2::LimitExceeded(resource));
    }
    items
        .try_reserve_exact(1)
        .map_err(|_| ArrangementError2::AllocationFailed)?;
    items.push(value);
    Ok(())
}

fn push_cut(
    cuts: &mut Vec<Cut>,
    source: usize,
    parameter: SegmentParameter2,
    limit: usize,
) -> Result<(), ArrangementError2> {
    push_bounded(
        cuts,
        Cut { source, parameter },
        limit,
        ArrangementResource2::Cuts,
    )
}

/// Builds an exact embedded graph from finite, nonzero raw input segments.
///
/// Endpoints, proper crossings, touches, and overlap boundaries become cuts.
/// Exact point comparison merges shared vertices; coincident pieces become one
/// edge with a directed use for every original source. Builds stage all state
/// privately and return no graph on an error. The input slice is not modified.
///
/// Pair and comparison caps bound the described loops; cuts, vertices, edges and
/// uses have independent item caps. Allocation is requested fallibly, but the
/// limits do not guarantee an exact byte bound or handle process aborts. Inputs
/// are raw `Segment2`s only, and no face or general rational segment is created.
pub fn build_arrangement2(
    inputs: &[Segment2],
    limits: ArrangementLimits2,
) -> Result<Arrangement2, ArrangementError2> {
    if inputs.len() > limits.max_segments {
        return Err(ArrangementError2::LimitExceeded(
            ArrangementResource2::Segments,
        ));
    }
    let n = inputs.len();
    let pairs = (n as u128).checked_mul(n.saturating_sub(1) as u128).ok_or(
        ArrangementError2::LimitExceeded(ArrangementResource2::Pairs),
    )? / 2;
    if pairs > limits.max_pairs as u128 {
        return Err(ArrangementError2::LimitExceeded(
            ArrangementResource2::Pairs,
        ));
    }
    if n.checked_mul(2).is_none_or(|count| count > limits.max_cuts) {
        return Err(ArrangementError2::LimitExceeded(ArrangementResource2::Cuts));
    }
    let mut cuts = Vec::new();
    for (source, &segment) in inputs.iter().enumerate() {
        push_cut(
            &mut cuts,
            source,
            parameter_for_input_point(segment, segment.start()),
            limits.max_cuts,
        )?;
        push_cut(
            &mut cuts,
            source,
            parameter_for_input_point(segment, segment.end()),
            limits.max_cuts,
        )?;
    }
    for a in 0..n {
        for b in a + 1..n {
            let first = inputs[a];
            let second = inputs[b];
            match intersect_segments(first, second) {
                SegmentIntersection::Disjoint => {}
                SegmentIntersection::Point(kind) => {
                    let (pa, pb) = match kind {
                        SegmentPointEvent::ProperCrossing => (
                            proper_crossing_witness(first, second)
                                .ok_or(ArrangementError2::InvariantViolation)?
                                .parameter(),
                            proper_crossing_witness(second, first)
                                .ok_or(ArrangementError2::InvariantViolation)?
                                .parameter(),
                        ),
                        SegmentPointEvent::BoundaryTouch => (
                            endpoint_touch_witness(first, second)
                                .ok_or(ArrangementError2::InvariantViolation)?
                                .parameter(),
                            endpoint_touch_witness(second, first)
                                .ok_or(ArrangementError2::InvariantViolation)?
                                .parameter(),
                        ),
                    };
                    push_cut(&mut cuts, a, pa, limits.max_cuts)?;
                    push_cut(&mut cuts, b, pb, limits.max_cuts)?;
                }
                SegmentIntersection::Overlap => {
                    let overlap = overlap_witness(first, second)
                        .ok_or(ArrangementError2::InvariantViolation)?;
                    for point in [overlap.start_point(), overlap.end_point()] {
                        let pa = point_on_segment_witness(first, point)
                            .ok_or(ArrangementError2::InvariantViolation)?
                            .parameter();
                        let pb = point_on_segment_witness(second, point)
                            .ok_or(ArrangementError2::InvariantViolation)?
                            .parameter();
                        push_cut(&mut cuts, a, pa, limits.max_cuts)?;
                        push_cut(&mut cuts, b, pb, limits.max_cuts)?;
                    }
                }
            }
        }
    }

    let mut comparisons = ComparisonBudget {
        remaining: limits.max_comparisons,
    };
    // A fallible insertion sort charges before each comparison and does not
    // allocate. Its quadratic worst case is bounded by the comparison cap.
    for i in 1..cuts.len() {
        let mut j = i;
        while j > 0 {
            comparisons.charge()?;
            let left = cuts[j - 1];
            let right = cuts[j];
            let order = match left.source.cmp(&right.source) {
                Ordering::Equal => left
                    .parameter
                    .compare_on_first(&right.parameter)
                    .ok_or(ArrangementError2::InvariantViolation)?,
                other => other,
            };
            if order != Ordering::Greater {
                break;
            }
            cuts.swap(j - 1, j);
            j -= 1;
        }
    }

    let mut sources = Vec::new();
    sources
        .try_reserve_exact(n)
        .map_err(|_| ArrangementError2::AllocationFailed)?;
    sources.extend_from_slice(inputs);
    let mut vertices: Vec<ArrangementVertex2> = Vec::new();
    let mut edges: Vec<ArrangementEdge2> = Vec::new();
    let mut uses = Vec::new();
    let mut previous: Option<(Cut, usize)> = None;
    for cut in cuts {
        if let Some((last, _)) = previous
            && last.source == cut.source
        {
            comparisons.charge()?;
            if last
                .parameter
                .compare_on_first(&cut.parameter)
                .ok_or(ArrangementError2::InvariantViolation)?
                == Ordering::Equal
            {
                continue;
            }
        }
        let mut vertex = None;
        for (index, candidate) in vertices.iter().enumerate() {
            comparisons.charge()?;
            if candidate.point.same_point(&cut.parameter) {
                vertex = Some(index);
                break;
            }
        }
        let vertex = match vertex {
            Some(index) => index,
            None => {
                let index = vertices.len();
                push_bounded(
                    &mut vertices,
                    ArrangementVertex2 {
                        point: cut.parameter,
                    },
                    limits.max_vertices,
                    ArrangementResource2::Vertices,
                )?;
                index
            }
        };
        if let Some((last, before)) = previous
            && last.source == cut.source
        {
            if before == vertex {
                return Err(ArrangementError2::InvariantViolation);
            }
            let endpoints = [before.min(vertex), before.max(vertex)];
            let mut edge = None;
            for (index, candidate) in edges.iter().enumerate() {
                comparisons.charge()?;
                if candidate.vertices == endpoints {
                    edge = Some(index);
                    break;
                }
            }
            let edge = match edge {
                Some(index) => index,
                None => {
                    let index = edges.len();
                    push_bounded(
                        &mut edges,
                        ArrangementEdge2 {
                            vertices: endpoints,
                        },
                        limits.max_edges,
                        ArrangementResource2::Edges,
                    )?;
                    index
                }
            };
            push_bounded(
                &mut uses,
                ArrangementEdgeUse2 {
                    source: cut.source,
                    edge,
                    forward: before == endpoints[0],
                },
                limits.max_edge_uses,
                ArrangementResource2::EdgeUses,
            )?;
        }
        previous = Some((cut, vertex));
    }
    Ok(Arrangement2 {
        sources,
        vertices,
        edges,
        uses,
    })
}
