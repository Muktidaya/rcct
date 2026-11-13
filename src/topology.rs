// SPDX-License-Identifier: Apache-2.0

//! Bounded combinatorial incidence, independent of geometric coordinates.
//!
//! Vertex IDs are contiguous indices `0..vertex_count`, and edge IDs are
//! indices into one graph. Raw IDs from another graph can alias valid indices:
//! callers must use IDs from the intended graph. Separate edges may share
//! endpoints. An owned wire borrows its immutable graph and contains its own
//! validated edge uses. A successful wire is a vertex-simple directed cycle,
//! not proof of geometric simplicity, planarity, manifoldness, or B-rep
//! validity. Single-edge loops and two-edge cycles are not supported here.

use std::fmt;

mod scoped;
pub use scoped::{
    BoundaryGroupError, BoundaryGroupLimits, BoundaryGroupResource, DeclaredFaceBoundaries,
    HandleError, ScopedEdgeId, ScopedEdgeUse, ScopedSnapshot, ScopedVertexId, ScopedWire,
    ScopedWireError, SnapshotError,
};
mod links;
pub use links::{
    VertexLink, VertexLinkError, VertexLinkKind, VertexLinkLimits, VertexLinkResource, VertexLinks,
    analyze_vertex_links,
};

/// An edge specified by its ordered, graph-local endpoint indices.
/// Validation occurs when an incidence graph is constructed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Edge {
    start: usize,
    end: usize,
}
impl Edge {
    /// Creates an edge description without validating its endpoints.
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
    /// Returns the first endpoint index.
    pub const fn start(self) -> usize {
        self.start
    }
    /// Returns the second endpoint index.
    pub const fn end(self) -> usize {
        self.end
    }
}

/// Logical item caps for graph construction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GraphLimits {
    /// Maximum declared vertex count, including isolated vertices.
    pub max_vertices: usize,
    /// Maximum number of edge records.
    pub max_edges: usize,
}

/// A graph construction failure. No partial graph is returned.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphError {
    /// The declared vertex count exceeds its limit.
    VertexLimit,
    /// The edge count exceeds its limit.
    EdgeLimit,
    /// An edge endpoint is outside the declared vertex range.
    UnknownVertex {
        /// Index of the invalid edge.
        edge: usize,
        /// The invalid vertex index.
        vertex: usize,
    },
    /// An edge has equal endpoint IDs; loop edges are not supported.
    LoopEdge {
        /// Index of the invalid edge.
        edge: usize,
    },
    /// Memory reservation failed.
    AllocationFailed,
}
impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid incidence graph: {self:?}")
    }
}
impl std::error::Error for GraphError {}

/// An immutable indexed multigraph; geometry is not stored or validated.
#[derive(Debug)]
pub struct IncidenceGraph {
    vertex_count: usize,
    edges: Vec<Edge>,
}
impl IncidenceGraph {
    /// Validates and copies edges. Isolated vertices and parallel edges are
    /// allowed, but edges joining a vertex to itself are unsupported.
    /// Limits are checked before scanning and allocation. IDs belong to this
    /// graph by convention; the type cannot detect indices from another graph.
    pub fn try_new(
        vertex_count: usize,
        edges: &[Edge],
        limits: GraphLimits,
    ) -> Result<Self, GraphError> {
        if vertex_count > limits.max_vertices {
            return Err(GraphError::VertexLimit);
        }
        if edges.len() > limits.max_edges {
            return Err(GraphError::EdgeLimit);
        }
        for (index, edge) in edges.iter().enumerate() {
            for vertex in [edge.start, edge.end] {
                if vertex >= vertex_count {
                    return Err(GraphError::UnknownVertex {
                        edge: index,
                        vertex,
                    });
                }
            }
            if edge.start == edge.end {
                return Err(GraphError::LoopEdge { edge: index });
            }
        }
        let mut stored = Vec::new();
        stored
            .try_reserve_exact(edges.len())
            .map_err(|_| GraphError::AllocationFailed)?;
        stored.extend_from_slice(edges);
        Ok(Self {
            vertex_count,
            edges: stored,
        })
    }
    /// Returns the declared count, including isolated vertices.
    pub const fn vertex_count(&self) -> usize {
        self.vertex_count
    }
    /// Returns edges in ID order.
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }
}

/// Traversal direction relative to the edge's stored endpoint order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    /// Traverse from start to end.
    Forward,
    /// Traverse from end to start.
    Reverse,
}

/// A graph-local edge ID and its traversal direction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EdgeUse {
    edge: usize,
    direction: Direction,
}
impl EdgeUse {
    /// Creates a use; its edge index is checked when constructing a wire.
    pub const fn new(edge: usize, direction: Direction) -> Self {
        Self { edge, direction }
    }
    /// Returns the edge index.
    pub const fn edge(self) -> usize {
        self.edge
    }
    /// Returns the traversal direction.
    pub const fn direction(self) -> Direction {
        self.direction
    }
    fn endpoints(self, graph: &IncidenceGraph) -> Option<(usize, usize)> {
        let edge = *graph.edges.get(self.edge)?;
        Some(match self.direction {
            Direction::Forward => (edge.start, edge.end),
            Direction::Reverse => (edge.end, edge.start),
        })
    }
}

/// Logical caps for validation and storage of a wire.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WireLimits {
    /// Maximum edge-use count.
    pub max_uses: usize,
    /// Maximum pairwise vertex identity comparisons, `n*(n-1)/2`.
    pub max_vertex_comparisons: usize,
}

/// A wire construction failure. No partial wire is returned.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WireError {
    /// This restricted API requires at least three uses.
    TooFewUses,
    /// The use count exceeds its cap.
    UseLimit,
    /// The required comparison count exceeds its cap or `usize` capacity.
    ComparisonLimit,
    /// A use refers to an edge outside the graph.
    UnknownEdge {
        /// Index in the supplied use sequence.
        at: usize,
        /// Invalid edge index.
        edge: usize,
    },
    /// The end of this use does not equal the next origin (including closure).
    Disconnected {
        /// Index of the first use of the mismatched pair.
        at: usize,
    },
    /// Two use origins have the same vertex ID.
    RepeatedVertex {
        /// Earlier position.
        first: usize,
        /// Later position.
        second: usize,
    },
    /// Memory reservation failed.
    AllocationFailed,
}
impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid closed wire: {self:?}")
    }
}
impl std::error::Error for WireError {}

/// A validated vertex-simple combinatorial directed cycle in one graph.
///
/// The original edge uses are copied and cannot be mutated through this type.
/// This validates IDs and incidence only; it does not validate an embedding,
/// geometric nonintersection, surface, manifold, shell, or solid.
#[derive(Debug)]
pub struct SimpleClosedWire<'g> {
    graph: &'g IncidenceGraph,
    uses: Vec<EdgeUse>,
}
impl<'g> SimpleClosedWire<'g> {
    /// Validates the sequence in the supplied graph. At least three uses are
    /// required. Every destination must equal the next origin, including the
    /// final-to-first pair, and each origin ID must be distinct.
    ///
    /// Item and pair-comparison limits are preflighted before scanning; the
    /// graph and input are never modified. Allocation is fallible, and failure
    /// returns no partial value. Limits count logical work, not allocator time,
    /// process memory availability, or all CPU instructions.
    pub fn try_new(
        graph: &'g IncidenceGraph,
        uses: &[EdgeUse],
        limits: WireLimits,
    ) -> Result<Self, WireError> {
        let n = uses.len();
        if n > limits.max_uses {
            return Err(WireError::UseLimit);
        }
        if n < 3 {
            return Err(WireError::TooFewUses);
        }
        // Divide first so an intermediate product need not fit when the final
        // binomial coefficient does. Checked multiplication handles the rest.
        let comparisons = comparison_count(n).ok_or(WireError::ComparisonLimit)?;
        if comparisons > limits.max_vertex_comparisons {
            return Err(WireError::ComparisonLimit);
        }
        for (at, &edge_use) in uses.iter().enumerate() {
            if edge_use.endpoints(graph).is_none() {
                return Err(WireError::UnknownEdge {
                    at,
                    edge: edge_use.edge,
                });
            }
        }
        for (at, &current) in uses.iter().enumerate() {
            let (_, end) = current.endpoints(graph).ok_or(WireError::UnknownEdge {
                at,
                edge: current.edge,
            })?;
            let next = if at + 1 == n { 0 } else { at + 1 };
            let (start, _) = uses[next].endpoints(graph).ok_or(WireError::UnknownEdge {
                at: next,
                edge: uses[next].edge,
            })?;
            if end != start {
                return Err(WireError::Disconnected { at });
            }
        }
        for (first, &first_use) in uses.iter().enumerate() {
            let (origin, _) = first_use.endpoints(graph).ok_or(WireError::UnknownEdge {
                at: first,
                edge: first_use.edge,
            })?;
            for (second, &second_use) in uses.iter().enumerate().skip(first + 1) {
                let (other, _) = second_use.endpoints(graph).ok_or(WireError::UnknownEdge {
                    at: second,
                    edge: second_use.edge,
                })?;
                if origin == other {
                    return Err(WireError::RepeatedVertex { first, second });
                }
            }
        }
        let mut stored = Vec::new();
        stored
            .try_reserve_exact(n)
            .map_err(|_| WireError::AllocationFailed)?;
        stored.extend_from_slice(uses);
        Ok(Self {
            graph,
            uses: stored,
        })
    }
    /// Returns the immutable graph used for validation.
    pub const fn graph(&self) -> &'g IncidenceGraph {
        self.graph
    }
    /// Returns the validated edge uses in traversal order.
    pub fn uses(&self) -> &[EdgeUse] {
        &self.uses
    }
}

/// Logical caps for collecting graph-edge incidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EdgeIncidenceLimits {
    /// Maximum graph edge count, including unused edges.
    pub max_edges: usize,
    /// Maximum number of supplied wire occurrences.
    pub max_wires: usize,
    /// Maximum total edge-use occurrences across the wires.
    pub max_uses: usize,
}

/// A limit reached during incidence analysis.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EdgeIncidenceResource {
    /// Graph edge count.
    Edges,
    /// Number of supplied wire occurrences.
    Wires,
    /// Total number of edge-use occurrences.
    Uses,
}

/// Failure to produce an incidence report. No partial report is returned.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EdgeIncidenceError {
    /// A configured item limit or representable count was exceeded.
    LimitExceeded(EdgeIncidenceResource),
    /// The wire borrows a different graph object, even if structurally equal.
    DifferentGraph {
        /// Position of the wire occurrence in the input.
        wire: usize,
    },
    /// An internal incidence invariant was violated.
    InvariantViolation,
    /// Memory reservation failed.
    AllocationFailed,
}
impl fmt::Display for EdgeIncidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "edge incidence analysis failed: {self:?}")
    }
}
impl std::error::Error for EdgeIncidenceError {}

/// Classification of use counts in the supplied collection only.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EdgeIncidenceKind {
    /// No supplied wire uses this edge.
    Unused,
    /// Exactly one use, in either direction.
    SingleUse,
    /// Exactly two uses, one in each direction.
    OppositePair,
    /// Exactly two uses in the same direction.
    SameDirectionPair,
    /// More than two uses, regardless of their directions.
    MoreThanTwoUses,
}

/// Exact directed use counts for one graph edge in the supplied collection.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EdgeIncidence {
    forward: usize,
    reverse: usize,
}
impl EdgeIncidence {
    /// Number of forward use occurrences.
    pub const fn forward(self) -> usize {
        self.forward
    }
    /// Number of reverse use occurrences.
    pub const fn reverse(self) -> usize {
        self.reverse
    }
    /// Classifies the stored counts without adding them.
    pub const fn kind(self) -> EdgeIncidenceKind {
        match (self.forward, self.reverse) {
            (0, 0) => EdgeIncidenceKind::Unused,
            (0, 1) | (1, 0) => EdgeIncidenceKind::SingleUse,
            (1, 1) => EdgeIncidenceKind::OppositePair,
            (0, 2) | (2, 0) => EdgeIncidenceKind::SameDirectionPair,
            _ => EdgeIncidenceKind::MoreThanTwoUses,
        }
    }
}

/// A report indexed by graph edge ID, including edges with no supplied uses.
/// The graph is borrowed; the count snapshot does not borrow the wires.
#[derive(Debug)]
pub struct EdgeIncidences<'g> {
    graph: &'g IncidenceGraph,
    edges: Vec<EdgeIncidence>,
}
impl<'g> EdgeIncidences<'g> {
    /// Returns the graph whose edge IDs index this report.
    pub const fn graph(&self) -> &'g IncidenceGraph {
        self.graph
    }
    /// Returns the records in graph edge-ID order.
    pub fn edges(&self) -> &[EdgeIncidence] {
        &self.edges
    }
}

/// Counts directed edge uses across supplied wire occurrences.
///
/// Every wire must borrow the identical graph object; structurally equal graph
/// instances are distinct. Duplicate wire references count as separate
/// occurrences. Classification describes only this collection and establishes
/// neither geometric nor shell validity, connectedness, vertex manifoldness,
/// face interior, or solid orientation. Parallel graph edges remain separate.
///
/// Graph edges and wire occurrences are capped first; graph identity and the
/// checked total number of uses are preflighted before allocation and counting.
/// Storage is reserved fallibly, and no input is mutated or partial result
/// returned on error. Limits bound logical items, not allocator time, process
/// memory availability, or all CPU instructions.
pub fn analyze_edge_incidence<'g>(
    graph: &'g IncidenceGraph,
    wires: &[&SimpleClosedWire<'_>],
    limits: EdgeIncidenceLimits,
) -> Result<EdgeIncidences<'g>, EdgeIncidenceError> {
    if graph.edges.len() > limits.max_edges {
        return Err(EdgeIncidenceError::LimitExceeded(
            EdgeIncidenceResource::Edges,
        ));
    }
    if wires.len() > limits.max_wires {
        return Err(EdgeIncidenceError::LimitExceeded(
            EdgeIncidenceResource::Wires,
        ));
    }
    let mut total = 0usize;
    for (index, wire) in wires.iter().enumerate() {
        if !std::ptr::eq(graph, wire.graph) {
            return Err(EdgeIncidenceError::DifferentGraph { wire: index });
        }
        total = checked_use_total(total, wire.uses.len(), limits.max_uses)?;
    }
    let mut edges = Vec::new();
    edges
        .try_reserve_exact(graph.edges.len())
        .map_err(|_| EdgeIncidenceError::AllocationFailed)?;
    edges.resize(graph.edges.len(), EdgeIncidence::default());
    for wire in wires {
        for edge_use in &wire.uses {
            let count = edges
                .get_mut(edge_use.edge)
                .ok_or(EdgeIncidenceError::InvariantViolation)?;
            let field = match edge_use.direction {
                Direction::Forward => &mut count.forward,
                Direction::Reverse => &mut count.reverse,
            };
            *field = field
                .checked_add(1)
                .ok_or(EdgeIncidenceError::InvariantViolation)?;
        }
    }
    Ok(EdgeIncidences { graph, edges })
}

fn checked_use_total(
    total: usize,
    additional: usize,
    limit: usize,
) -> Result<usize, EdgeIncidenceError> {
    let next = total
        .checked_add(additional)
        .ok_or(EdgeIncidenceError::LimitExceeded(
            EdgeIncidenceResource::Uses,
        ))?;
    if next > limit {
        Err(EdgeIncidenceError::LimitExceeded(
            EdgeIncidenceResource::Uses,
        ))
    } else {
        Ok(next)
    }
}

// The binomial coefficient is preflighted before any quadratic scan.
fn comparison_count(n: usize) -> Option<usize> {
    let previous = n.checked_sub(1)?;
    let (a, b) = if n.is_multiple_of(2) {
        (n / 2, previous)
    } else {
        (n, previous / 2)
    };
    a.checked_mul(b)
}

#[cfg(test)]
mod tests {
    use super::{EdgeIncidenceError, EdgeIncidenceResource, checked_use_total, comparison_count};
    #[test]
    fn total_use_count_checks_overflow() {
        assert_eq!(
            checked_use_total(usize::MAX, 1, usize::MAX),
            Err(EdgeIncidenceError::LimitExceeded(
                EdgeIncidenceResource::Uses
            ))
        );
        assert_eq!(checked_use_total(2, 3, 5), Ok(5));
    }
    #[test]
    fn comparison_count_checks_overflow() {
        assert_eq!(comparison_count(3), Some(3));
        assert_eq!(comparison_count(4), Some(6));
        assert_eq!(comparison_count(usize::MAX), None);
    }
}
