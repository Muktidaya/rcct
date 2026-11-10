// SPDX-License-Identifier: Apache-2.0

//! Bounded vertex-link diagnostics for supplied wire occurrences.

use super::{Direction, IncidenceGraph, SimpleClosedWire};
use std::fmt;

/// Logical limits for collecting and analyzing vertex links.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VertexLinkLimits {
    /// Maximum graph vertex count, including isolated vertices.
    pub max_vertices: usize,
    /// Maximum graph edge-end count, twice the graph edge count.
    pub max_edge_ends: usize,
    /// Maximum number of supplied wire occurrences.
    pub max_wires: usize,
    /// Maximum total number of edge-use occurrences.
    pub max_uses: usize,
    /// Maximum union-find parent pointers followed during analysis.
    pub max_parent_steps: usize,
}

/// A configured resource cap for vertex-link analysis.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VertexLinkResource {
    /// Declared graph vertex count.
    Vertices,
    /// Twice the graph edge count or its representable capacity.
    EdgeEnds,
    /// Number of supplied wire occurrences.
    Wires,
    /// Total edge-use occurrences or its representable capacity.
    Uses,
    /// Union-find parent pointers followed.
    ParentSteps,
}

/// A failure to produce a complete vertex-link report.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VertexLinkError {
    /// A configured cap or representable item count was exceeded.
    LimitExceeded(VertexLinkResource),
    /// A wire borrows a different graph object, even if structurally equal.
    DifferentGraph {
        /// Position of the wire occurrence in the supplied slice.
        wire: usize,
    },
    /// An internal graph or accounting invariant was violated.
    InvariantViolation,
    /// A storage reservation failed.
    AllocationFailed,
}
impl fmt::Display for VertexLinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "vertex-link analysis failed: {self:?}")
    }
}
impl std::error::Error for VertexLinkError {}

/// Shape of the active combinatorial link in the supplied collection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VertexLinkKind {
    /// No supplied wire uses an edge end at this vertex.
    Isolated,
    /// One connected link path: two degree-one nodes and all others degree two.
    SinglePath,
    /// One connected link cycle, including a two-node parallel-edge cycle.
    SingleCycle,
    /// A disconnected, branching, or otherwise different active link.
    Other,
}

/// Aggregate link counts for one graph vertex.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VertexLink {
    active_ends: usize,
    corners: usize,
    components: usize,
    degree_one: usize,
    degree_two: usize,
    other_degree: usize,
}
impl VertexLink {
    /// Number of distinct graph-edge ends used at this vertex.
    pub const fn active_ends(self) -> usize {
        self.active_ends
    }
    /// Number of supplied wire-corner occurrences at this vertex.
    pub const fn corners(self) -> usize {
        self.corners
    }
    /// Number of connected components of the active link multigraph.
    pub const fn components(self) -> usize {
        self.components
    }
    /// Number of active link nodes of degree one.
    pub const fn degree_one(self) -> usize {
        self.degree_one
    }
    /// Number of active link nodes of degree two.
    pub const fn degree_two(self) -> usize {
        self.degree_two
    }
    /// Number of active link nodes of degree other than one or two.
    pub const fn other_degree(self) -> usize {
        self.other_degree
    }
    /// Classifies only the active link represented by these counts.
    pub const fn kind(self) -> VertexLinkKind {
        if self.active_ends == 0 {
            VertexLinkKind::Isolated
        } else if self.components == 1 && self.other_degree == 0 && self.degree_one == 2 {
            VertexLinkKind::SinglePath
        } else if self.components == 1 && self.other_degree == 0 && self.degree_one == 0 {
            VertexLinkKind::SingleCycle
        } else {
            VertexLinkKind::Other
        }
    }
}

/// Snapshot of link counts in graph vertex-ID order, including inactive vertices.
#[derive(Debug)]
pub struct VertexLinks<'g> {
    graph: &'g IncidenceGraph,
    vertices: Vec<VertexLink>,
    parent_steps: usize,
}
impl<'g> VertexLinks<'g> {
    /// Returns the graph whose vertex IDs index the report.
    pub const fn graph(&self) -> &'g IncidenceGraph {
        self.graph
    }
    /// Returns one result per graph vertex in ID order.
    pub fn vertices(&self) -> &[VertexLink] {
        &self.vertices
    }
    /// Returns the number of parent pointers followed by this execution.
    /// This implementation count is useful for setting a matching work cap;
    /// it is not promised to remain identical across implementations.
    pub const fn parent_steps(&self) -> usize {
        self.parent_steps
    }
}

fn filled<T: Clone>(count: usize, value: T) -> Result<Vec<T>, VertexLinkError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| VertexLinkError::AllocationFailed)?;
    values.resize(count, value);
    Ok(values)
}

// Every parent-pointer traversal is charged; tests of a root cost no steps.
fn root(
    parent: &[usize],
    mut node: usize,
    steps: &mut usize,
    limit: usize,
) -> Result<usize, VertexLinkError> {
    loop {
        let next = *parent
            .get(node)
            .ok_or(VertexLinkError::InvariantViolation)?;
        if next == node {
            return Ok(node);
        }
        *steps = steps.checked_add(1).ok_or(VertexLinkError::LimitExceeded(
            VertexLinkResource::ParentSteps,
        ))?;
        if *steps > limit {
            return Err(VertexLinkError::LimitExceeded(
                VertexLinkResource::ParentSteps,
            ));
        }
        node = next;
    }
}

/// Analyzes the vertex-link multigraphs induced by supplied wire occurrences.
///
/// Link nodes are graph-edge ends. At each directed wire corner, the preceding
/// use's destination end is connected to the next use's origin end. Each wire
/// occurrence counts separately; duplicate wires produce duplicate link edges.
/// Every wire must borrow the identical graph object. A link includes only ends
/// used by these occurrences, so an omitted graph edge does not make a vertex
/// active. Classifications describe this input alone and do not establish a
/// surface, geometric embedding, manifold, valid shell, or solid.
///
/// Vertex, edge-end, wire and total-use counts are preflighted; graph identity
/// is checked before allocating. Union-by-size uses private fallible storage;
/// every parent pointer followed is charged to `max_parent_steps`. Limits
/// bound logical items and traversals, not allocator behavior, resident bytes,
/// or all CPU instructions. Errors return no partial report or input mutation.
/// Time is O(V + E + U + P), where P is the charged parent-step count.
pub fn analyze_vertex_links<'g>(
    graph: &'g IncidenceGraph,
    wires: &[&SimpleClosedWire<'_>],
    limits: VertexLinkLimits,
) -> Result<VertexLinks<'g>, VertexLinkError> {
    if graph.vertex_count > limits.max_vertices {
        return Err(VertexLinkError::LimitExceeded(VertexLinkResource::Vertices));
    }
    let ends = checked_edge_ends(graph.edges.len(), limits.max_edge_ends)?;
    if wires.len() > limits.max_wires {
        return Err(VertexLinkError::LimitExceeded(VertexLinkResource::Wires));
    }
    let mut total = 0usize;
    for (index, wire) in wires.iter().enumerate() {
        if !std::ptr::eq(graph, wire.graph) {
            return Err(VertexLinkError::DifferentGraph { wire: index });
        }
        total = checked_uses(total, wire.uses.len(), limits.max_uses)?;
    }
    let mut parent = Vec::new();
    parent
        .try_reserve_exact(ends)
        .map_err(|_| VertexLinkError::AllocationFailed)?;
    parent.extend(0..ends);
    let mut size = filled(ends, 1usize)?;
    let mut degrees = filled(ends, 0usize)?;
    let mut vertices = filled(graph.vertex_count, VertexLink::default())?;
    let mut steps = 0usize;
    for wire in wires {
        for (index, outgoing) in wire.uses.iter().enumerate() {
            let previous = wire.uses[if index == 0 {
                wire.uses.len() - 1
            } else {
                index - 1
            }];
            let edge = graph
                .edges
                .get(outgoing.edge)
                .ok_or(VertexLinkError::InvariantViolation)?;
            let before = graph
                .edges
                .get(previous.edge)
                .ok_or(VertexLinkError::InvariantViolation)?;
            let (out_side, vertex) = match outgoing.direction {
                Direction::Forward => (0, edge.start),
                Direction::Reverse => (1, edge.end),
            };
            let (in_side, before_vertex) = match previous.direction {
                Direction::Forward => (1, before.end),
                Direction::Reverse => (0, before.start),
            };
            if vertex != before_vertex {
                return Err(VertexLinkError::InvariantViolation);
            }
            let a = outgoing.edge * 2 + out_side;
            let b = previous.edge * 2 + in_side;
            if a == b {
                return Err(VertexLinkError::InvariantViolation);
            }
            degrees[a] = degrees[a]
                .checked_add(1)
                .ok_or(VertexLinkError::InvariantViolation)?;
            degrees[b] = degrees[b]
                .checked_add(1)
                .ok_or(VertexLinkError::InvariantViolation)?;
            let corner = vertices
                .get_mut(vertex)
                .ok_or(VertexLinkError::InvariantViolation)?;
            corner.corners = corner
                .corners
                .checked_add(1)
                .ok_or(VertexLinkError::InvariantViolation)?;
            let mut x = root(&parent, a, &mut steps, limits.max_parent_steps)?;
            let mut y = root(&parent, b, &mut steps, limits.max_parent_steps)?;
            if x != y {
                if size[x] < size[y] {
                    std::mem::swap(&mut x, &mut y);
                }
                size[x] = size[x]
                    .checked_add(size[y])
                    .ok_or(VertexLinkError::InvariantViolation)?;
                parent[y] = x;
            }
        }
    }
    for (node, &degree) in degrees.iter().enumerate() {
        if degree == 0 {
            continue;
        }
        let edge = graph
            .edges
            .get(node / 2)
            .ok_or(VertexLinkError::InvariantViolation)?;
        let vertex = if node.is_multiple_of(2) {
            edge.start
        } else {
            edge.end
        };
        let entry = vertices
            .get_mut(vertex)
            .ok_or(VertexLinkError::InvariantViolation)?;
        entry.active_ends = entry
            .active_ends
            .checked_add(1)
            .ok_or(VertexLinkError::InvariantViolation)?;
        let bin = match degree {
            1 => &mut entry.degree_one,
            2 => &mut entry.degree_two,
            _ => &mut entry.other_degree,
        };
        *bin = bin
            .checked_add(1)
            .ok_or(VertexLinkError::InvariantViolation)?;
        if root(&parent, node, &mut steps, limits.max_parent_steps)? == node {
            entry.components = entry
                .components
                .checked_add(1)
                .ok_or(VertexLinkError::InvariantViolation)?;
        }
    }
    Ok(VertexLinks {
        graph,
        vertices,
        parent_steps: steps,
    })
}

fn checked_edge_ends(edges: usize, limit: usize) -> Result<usize, VertexLinkError> {
    let ends = edges
        .checked_mul(2)
        .ok_or(VertexLinkError::LimitExceeded(VertexLinkResource::EdgeEnds))?;
    if ends > limit {
        Err(VertexLinkError::LimitExceeded(VertexLinkResource::EdgeEnds))
    } else {
        Ok(ends)
    }
}

fn checked_uses(total: usize, more: usize, limit: usize) -> Result<usize, VertexLinkError> {
    let next = total
        .checked_add(more)
        .ok_or(VertexLinkError::LimitExceeded(VertexLinkResource::Uses))?;
    if next > limit {
        Err(VertexLinkError::LimitExceeded(VertexLinkResource::Uses))
    } else {
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn item_count_overflow_is_rejected() {
        assert_eq!(
            checked_edge_ends(usize::MAX, usize::MAX),
            Err(VertexLinkError::LimitExceeded(VertexLinkResource::EdgeEnds))
        );
        assert_eq!(
            checked_uses(usize::MAX, 1, usize::MAX),
            Err(VertexLinkError::LimitExceeded(VertexLinkResource::Uses))
        );
        assert_eq!(checked_edge_ends(2, 4), Ok(4));
        assert_eq!(checked_uses(2, 3, 5), Ok(5));
    }
}
