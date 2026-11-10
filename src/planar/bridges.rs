// SPDX-License-Identifier: Apache-2.0

//! Bounded, iterative connected-component and graph-bridge analysis.

use super::Arrangement2;
use std::fmt;

/// Logical caps for analyzing the undirected arrangement graph.
///
/// Counts bound named work and items, not exact bytes, all instructions,
/// allocator behavior, or wall time.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BridgeAnalysisLimits2 {
    /// Maximum graph vertices.
    pub max_vertices: usize,
    /// Maximum graph edges.
    pub max_edges: usize,
    /// Maximum adjacency entries scanned, exactly twice the edge count.
    pub max_adjacency_visits: usize,
    /// Maximum simultaneous iterative depth-first frames.
    pub max_stack_depth: usize,
    /// Maximum connected components.
    pub max_components: usize,
}

/// A logical bridge-analysis resource whose cap was reached.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BridgeAnalysisResource2 {
    /// Input vertices.
    Vertices,
    /// Input edges.
    Edges,
    /// Adjacency entries scanned.
    AdjacencyVisits,
    /// Simultaneous iterative search frames.
    StackDepth,
    /// Connected components.
    Components,
}

/// Bridge analysis failed without returning a partial result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BridgeAnalysisError2 {
    /// A caller-provided logical cap was reached.
    LimitExceeded(BridgeAnalysisResource2),
    /// A fallible reservation was refused.
    AllocationFailed,
    /// A required graph or traversal invariant did not hold.
    InvariantViolation,
}
impl fmt::Display for BridgeAnalysisError2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LimitExceeded(resource) => {
                write!(f, "bridge analysis limit exceeded: {resource:?}")
            }
            Self::AllocationFailed => f.write_str("bridge analysis allocation failed"),
            Self::InvariantViolation => f.write_str("bridge analysis invariant failed"),
        }
    }
}
impl std::error::Error for BridgeAnalysisError2 {}

/// Connected components and graph bridges tied to one immutable arrangement.
///
/// A bridge is an undirected *merged graph edge* whose removal increases the
/// component count. Multiple coincident source uses do not create parallel
/// graph edges. Results do not assign faces or classify individual source uses.
pub struct BridgeAnalysis2<'a> {
    arrangement: &'a Arrangement2,
    components: Vec<usize>,
    count: usize,
    bridges: Vec<bool>,
}
impl<'a> BridgeAnalysis2<'a> {
    /// Returns the arrangement used by this result.
    pub const fn arrangement(&self) -> &'a Arrangement2 {
        self.arrangement
    }
    /// Returns the number of connected components; the empty graph has zero.
    pub const fn component_count(&self) -> usize {
        self.count
    }
    /// Returns a deterministic component index for a valid vertex.
    pub fn component_of_vertex(&self, vertex: usize) -> Option<usize> {
        self.components.get(vertex).copied()
    }
    /// Returns whether a valid merged graph edge is a bridge.
    pub fn is_bridge(&self, edge: usize) -> Option<bool> {
        self.bridges.get(edge).copied()
    }
}

fn filled<T: Clone>(size: usize, value: T) -> Result<Vec<T>, BridgeAnalysisError2> {
    let mut result = Vec::new();
    result
        .try_reserve_exact(size)
        .map_err(|_| BridgeAnalysisError2::AllocationFailed)?;
    result.resize(size, value);
    Ok(result)
}

#[derive(Clone, Copy)]
struct Frame {
    vertex: usize,
    incoming: Option<usize>,
    next: usize,
}

/// Classifies bridges and connected components with an iterative DFS.
///
/// Every undirected edge contributes exactly two adjacency entries. A private
/// draft is returned only on success; all recoverable failures leave the input
/// graph unchanged. Traversal is iterative, uses no recursive call stack, and
/// fallible reservations report allocation failure. Process aborts and exact
/// allocator byte usage remain outside the recoverable-error contract.
pub fn analyze_bridges2(
    arrangement: &Arrangement2,
    limits: BridgeAnalysisLimits2,
) -> Result<BridgeAnalysis2<'_>, BridgeAnalysisError2> {
    let vertices = arrangement.vertices().len();
    let edges = arrangement.edges().len();
    if vertices > limits.max_vertices {
        return Err(BridgeAnalysisError2::LimitExceeded(
            BridgeAnalysisResource2::Vertices,
        ));
    }
    if edges > limits.max_edges {
        return Err(BridgeAnalysisError2::LimitExceeded(
            BridgeAnalysisResource2::Edges,
        ));
    }
    let adjacency_count = edges
        .checked_mul(2)
        .ok_or(BridgeAnalysisError2::LimitExceeded(
            BridgeAnalysisResource2::AdjacencyVisits,
        ))?;
    if adjacency_count > limits.max_adjacency_visits {
        return Err(BridgeAnalysisError2::LimitExceeded(
            BridgeAnalysisResource2::AdjacencyVisits,
        ));
    }
    let offset_count = vertices
        .checked_add(1)
        .ok_or(BridgeAnalysisError2::LimitExceeded(
            BridgeAnalysisResource2::Vertices,
        ))?;
    let mut offsets = filled(offset_count, 0_usize)?;
    for edge in arrangement.edges() {
        let [a, b] = edge.vertices();
        if a >= vertices || b >= vertices || a >= b {
            return Err(BridgeAnalysisError2::InvariantViolation);
        }
        offsets[a + 1] = offsets[a + 1]
            .checked_add(1)
            .ok_or(BridgeAnalysisError2::InvariantViolation)?;
        offsets[b + 1] = offsets[b + 1]
            .checked_add(1)
            .ok_or(BridgeAnalysisError2::InvariantViolation)?;
    }
    for vertex in 0..vertices {
        offsets[vertex + 1] = offsets[vertex + 1]
            .checked_add(offsets[vertex])
            .ok_or(BridgeAnalysisError2::InvariantViolation)?;
    }
    if offsets[vertices] != adjacency_count {
        return Err(BridgeAnalysisError2::InvariantViolation);
    }
    let mut positions = filled(vertices, 0_usize)?;
    positions.copy_from_slice(&offsets[..vertices]);
    let mut adjacency = filled(adjacency_count, usize::MAX)?;
    for (edge, item) in arrangement.edges().iter().enumerate() {
        for vertex in item.vertices() {
            let position = positions[vertex];
            let slot = adjacency
                .get_mut(position)
                .ok_or(BridgeAnalysisError2::InvariantViolation)?;
            *slot = edge;
            positions[vertex] = position
                .checked_add(1)
                .ok_or(BridgeAnalysisError2::InvariantViolation)?;
        }
    }
    let mut discovered = filled(vertices, usize::MAX)?;
    let mut low = filled(vertices, 0_usize)?;
    let mut components = filled(vertices, usize::MAX)?;
    let mut bridges = filled(edges, false)?;
    let mut frames = Vec::new();
    frames
        .try_reserve_exact(vertices.min(limits.max_stack_depth))
        .map_err(|_| BridgeAnalysisError2::AllocationFailed)?;
    let mut time = 0_usize;
    let mut count = 0_usize;
    for root in 0..vertices {
        if discovered[root] != usize::MAX {
            continue;
        }
        if count >= limits.max_components {
            return Err(BridgeAnalysisError2::LimitExceeded(
                BridgeAnalysisResource2::Components,
            ));
        }
        if limits.max_stack_depth == 0 {
            return Err(BridgeAnalysisError2::LimitExceeded(
                BridgeAnalysisResource2::StackDepth,
            ));
        }
        discovered[root] = time;
        low[root] = time;
        time = time
            .checked_add(1)
            .ok_or(BridgeAnalysisError2::InvariantViolation)?;
        components[root] = count;
        count = count
            .checked_add(1)
            .ok_or(BridgeAnalysisError2::InvariantViolation)?;
        frames.push(Frame {
            vertex: root,
            incoming: None,
            next: offsets[root],
        });
        while let Some(frame) = frames.last_mut() {
            let vertex = frame.vertex;
            if frame.next < offsets[vertex + 1] {
                let edge = adjacency[frame.next];
                frame.next += 1;
                if Some(edge) == frame.incoming {
                    continue;
                }
                let [a, b] = arrangement
                    .edges()
                    .get(edge)
                    .ok_or(BridgeAnalysisError2::InvariantViolation)?
                    .vertices();
                let next = if a == vertex {
                    b
                } else if b == vertex {
                    a
                } else {
                    return Err(BridgeAnalysisError2::InvariantViolation);
                };
                if discovered[next] == usize::MAX {
                    if frames.len() >= limits.max_stack_depth {
                        return Err(BridgeAnalysisError2::LimitExceeded(
                            BridgeAnalysisResource2::StackDepth,
                        ));
                    }
                    discovered[next] = time;
                    low[next] = time;
                    time = time
                        .checked_add(1)
                        .ok_or(BridgeAnalysisError2::InvariantViolation)?;
                    components[next] = components[vertex];
                    frames.push(Frame {
                        vertex: next,
                        incoming: Some(edge),
                        next: offsets[next],
                    });
                } else {
                    low[vertex] = low[vertex].min(discovered[next]);
                }
            } else {
                let completed = frames
                    .pop()
                    .ok_or(BridgeAnalysisError2::InvariantViolation)?;
                if let Some(edge) = completed.incoming {
                    let [a, b] = arrangement
                        .edges()
                        .get(edge)
                        .ok_or(BridgeAnalysisError2::InvariantViolation)?
                        .vertices();
                    let parent = if a == completed.vertex {
                        b
                    } else if b == completed.vertex {
                        a
                    } else {
                        return Err(BridgeAnalysisError2::InvariantViolation);
                    };
                    if frames.last().is_none_or(|frame| frame.vertex != parent) {
                        return Err(BridgeAnalysisError2::InvariantViolation);
                    }
                    if low[completed.vertex] > discovered[parent] {
                        bridges[edge] = true;
                    }
                    low[parent] = low[parent].min(low[completed.vertex]);
                }
            }
        }
    }
    Ok(BridgeAnalysis2 {
        arrangement,
        components,
        count,
        bridges,
    })
}
