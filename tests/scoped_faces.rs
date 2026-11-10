// SPDX-License-Identifier: Apache-2.0
use rcct::topology::{
    BoundaryGroupError, BoundaryGroupLimits, BoundaryGroupResource, DeclaredFaceBoundaries,
    Direction, Edge, GraphLimits, HandleError, IncidenceGraph, ScopedEdgeUse, ScopedSnapshot,
    ScopedWire, ScopedWireError, WireError, WireLimits,
};

fn graph() -> IncidenceGraph {
    let edges = [
        Edge::new(0, 1),
        Edge::new(1, 2),
        Edge::new(2, 0),
        Edge::new(3, 4),
        Edge::new(4, 5),
        Edge::new(5, 3),
        Edge::new(0, 6),
        Edge::new(6, 7),
        Edge::new(7, 0),
        Edge::new(8, 9),
        Edge::new(9, 10),
        Edge::new(10, 8),
    ];
    IncidenceGraph::try_new(
        12,
        &edges,
        GraphLimits {
            max_vertices: 12,
            max_edges: 12,
        },
    )
    .unwrap()
}
fn uses(snapshot: &ScopedSnapshot<'_>, ids: [usize; 3]) -> [ScopedEdgeUse; 3] {
    ids.map(|id| ScopedEdgeUse::new(snapshot.edge(id).unwrap(), Direction::Forward))
}
fn wire<'s, 'g>(snapshot: &'s ScopedSnapshot<'g>, ids: [usize; 3]) -> ScopedWire<'s, 'g> {
    ScopedWire::try_new(
        snapshot,
        &uses(snapshot, ids),
        WireLimits {
            max_uses: 3,
            max_vertex_comparisons: 3,
        },
    )
    .unwrap()
}
fn limits(boundaries: usize, uses: usize, comparisons: usize) -> BoundaryGroupLimits {
    BoundaryGroupLimits {
        max_boundaries: boundaries,
        max_uses: uses,
        max_vertex_comparisons: comparisons,
    }
}

#[test]
fn snapshots_distinguish_same_graph_foreign_graph_and_stale_handles() {
    let g = graph();
    let h = graph();
    let second = ScopedSnapshot::try_new(&g).unwrap();
    let foreign = ScopedSnapshot::try_new(&h).unwrap();
    let edge = {
        let first = ScopedSnapshot::try_new(&g).unwrap();
        let edge = first.edge(0).unwrap();
        let vertex = first.vertex(0).unwrap();
        assert_eq!(first.edge_index(edge), Ok(0));
        assert_eq!(first.vertex_index(vertex), Ok(0));
        assert_eq!(second.edge_index(edge), Err(HandleError::DifferentSnapshot));
        assert_eq!(
            second.vertex_index(vertex),
            Err(HandleError::DifferentSnapshot)
        );
        assert_eq!(
            foreign.edge_index(edge),
            Err(HandleError::DifferentSnapshot)
        );
        assert!(first.edge(12).is_none());
        assert!(first.vertex(12).is_none());
        edge
    };
    let replacement = ScopedSnapshot::try_new(&g).unwrap();
    assert_eq!(
        replacement.edge_index(edge),
        Err(HandleError::DifferentSnapshot)
    );
    assert_ne!(replacement.edge(0).unwrap(), edge);
}

#[test]
fn scoped_wires_enforce_all_provenance_and_resource_limits() {
    let g = graph();
    let a = ScopedSnapshot::try_new(&g).unwrap();
    let b = ScopedSnapshot::try_new(&g).unwrap();
    let original = uses(&a, [0, 1, 2]);
    let result = ScopedWire::try_new(
        &a,
        &original,
        WireLimits {
            max_uses: 3,
            max_vertex_comparisons: 3,
        },
    )
    .unwrap();
    assert_eq!(result.as_wire().uses().len(), 3);
    let mut mixed = original;
    mixed[2] = ScopedEdgeUse::new(b.edge(2).unwrap(), Direction::Forward);
    assert!(matches!(
        ScopedWire::try_new(
            &a,
            &mixed,
            WireLimits {
                max_uses: 3,
                max_vertex_comparisons: 3
            }
        ),
        Err(ScopedWireError::Handle {
            at: 2,
            error: HandleError::DifferentSnapshot
        })
    ));
    assert!(matches!(
        ScopedWire::try_new(
            &a,
            &original,
            WireLimits {
                max_uses: 2,
                max_vertex_comparisons: 3
            }
        ),
        Err(ScopedWireError::Wire(WireError::UseLimit))
    ));
    assert!(matches!(
        ScopedWire::try_new(
            &a,
            &original,
            WireLimits {
                max_uses: 3,
                max_vertex_comparisons: 2
            }
        ),
        Err(ScopedWireError::Wire(WireError::ComparisonLimit))
    ));
    assert!(matches!(
        ScopedWire::try_new(
            &a,
            &original[..2],
            WireLimits {
                max_uses: 3,
                max_vertex_comparisons: 3
            }
        ),
        Err(ScopedWireError::Wire(WireError::TooFewUses))
    ));
    assert_eq!(original, uses(&a, [0, 1, 2]));
}

#[test]
fn boundary_declarations_are_bounded_and_graph_disjoint() {
    let g = graph();
    let snapshot = ScopedSnapshot::try_new(&g).unwrap();
    let foreign = ScopedSnapshot::try_new(&g).unwrap();
    let outer = wire(&snapshot, [0, 1, 2]);
    let inner = wire(&snapshot, [3, 4, 5]);
    let second_inner = wire(&snapshot, [9, 10, 11]);
    let shared = wire(&snapshot, [6, 7, 8]);
    let reversed_uses = [
        ScopedEdgeUse::new(snapshot.edge(2).unwrap(), Direction::Reverse),
        ScopedEdgeUse::new(snapshot.edge(1).unwrap(), Direction::Reverse),
        ScopedEdgeUse::new(snapshot.edge(0).unwrap(), Direction::Reverse),
    ];
    let reversed = ScopedWire::try_new(
        &snapshot,
        &reversed_uses,
        WireLimits {
            max_uses: 3,
            max_vertex_comparisons: 3,
        },
    )
    .unwrap();
    let alien = wire(&foreign, [3, 4, 5]);
    let group = DeclaredFaceBoundaries::try_new(&outer, &[&inner, &second_inner], limits(3, 9, 27))
        .unwrap();
    assert!(std::ptr::eq(group.outer(), &outer));
    assert_eq!(group.inner().len(), 2);
    assert!(std::ptr::eq(group.inner()[0], &inner));
    assert!(DeclaredFaceBoundaries::try_new(&outer, &[], limits(1, 3, 0)).is_ok());
    assert!(matches!(
        DeclaredFaceBoundaries::try_new(&outer, &[&inner, &second_inner], limits(2, 9, 27)),
        Err(BoundaryGroupError::LimitExceeded(
            BoundaryGroupResource::Boundaries
        ))
    ));
    assert!(matches!(
        DeclaredFaceBoundaries::try_new(&outer, &[&inner, &second_inner], limits(3, 8, 27)),
        Err(BoundaryGroupError::LimitExceeded(
            BoundaryGroupResource::Uses
        ))
    ));
    assert!(matches!(
        DeclaredFaceBoundaries::try_new(&outer, &[&inner, &second_inner], limits(3, 9, 26)),
        Err(BoundaryGroupError::LimitExceeded(
            BoundaryGroupResource::VertexComparisons
        ))
    ));
    assert!(matches!(
        DeclaredFaceBoundaries::try_new(&outer, &[&alien], limits(2, 6, 9)),
        Err(BoundaryGroupError::DifferentSnapshot { boundary: 1 })
    ));
    assert!(matches!(
        DeclaredFaceBoundaries::try_new(&outer, &[&shared], limits(2, 6, 9)),
        Err(BoundaryGroupError::SharedVertex {
            first: 0,
            second: 1,
            vertex: 0
        })
    ));
    assert!(matches!(
        DeclaredFaceBoundaries::try_new(&outer, &[&outer], limits(2, 6, 9)),
        Err(BoundaryGroupError::SharedVertex {
            first: 0,
            second: 1,
            vertex: 0
        })
    ));
    assert!(matches!(
        DeclaredFaceBoundaries::try_new(&outer, &[&reversed], limits(2, 6, 9)),
        Err(BoundaryGroupError::SharedVertex {
            first: 0,
            second: 1,
            ..
        })
    ));
    assert_eq!(outer.as_wire().uses().len(), 3);
}

#[test]
fn concurrent_snapshot_creation_mints_distinct_handles() {
    let g = graph();
    let handles = std::thread::scope(|scope| {
        let a = scope.spawn(|| ScopedSnapshot::try_new(&g).unwrap().edge(0).unwrap());
        let b = scope.spawn(|| ScopedSnapshot::try_new(&g).unwrap().edge(0).unwrap());
        [a.join().unwrap(), b.join().unwrap()]
    });
    assert_ne!(handles[0], handles[1]);
    let next = ScopedSnapshot::try_new(&g).unwrap();
    assert_eq!(
        next.edge_index(handles[0]),
        Err(HandleError::DifferentSnapshot)
    );
}
