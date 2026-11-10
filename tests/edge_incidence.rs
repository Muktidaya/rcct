// SPDX-License-Identifier: Apache-2.0
use rcct::topology::{
    Direction, Edge, EdgeIncidenceError, EdgeIncidenceKind, EdgeIncidenceLimits,
    EdgeIncidenceResource, EdgeUse, GraphLimits, IncidenceGraph, SimpleClosedWire, WireLimits,
    analyze_edge_incidence,
};

fn f(id: usize) -> EdgeUse {
    EdgeUse::new(id, Direction::Forward)
}
fn r(id: usize) -> EdgeUse {
    EdgeUse::new(id, Direction::Reverse)
}
fn graph() -> IncidenceGraph {
    let edges = [
        Edge::new(0, 1),
        Edge::new(0, 2),
        Edge::new(0, 3),
        Edge::new(1, 2),
        Edge::new(1, 3),
        Edge::new(2, 3),
        Edge::new(0, 1),
    ];
    IncidenceGraph::try_new(
        4,
        &edges,
        GraphLimits {
            max_vertices: 4,
            max_edges: 7,
        },
    )
    .unwrap()
}
fn wire<'a>(g: &'a IncidenceGraph, uses: &[EdgeUse]) -> SimpleClosedWire<'a> {
    SimpleClosedWire::try_new(
        g,
        uses,
        WireLimits {
            max_uses: 3,
            max_vertex_comparisons: 3,
        },
    )
    .unwrap()
}
fn limits(wires: usize, uses: usize) -> EdgeIncidenceLimits {
    EdgeIncidenceLimits {
        max_edges: 7,
        max_wires: wires,
        max_uses: uses,
    }
}

#[test]
fn tetrahedron_counts_and_independent_occurrence_oracle() {
    let g = graph();
    let paths = [
        [f(0), f(3), r(1)],
        [f(2), r(4), r(0)],
        [f(1), f(5), r(2)],
        [f(4), r(5), r(3)],
    ];
    let ws: Vec<_> = paths.iter().map(|path| wire(&g, path)).collect();
    let refs: Vec<_> = ws.iter().collect();
    let report = analyze_edge_incidence(&g, &refs, limits(4, 12)).unwrap();
    assert!(std::ptr::eq(report.graph(), &g));
    for edge_id in 0..7 {
        let expected_forward = paths
            .iter()
            .flatten()
            .filter(|edge_use| {
                edge_use.edge() == edge_id && edge_use.direction() == Direction::Forward
            })
            .count();
        let expected_reverse = paths
            .iter()
            .flatten()
            .filter(|edge_use| {
                edge_use.edge() == edge_id && edge_use.direction() == Direction::Reverse
            })
            .count();
        let incidence = report.edges()[edge_id];
        assert_eq!(
            (incidence.forward(), incidence.reverse()),
            (expected_forward, expected_reverse)
        );
        assert_eq!(
            incidence.kind(),
            if edge_id == 6 {
                EdgeIncidenceKind::Unused
            } else {
                EdgeIncidenceKind::OppositePair
            }
        );
    }
}

#[test]
fn classifies_collection_occurrences_not_geometry() {
    let g = graph();
    let a = wire(&g, &[f(0), f(3), r(1)]);
    let reverse_a = wire(&g, &[f(1), r(3), r(0)]);
    let empty = analyze_edge_incidence(&g, &[], limits(0, 0)).unwrap();
    assert!(
        empty
            .edges()
            .iter()
            .all(|e| e.kind() == EdgeIncidenceKind::Unused)
    );
    let single = analyze_edge_incidence(&g, &[&a], limits(1, 3)).unwrap();
    assert_eq!(single.edges()[0].kind(), EdgeIncidenceKind::SingleUse);
    let pair = analyze_edge_incidence(&g, &[&a, &reverse_a], limits(2, 6)).unwrap();
    assert_eq!(pair.edges()[0].kind(), EdgeIncidenceKind::OppositePair);
    let duplicate = analyze_edge_incidence(&g, &[&a, &a], limits(2, 6)).unwrap();
    assert_eq!(
        duplicate.edges()[0].kind(),
        EdgeIncidenceKind::SameDirectionPair
    );
    assert_eq!(
        (
            duplicate.edges()[0].forward(),
            duplicate.edges()[0].reverse()
        ),
        (2, 0)
    );
    assert_eq!(
        (
            duplicate.edges()[1].forward(),
            duplicate.edges()[1].reverse()
        ),
        (0, 2)
    );
    assert_eq!(
        duplicate.edges()[1].kind(),
        EdgeIncidenceKind::SameDirectionPair
    );
    let triple = analyze_edge_incidence(&g, &[&a, &a, &reverse_a], limits(3, 9)).unwrap();
    assert_eq!(triple.edges()[0].kind(), EdgeIncidenceKind::MoreThanTwoUses);
    assert_eq!(
        (triple.edges()[0].forward(), triple.edges()[0].reverse()),
        (2, 1)
    );
}

#[test]
fn caps_graph_identity_and_atomic_failure() {
    let g = graph();
    let other = graph();
    let a = wire(&g, &[f(0), f(3), r(1)]);
    let alien = wire(&other, &[f(0), f(3), r(1)]);
    let input = [&a];
    assert!(matches!(
        analyze_edge_incidence(
            &g,
            &input,
            EdgeIncidenceLimits {
                max_edges: 6,
                ..limits(1, 3)
            }
        ),
        Err(EdgeIncidenceError::LimitExceeded(
            EdgeIncidenceResource::Edges
        ))
    ));
    assert!(matches!(
        analyze_edge_incidence(&g, &input, limits(0, 3)),
        Err(EdgeIncidenceError::LimitExceeded(
            EdgeIncidenceResource::Wires
        ))
    ));
    assert!(matches!(
        analyze_edge_incidence(&g, &input, limits(1, 2)),
        Err(EdgeIncidenceError::LimitExceeded(
            EdgeIncidenceResource::Uses
        ))
    ));
    assert!(matches!(
        analyze_edge_incidence(&g, &[&a, &alien], limits(2, 6)),
        Err(EdgeIncidenceError::DifferentGraph { wire: 1 })
    ));
    assert!(analyze_edge_incidence(&g, &input, limits(1, 3)).is_ok());
    assert_eq!(a.uses(), &[f(0), f(3), r(1)]);
    assert_eq!(g.edges().len(), 7);
}

#[test]
fn empty_graph_and_disconnected_wires_remain_reports() {
    let empty_graph = IncidenceGraph::try_new(
        0,
        &[],
        GraphLimits {
            max_vertices: 0,
            max_edges: 0,
        },
    )
    .unwrap();
    let empty_report = analyze_edge_incidence(
        &empty_graph,
        &[],
        EdgeIncidenceLimits {
            max_edges: 0,
            max_wires: 0,
            max_uses: 0,
        },
    )
    .unwrap();
    assert!(empty_report.edges().is_empty());
    let edges = [
        Edge::new(0, 1),
        Edge::new(1, 2),
        Edge::new(2, 0),
        Edge::new(3, 4),
        Edge::new(4, 5),
        Edge::new(5, 3),
    ];
    let g = IncidenceGraph::try_new(
        6,
        &edges,
        GraphLimits {
            max_vertices: 6,
            max_edges: 6,
        },
    )
    .unwrap();
    let a = wire(&g, &[f(0), f(1), f(2)]);
    let b = wire(&g, &[f(3), f(4), f(5)]);
    let report = analyze_edge_incidence(
        &g,
        &[&a, &b],
        EdgeIncidenceLimits {
            max_edges: 6,
            max_wires: 2,
            max_uses: 6,
        },
    )
    .unwrap();
    assert!(
        report
            .edges()
            .iter()
            .all(|edge| edge.kind() == EdgeIncidenceKind::SingleUse)
    );
}
