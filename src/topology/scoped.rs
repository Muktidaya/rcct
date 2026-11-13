// SPDX-License-Identifier: Apache-2.0

//! Process-local snapshot provenance and declared boundary groups.

use super::{
    Direction, EdgeUse, IncidenceGraph, SimpleClosedWire, WireError, WireLimits, comparison_count,
};
use std::fmt;
use std::sync::Mutex;

// IDs are process-local to this loaded instance and never recycled. Exhaustion
// fails before wrapping; poisoned state is not silently recovered or reset.
static NEXT_SNAPSHOT: Mutex<u64> = Mutex::new(1);

/// Failure to mint a new process-local snapshot identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnapshotError {
    /// No further unique identity can be issued by this loaded instance.
    IdentityExhausted,
    /// The identity counter mutex is poisoned; no recovery is attempted.
    IdentityUnavailable,
}
impl fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "snapshot identity unavailable: {self:?}")
    }
}
impl std::error::Error for SnapshotError {}

fn issue_identity(counter: &Mutex<u64>) -> Result<u64, SnapshotError> {
    let mut next = counter
        .lock()
        .map_err(|_| SnapshotError::IdentityUnavailable)?;
    let id = *next;
    *next = id.checked_add(1).ok_or(SnapshotError::IdentityExhausted)?;
    Ok(id)
}

/// A vertex handle scoped to one process-local snapshot identity.
///
/// Fields cannot be constructed directly in safe code. Handles are ephemeral:
/// they are not serializable identities or credentials across processes, fork,
/// program restarts or separately loaded instances of this crate.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ScopedVertexId {
    snapshot: u64,
    index: usize,
}

/// An edge handle scoped to one process-local snapshot identity.
///
/// This is a distinct type from [`ScopedVertexId`]. Fields cannot be
/// constructed directly in safe code. It has the same process-local limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ScopedEdgeId {
    snapshot: u64,
    index: usize,
}

/// Failure to resolve a handle against a specific snapshot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HandleError {
    /// The handle was minted by a different snapshot.
    DifferentSnapshot,
    /// The index is not present in the snapshot's graph.
    UnknownIndex,
}
impl fmt::Display for HandleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "snapshot handle rejected: {self:?}")
    }
}
impl std::error::Error for HandleError {}

/// A fresh identity over one borrowed immutable incidence graph.
///
/// Every successful construction receives a distinct identity in this loaded
/// crate instance, even for two snapshots borrowing the same graph. No identity
/// is reused, including after a snapshot is dropped. A poisoned counter makes
/// future snapshot creation fail rather than silently recovering. Identities
/// are process-local, not persistent across processes, fork, or restarts.
#[derive(Debug)]
pub struct ScopedSnapshot<'g> {
    graph: &'g IncidenceGraph,
    identity: u64,
}
impl<'g> ScopedSnapshot<'g> {
    /// Mints a fresh snapshot identity without copying or modifying the graph.
    pub fn try_new(graph: &'g IncidenceGraph) -> Result<Self, SnapshotError> {
        Ok(Self {
            graph,
            identity: issue_identity(&NEXT_SNAPSHOT)?,
        })
    }
    /// Returns the immutable underlying graph. Raw indices read from it do not
    /// authenticate their provenance when supplied to another snapshot.
    pub const fn graph(&self) -> &'g IncidenceGraph {
        self.graph
    }
    /// Mints a vertex handle for an index present in this graph.
    pub fn vertex(&self, index: usize) -> Option<ScopedVertexId> {
        (index < self.graph.vertex_count).then_some(ScopedVertexId {
            snapshot: self.identity,
            index,
        })
    }
    /// Mints an edge handle for an index present in this graph.
    pub fn edge(&self, index: usize) -> Option<ScopedEdgeId> {
        (index < self.graph.edges.len()).then_some(ScopedEdgeId {
            snapshot: self.identity,
            index,
        })
    }
    /// Resolves a vertex handle after checking snapshot identity and range.
    pub fn vertex_index(&self, id: ScopedVertexId) -> Result<usize, HandleError> {
        if id.snapshot != self.identity {
            return Err(HandleError::DifferentSnapshot);
        }
        if id.index >= self.graph.vertex_count {
            return Err(HandleError::UnknownIndex);
        }
        Ok(id.index)
    }
    /// Resolves an edge handle after checking snapshot identity and range.
    pub fn edge_index(&self, id: ScopedEdgeId) -> Result<usize, HandleError> {
        if id.snapshot != self.identity {
            return Err(HandleError::DifferentSnapshot);
        }
        if id.index >= self.graph.edges.len() {
            return Err(HandleError::UnknownIndex);
        }
        Ok(id.index)
    }
}

/// An oriented edge handle to be checked when a scoped wire is built.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScopedEdgeUse {
    edge: ScopedEdgeId,
    direction: Direction,
}
impl ScopedEdgeUse {
    /// Creates an occurrence from an existing scoped edge handle.
    pub const fn new(edge: ScopedEdgeId, direction: Direction) -> Self {
        Self { edge, direction }
    }
    /// Returns its scoped edge handle.
    pub const fn edge(self) -> ScopedEdgeId {
        self.edge
    }
    /// Returns its traversal direction.
    pub const fn direction(self) -> Direction {
        self.direction
    }
}

/// Failure to construct a scoped wire. No partial wire is returned.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScopedWireError {
    /// A handle failed provenance or range validation.
    Handle {
        /// Position of the use in the input sequence.
        at: usize,
        /// Why the handle was rejected.
        error: HandleError,
    },
    /// Existing combinatorial wire validation rejected the sequence.
    Wire(WireError),
    /// Temporary use storage could not be reserved.
    AllocationFailed,
}
impl fmt::Display for ScopedWireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "scoped wire rejected: {self:?}")
    }
}
impl std::error::Error for ScopedWireError {}

/// A restricted vertex-simple closed wire tied to one snapshot identity.
#[derive(Debug)]
pub struct ScopedWire<'s, 'g> {
    snapshot: &'s ScopedSnapshot<'g>,
    wire: SimpleClosedWire<'g>,
}
impl<'s, 'g> ScopedWire<'s, 'g> {
    /// Checks limits and all scoped handles, then validates an owned wire.
    ///
    /// The existing [`SimpleClosedWire`] restrictions apply. Input is not
    /// modified, allocation is fallible, and no partial wire is returned.
    /// During construction a temporary use buffer and the validated owned use
    /// buffer can both hold up to `limits.max_uses` entries.
    pub fn try_new(
        snapshot: &'s ScopedSnapshot<'g>,
        uses: &[ScopedEdgeUse],
        limits: WireLimits,
    ) -> Result<Self, ScopedWireError> {
        let n = uses.len();
        if n > limits.max_uses {
            return Err(ScopedWireError::Wire(WireError::UseLimit));
        }
        if n < 3 {
            return Err(ScopedWireError::Wire(WireError::TooFewUses));
        }
        let comparisons =
            comparison_count(n).ok_or(ScopedWireError::Wire(WireError::ComparisonLimit))?;
        if comparisons > limits.max_vertex_comparisons {
            return Err(ScopedWireError::Wire(WireError::ComparisonLimit));
        }
        for (at, edge_use) in uses.iter().enumerate() {
            snapshot
                .edge_index(edge_use.edge)
                .map_err(|error| ScopedWireError::Handle { at, error })?;
        }
        let mut raw = Vec::new();
        raw.try_reserve_exact(n)
            .map_err(|_| ScopedWireError::AllocationFailed)?;
        for edge_use in uses {
            raw.push(EdgeUse::new(edge_use.edge.index, edge_use.direction));
        }
        let wire = SimpleClosedWire::try_new(snapshot.graph, &raw, limits)
            .map_err(ScopedWireError::Wire)?;
        Ok(Self { snapshot, wire })
    }
    /// Returns the snapshot whose identity authenticated the edge uses.
    pub const fn snapshot(&self) -> &'s ScopedSnapshot<'g> {
        self.snapshot
    }
    /// Returns the validated combinatorial wire for read-only diagnostics.
    /// Its raw IDs do not authenticate input to another snapshot.
    pub const fn as_wire(&self) -> &SimpleClosedWire<'g> {
        &self.wire
    }
}

/// Logical caps for a declared boundary group.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BoundaryGroupLimits {
    /// Maximum total boundaries, including the outer declaration.
    pub max_boundaries: usize,
    /// Maximum total uses across boundary occurrences.
    pub max_uses: usize,
    /// Maximum cross-boundary vertex-ID comparisons.
    pub max_vertex_comparisons: usize,
}

/// A resource cap reached while declaring boundaries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundaryGroupResource {
    /// Total boundary count.
    Boundaries,
    /// Total edge uses.
    Uses,
    /// Cross-boundary vertex comparisons.
    VertexComparisons,
}

/// Failure to declare a bounded group of graph-disjoint wire boundaries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundaryGroupError {
    /// An input count, work cap, or representable count was exceeded.
    LimitExceeded(BoundaryGroupResource),
    /// A boundary belongs to a different snapshot.
    DifferentSnapshot {
        /// Input position; outer is zero, then inner order.
        boundary: usize,
    },
    /// Two boundaries share a vertex identity. This includes reused edges.
    SharedVertex {
        /// Earlier boundary position.
        first: usize,
        /// Later boundary position.
        second: usize,
        /// Graph-local vertex index, for diagnostics only.
        vertex: usize,
    },
    /// An internal graph invariant was violated.
    InvariantViolation,
    /// Reference storage could not be reserved.
    AllocationFailed,
}
impl fmt::Display for BoundaryGroupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "boundary group rejected: {self:?}")
    }
}
impl std::error::Error for BoundaryGroupError {}

/// One declared outer and zero or more declared inner graph-disjoint wires.
///
/// The roles express caller intent only. This is not a face, containment,
/// planarity, geometric disjointness, a manifold, a shell, or a solid proof.
#[derive(Debug)]
pub struct DeclaredFaceBoundaries<'w, 's, 'g> {
    boundaries: Vec<&'w ScopedWire<'s, 'g>>,
}
impl<'w, 's, 'g> DeclaredFaceBoundaries<'w, 's, 'g> {
    /// Validates snapshot identity and rejects any shared vertex across loops.
    /// The outer boundary is position zero; inner boundaries retain their order.
    /// Graph-disjointness is a deliberate restricted subset. Limits preflight
    /// item counts and `sum(n_i*n_j)` comparisons over all boundary pairs.
    /// Fallible allocation and private scratch leave inputs unchanged on error.
    pub fn try_new(
        outer: &'w ScopedWire<'s, 'g>,
        inner: &[&'w ScopedWire<'s, 'g>],
        limits: BoundaryGroupLimits,
    ) -> Result<Self, BoundaryGroupError> {
        let count = inner
            .len()
            .checked_add(1)
            .ok_or(BoundaryGroupError::LimitExceeded(
                BoundaryGroupResource::Boundaries,
            ))?;
        if count > limits.max_boundaries {
            return Err(BoundaryGroupError::LimitExceeded(
                BoundaryGroupResource::Boundaries,
            ));
        }
        let mut uses = 0usize;
        let mut comparisons = 0usize;
        for (at, wire) in std::iter::once(outer)
            .chain(inner.iter().copied())
            .enumerate()
        {
            if wire.snapshot.identity != outer.snapshot.identity {
                return Err(BoundaryGroupError::DifferentSnapshot { boundary: at });
            }
            (uses, comparisons) =
                account_boundary(uses, comparisons, wire.wire.uses.len(), limits)?;
        }
        let mut boundaries = Vec::new();
        boundaries
            .try_reserve_exact(count)
            .map_err(|_| BoundaryGroupError::AllocationFailed)?;
        boundaries.push(outer);
        boundaries.extend_from_slice(inner);
        for first in 0..count {
            for second in (first + 1)..count {
                for &a in &boundaries[first].wire.uses {
                    let av = origin(outer.snapshot.graph, a)?;
                    for &b in &boundaries[second].wire.uses {
                        if av == origin(outer.snapshot.graph, b)? {
                            return Err(BoundaryGroupError::SharedVertex {
                                first,
                                second,
                                vertex: av,
                            });
                        }
                    }
                }
            }
        }
        Ok(Self { boundaries })
    }
    /// Returns the designated outer wire.
    pub fn outer(&self) -> &'w ScopedWire<'s, 'g> {
        self.boundaries[0]
    }
    /// Returns designated inner wires in input order.
    pub fn inner(&self) -> &[&'w ScopedWire<'s, 'g>] {
        &self.boundaries[1..]
    }
}

fn account_boundary(
    uses: usize,
    comparisons: usize,
    len: usize,
    limits: BoundaryGroupLimits,
) -> Result<(usize, usize), BoundaryGroupError> {
    let product = uses
        .checked_mul(len)
        .ok_or(BoundaryGroupError::LimitExceeded(
            BoundaryGroupResource::VertexComparisons,
        ))?;
    let next_comparisons =
        comparisons
            .checked_add(product)
            .ok_or(BoundaryGroupError::LimitExceeded(
                BoundaryGroupResource::VertexComparisons,
            ))?;
    if next_comparisons > limits.max_vertex_comparisons {
        return Err(BoundaryGroupError::LimitExceeded(
            BoundaryGroupResource::VertexComparisons,
        ));
    }
    let next_uses = uses
        .checked_add(len)
        .ok_or(BoundaryGroupError::LimitExceeded(
            BoundaryGroupResource::Uses,
        ))?;
    if next_uses > limits.max_uses {
        return Err(BoundaryGroupError::LimitExceeded(
            BoundaryGroupResource::Uses,
        ));
    }
    Ok((next_uses, next_comparisons))
}

fn origin(graph: &IncidenceGraph, edge_use: EdgeUse) -> Result<usize, BoundaryGroupError> {
    let edge = graph
        .edges
        .get(edge_use.edge)
        .ok_or(BoundaryGroupError::InvariantViolation)?;
    Ok(match edge_use.direction {
        Direction::Forward => edge.start,
        Direction::Reverse => edge.end,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn boundary_count_overflow_is_rejected() {
        let limits = BoundaryGroupLimits {
            max_boundaries: usize::MAX,
            max_uses: usize::MAX,
            max_vertex_comparisons: usize::MAX,
        };
        assert_eq!(
            account_boundary(usize::MAX, 0, 2, limits),
            Err(BoundaryGroupError::LimitExceeded(
                BoundaryGroupResource::VertexComparisons
            ))
        );
        assert_eq!(
            account_boundary(usize::MAX, 0, 1, limits),
            Err(BoundaryGroupError::LimitExceeded(
                BoundaryGroupResource::Uses
            ))
        );
        assert_eq!(account_boundary(3, 0, 3, limits), Ok((6, 9)));
    }
    #[test]
    fn matching_but_out_of_range_handles_are_rejected() {
        let graph = IncidenceGraph::try_new(
            1,
            &[],
            super::super::GraphLimits {
                max_vertices: 1,
                max_edges: 0,
            },
        )
        .unwrap();
        let snapshot = ScopedSnapshot::try_new(&graph).unwrap();
        assert_eq!(
            snapshot.vertex_index(ScopedVertexId {
                snapshot: snapshot.identity,
                index: 1
            }),
            Err(HandleError::UnknownIndex)
        );
        assert_eq!(
            snapshot.edge_index(ScopedEdgeId {
                snapshot: snapshot.identity,
                index: 0
            }),
            Err(HandleError::UnknownIndex)
        );
    }
    #[test]
    fn identity_exhaustion_is_permanent() {
        let counter = Mutex::new(u64::MAX - 1);
        assert_eq!(issue_identity(&counter), Ok(u64::MAX - 1));
        assert_eq!(
            issue_identity(&counter),
            Err(SnapshotError::IdentityExhausted)
        );
        assert_eq!(
            issue_identity(&counter),
            Err(SnapshotError::IdentityExhausted)
        );
    }
    #[test]
    fn identity_poison_is_reported_without_recovery() {
        let counter = Mutex::new(1);
        std::thread::scope(|scope| {
            let _ = scope
                .spawn(|| {
                    let _guard = counter.lock().unwrap();
                    panic!("test poison");
                })
                .join();
        });
        assert_eq!(
            issue_identity(&counter),
            Err(SnapshotError::IdentityUnavailable)
        );
    }
}
