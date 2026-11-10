// SPDX-License-Identifier: Apache-2.0
use rcct::topology::{
    Direction, Edge, EdgeUse, GraphError, GraphLimits, IncidenceGraph, SimpleClosedWire, WireError,
    WireLimits,
};

fn graph(vertices: usize, edges: &[Edge]) -> IncidenceGraph {
    IncidenceGraph::try_new(
        vertices,
        edges,
        GraphLimits {
            max_vertices: vertices,
            max_edges: edges.len(),
        },
    )
    .unwrap()
}
fn limits(n: usize) -> WireLimits {
    WireLimits {
        max_uses: n,
        max_vertex_comparisons: n * (n - 1) / 2,
    }
}
fn forward(i: usize) -> EdgeUse {
    EdgeUse::new(i, Direction::Forward)
}
fn reverse(i: usize) -> EdgeUse {
    EdgeUse::new(i, Direction::Reverse)
}

#[test]
fn graph_checks_endpoints_limits_and_preserves_parallel_edges() {
    let edges = [Edge::new(0, 1), Edge::new(0, 1)];
    let g = graph(3, &edges);
    assert_eq!(g.vertex_count(), 3);
    assert_eq!(g.edges(), &edges);
    assert!(matches!(
        IncidenceGraph::try_new(
            3,
            &edges,
            GraphLimits {
                max_vertices: 2,
                max_edges: 2
            }
        ),
        Err(GraphError::VertexLimit)
    ));
    assert!(matches!(
        IncidenceGraph::try_new(
            3,
            &edges,
            GraphLimits {
                max_vertices: 3,
                max_edges: 1
            }
        ),
        Err(GraphError::EdgeLimit)
    ));
    assert!(matches!(
        IncidenceGraph::try_new(
            2,
            &[Edge::new(0, 2)],
            GraphLimits {
                max_vertices: 2,
                max_edges: 1
            }
        ),
        Err(GraphError::UnknownVertex { edge: 0, vertex: 2 })
    ));
    assert!(matches!(
        IncidenceGraph::try_new(
            2,
            &[Edge::new(1, 1)],
            GraphLimits {
                max_vertices: 2,
                max_edges: 1
            }
        ),
        Err(GraphError::LoopEdge { edge: 0 })
    ));
}

#[test]
fn directed_cycles_rotate_reverse_and_preserve_input() {
    let g = graph(4, &[Edge::new(0, 1), Edge::new(2, 1), Edge::new(2, 0)]);
    let mut uses = [forward(0), reverse(1), forward(2)];
    for _ in 0..3 {
        let saved = uses;
        let wire = SimpleClosedWire::try_new(&g, &uses, limits(3)).unwrap();
        assert!(std::ptr::eq(wire.graph(), &g));
        assert_eq!(wire.uses(), &saved);
        uses.rotate_left(1);
    }
    let reversed = [reverse(2), forward(1), reverse(0)];
    assert!(SimpleClosedWire::try_new(&g, &reversed, limits(3)).is_ok());
}

#[test]
fn invalid_cycles_and_exact_resource_limits_fail_without_mutation() {
    let edges = [
        Edge::new(0, 1),
        Edge::new(1, 2),
        Edge::new(2, 0),
        Edge::new(1, 3),
        Edge::new(3, 1),
    ];
    let g = graph(4, &edges);
    let good = [forward(0), forward(1), forward(2)];
    assert!(SimpleClosedWire::try_new(&g, &good, limits(3)).is_ok());
    assert!(matches!(
        SimpleClosedWire::try_new(
            &g,
            &good,
            WireLimits {
                max_uses: 2,
                max_vertex_comparisons: 3
            }
        ),
        Err(WireError::UseLimit)
    ));
    assert!(matches!(
        SimpleClosedWire::try_new(
            &g,
            &good,
            WireLimits {
                max_uses: 3,
                max_vertex_comparisons: 2
            }
        ),
        Err(WireError::ComparisonLimit)
    ));
    assert!(matches!(
        SimpleClosedWire::try_new(&g, &good[..2], limits(3)),
        Err(WireError::TooFewUses)
    ));
    assert!(matches!(
        SimpleClosedWire::try_new(&g, &[forward(0), forward(1), forward(99)], limits(3)),
        Err(WireError::UnknownEdge { at: 2, edge: 99 })
    ));
    assert!(matches!(
        SimpleClosedWire::try_new(&g, &[forward(0), forward(1), reverse(2)], limits(3)),
        Err(WireError::Disconnected { .. })
    ));
    let repeated = [forward(0), forward(3), forward(4), forward(1), forward(2)];
    assert!(matches!(
        SimpleClosedWire::try_new(&g, &repeated, limits(5)),
        Err(WireError::RepeatedVertex {
            first: 1,
            second: 3
        })
    ));
    assert_eq!(g.edges(), &edges);
    assert_eq!(good, [forward(0), forward(1), forward(2)]);
}

#[test]
fn exhaustive_triangle_oracle_from_independent_endpoint_lists() {
    let pairs = [(0, 1), (1, 2), (2, 0), (0, 2), (2, 1), (1, 0)];
    let edges: Vec<_> = pairs.iter().map(|&(a, b)| Edge::new(a, b)).collect();
    let g = graph(3, &edges);
    for a in 0usize..12 {
        for b in 0usize..12 {
            for c in 0usize..12 {
                let ids = [a, b, c];
                let uses = ids.map(|id| {
                    if id.is_multiple_of(2) {
                        forward(id / 2)
                    } else {
                        reverse(id / 2)
                    }
                });
                let endpoints = ids.map(|id| {
                    let (x, y) = pairs[id / 2];
                    if id.is_multiple_of(2) { (x, y) } else { (y, x) }
                });
                let expected = endpoints[0].1 == endpoints[1].0
                    && endpoints[1].1 == endpoints[2].0
                    && endpoints[2].1 == endpoints[0].0
                    && endpoints[0].0 != endpoints[1].0
                    && endpoints[1].0 != endpoints[2].0
                    && endpoints[0].0 != endpoints[2].0;
                assert_eq!(
                    SimpleClosedWire::try_new(&g, &uses, limits(3)).is_ok(),
                    expected,
                    "{ids:?}"
                );
            }
        }
    }
}

#[test]
fn wraparound_duplicate_edge_and_owned_input() {
    let g = graph(
        4,
        &[
            Edge::new(0, 1),
            Edge::new(1, 2),
            Edge::new(2, 3),
            Edge::new(2, 0),
        ],
    );
    assert!(matches!(
        SimpleClosedWire::try_new(&g, &[forward(0), forward(1), forward(2)], limits(3)),
        Err(WireError::Disconnected { at: 2 })
    ));
    assert!(matches!(
        SimpleClosedWire::try_new(
            &g,
            &[forward(0), reverse(0), forward(0), forward(1), forward(3)],
            limits(5)
        ),
        Err(WireError::RepeatedVertex {
            first: 0,
            second: 2
        })
    ));
    let mut input = [forward(0), forward(1), forward(3)];
    let wire = SimpleClosedWire::try_new(&g, &input, limits(3)).unwrap();
    input[0] = reverse(0);
    assert_eq!(wire.uses(), &[forward(0), forward(1), forward(3)]);
    assert_ne!(input[0], wire.uses()[0]);
    assert_eq!(graph(0, &[]).vertex_count(), 0);
    assert!(matches!(
        IncidenceGraph::try_new(
            2,
            &[Edge::new(2, 1)],
            GraphLimits {
                max_vertices: 2,
                max_edges: 1
            }
        ),
        Err(GraphError::UnknownVertex { edge: 0, vertex: 2 })
    ));
}
