// SPDX-License-Identifier: Apache-2.0
use rcct::topology::{
    Direction, Edge, EdgeIncidenceKind, EdgeIncidenceLimits, GraphLimits, IncidenceGraph,
    ScopedEdgeUse, ScopedSnapshot, ScopedWire, VertexLinkKind, VertexLinkLimits, WireLimits,
    analyze_edge_incidence, analyze_vertex_links,
};

#[test]
fn scoped_wires_feed_both_incidence_diagnostics() {
    let graph = IncidenceGraph::try_new(
        3,
        &[Edge::new(0, 1), Edge::new(1, 2), Edge::new(2, 0)],
        GraphLimits {
            max_vertices: 3,
            max_edges: 3,
        },
    )
    .unwrap();
    let snapshot = ScopedSnapshot::try_new(&graph).unwrap();
    let edge = |id, direction| ScopedEdgeUse::new(snapshot.edge(id).unwrap(), direction);
    let limits = WireLimits {
        max_uses: 3,
        max_vertex_comparisons: 3,
    };
    let forward = ScopedWire::try_new(
        &snapshot,
        &[
            edge(0, Direction::Forward),
            edge(1, Direction::Forward),
            edge(2, Direction::Forward),
        ],
        limits,
    )
    .unwrap();
    let reverse = ScopedWire::try_new(
        &snapshot,
        &[
            edge(2, Direction::Reverse),
            edge(1, Direction::Reverse),
            edge(0, Direction::Reverse),
        ],
        limits,
    )
    .unwrap();
    let wires = [forward.as_wire(), reverse.as_wire()];
    let incidence = analyze_edge_incidence(
        &graph,
        &wires,
        EdgeIncidenceLimits {
            max_edges: 3,
            max_wires: 2,
            max_uses: 6,
        },
    )
    .unwrap();
    assert!(
        incidence
            .edges()
            .iter()
            .all(|edge| edge.kind() == EdgeIncidenceKind::OppositePair)
    );
    let links = analyze_vertex_links(
        &graph,
        &wires,
        VertexLinkLimits {
            max_vertices: 3,
            max_edge_ends: 6,
            max_wires: 2,
            max_uses: 6,
            max_parent_steps: usize::MAX,
        },
    )
    .unwrap();
    assert!(
        links
            .vertices()
            .iter()
            .all(|vertex| vertex.kind() == VertexLinkKind::SingleCycle)
    );
    // These diagnostics do not establish a geometric face or shell.
}
