// SPDX-License-Identifier: Apache-2.0
use rcct::topology::{
    Direction, Edge, EdgeUse, GraphLimits, IncidenceGraph, SimpleClosedWire, VertexLinkError,
    VertexLinkKind, VertexLinkLimits, VertexLinkResource, WireLimits, analyze_vertex_links,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

fn build(faces: &[Vec<usize>]) -> (IncidenceGraph, Vec<Vec<EdgeUse>>) {
    let mut ids = BTreeMap::new();
    let mut edges = Vec::new();
    let mut paths = Vec::new();
    let max_vertex = faces.iter().flatten().copied().max().unwrap_or(0);
    for face in faces {
        let mut path = Vec::new();
        for i in 0..face.len() {
            let (a, b) = (face[i], face[(i + 1) % face.len()]);
            let key = (a.min(b), a.max(b));
            let id = *ids.entry(key).or_insert_with(|| {
                let id = edges.len();
                // Vary stored edge direction independently of face direction.
                edges.push(if id.is_multiple_of(2) {
                    Edge::new(key.0, key.1)
                } else {
                    Edge::new(key.1, key.0)
                });
                id
            });
            let direction = if edges[id].start() == a {
                Direction::Forward
            } else {
                Direction::Reverse
            };
            path.push(EdgeUse::new(id, direction));
        }
        paths.push(path);
    }
    // An unused graph edge and isolated vertices must remain inactive.
    edges.push(Edge::new(max_vertex + 1, max_vertex + 2));
    let vertex_count = max_vertex + 4;
    let edge_count = edges.len();
    (
        IncidenceGraph::try_new(
            vertex_count,
            &edges,
            GraphLimits {
                max_vertices: vertex_count,
                max_edges: edge_count,
            },
        )
        .unwrap(),
        paths,
    )
}
fn wire<'a>(g: &'a IncidenceGraph, path: &[EdgeUse]) -> SimpleClosedWire<'a> {
    let n = path.len();
    SimpleClosedWire::try_new(
        g,
        path,
        WireLimits {
            max_uses: n,
            max_vertex_comparisons: n * (n - 1) / 2,
        },
    )
    .unwrap()
}
fn limits(g: &IncidenceGraph, paths: &[Vec<EdgeUse>]) -> VertexLinkLimits {
    VertexLinkLimits {
        max_vertices: g.vertex_count(),
        max_edge_ends: g.edges().len() * 2,
        max_wires: paths.len(),
        max_uses: paths.iter().map(Vec::len).sum(),
        max_parent_steps: usize::MAX,
    }
}

// Independent oracle: build explicit per-vertex adjacency lists keyed by edge
// ID, then count connected components with breadth-first traversal.
fn verify_against_adjacency(faces: &[Vec<usize>]) -> Vec<VertexLinkKind> {
    let (g, paths) = build(faces);
    let ws: Vec<_> = paths.iter().map(|p| wire(&g, p)).collect();
    let refs: Vec<_> = ws.iter().collect();
    let result = analyze_vertex_links(&g, &refs, limits(&g, &paths)).unwrap();
    assert!(std::ptr::eq(result.graph(), &g));
    let mut adj: Vec<BTreeMap<usize, Vec<usize>>> = vec![BTreeMap::new(); g.vertex_count()];
    let mut corners = vec![0usize; g.vertex_count()];
    for path in &paths {
        for (i, current) in path.iter().enumerate() {
            let before = path[if i == 0 { path.len() - 1 } else { i - 1 }];
            let edge = g.edges()[current.edge()];
            let vertex = if current.direction() == Direction::Forward {
                edge.start()
            } else {
                edge.end()
            };
            let prior_edge = g.edges()[before.edge()];
            let prior_end = if before.direction() == Direction::Forward {
                prior_edge.end()
            } else {
                prior_edge.start()
            };
            assert_eq!(vertex, prior_end);
            adj[vertex]
                .entry(current.edge())
                .or_default()
                .push(before.edge());
            adj[vertex]
                .entry(before.edge())
                .or_default()
                .push(current.edge());
            corners[vertex] += 1;
        }
    }
    let mut kinds = Vec::new();
    for (vertex, map) in adj.iter().enumerate() {
        let mut seen = BTreeSet::new();
        let mut components = 0;
        for &start in map.keys() {
            if seen.insert(start) {
                components += 1;
                let mut queue = VecDeque::from([start]);
                while let Some(node) = queue.pop_front() {
                    for &other in &map[&node] {
                        if seen.insert(other) {
                            queue.push_back(other);
                        }
                    }
                }
            }
        }
        let one = map
            .values()
            .filter(|neighbors| neighbors.len() == 1)
            .count();
        let two = map
            .values()
            .filter(|neighbors| neighbors.len() == 2)
            .count();
        let other = map.len() - one - two;
        let kind = if map.is_empty() {
            VertexLinkKind::Isolated
        } else if components == 1 && other == 0 && one == 2 {
            VertexLinkKind::SinglePath
        } else if components == 1 && other == 0 && one == 0 {
            VertexLinkKind::SingleCycle
        } else {
            VertexLinkKind::Other
        };
        let got = result.vertices()[vertex];
        assert_eq!(
            (
                got.active_ends(),
                got.corners(),
                got.components(),
                got.degree_one(),
                got.degree_two(),
                got.other_degree(),
                got.kind()
            ),
            (
                map.len(),
                corners[vertex],
                components,
                one,
                two,
                other,
                kind
            )
        );
        kinds.push(kind);
    }
    kinds
}

#[test]
fn paths_parallel_cycles_branches_and_disconnected_links() {
    let triangle = vec![0, 1, 2];
    let single = verify_against_adjacency(std::slice::from_ref(&triangle));
    assert_eq!(single[0], VertexLinkKind::SinglePath);
    let duplicate = verify_against_adjacency(&[triangle.clone(), triangle.clone()]);
    assert_eq!(duplicate[0], VertexLinkKind::SingleCycle); // two parallel link edges
    let triple = verify_against_adjacency(&[triangle.clone(), triangle.clone(), triangle.clone()]);
    assert_eq!(triple[0], VertexLinkKind::Other);
    let disconnected =
        verify_against_adjacency(&[triangle.clone(), triangle, vec![0, 3, 4], vec![0, 3, 4]]);
    assert_eq!(disconnected[0], VertexLinkKind::Other);
}

#[test]
fn tetrahedron_reversal_and_reordering() {
    let mut faces = vec![vec![0, 1, 2], vec![0, 3, 1], vec![0, 2, 3], vec![1, 3, 2]];
    let original = verify_against_adjacency(&faces);
    assert!(
        original[..4]
            .iter()
            .all(|kind| *kind == VertexLinkKind::SingleCycle)
    );
    for face in &mut faces {
        face.reverse();
    }
    faces.reverse();
    assert_eq!(verify_against_adjacency(&faces), original);
}

#[test]
fn exact_caps_foreign_graph_and_failure_atomicity() {
    let faces = vec![vec![0, 1, 2], vec![0, 1, 2]];
    let (g, paths) = build(&faces);
    let (other, other_paths) = build(&faces);
    let w = wire(&g, &paths[0]);
    let alien = wire(&other, &other_paths[0]);
    let cap = limits(&g, &paths);
    let good = analyze_vertex_links(&g, &[&w, &w], cap).unwrap();
    assert!(good.parent_steps() > 0);
    assert!(
        analyze_vertex_links(
            &g,
            &[&w, &w],
            VertexLinkLimits {
                max_parent_steps: good.parent_steps(),
                ..cap
            }
        )
        .is_ok()
    );
    assert!(matches!(
        analyze_vertex_links(
            &g,
            &[&w, &w],
            VertexLinkLimits {
                max_parent_steps: good.parent_steps() - 1,
                ..cap
            }
        ),
        Err(VertexLinkError::LimitExceeded(
            VertexLinkResource::ParentSteps
        ))
    ));
    assert!(matches!(
        analyze_vertex_links(
            &g,
            &[&w, &w],
            VertexLinkLimits {
                max_vertices: cap.max_vertices - 1,
                ..cap
            }
        ),
        Err(VertexLinkError::LimitExceeded(VertexLinkResource::Vertices))
    ));
    assert!(matches!(
        analyze_vertex_links(
            &g,
            &[&w, &w],
            VertexLinkLimits {
                max_edge_ends: cap.max_edge_ends - 1,
                ..cap
            }
        ),
        Err(VertexLinkError::LimitExceeded(VertexLinkResource::EdgeEnds))
    ));
    assert!(matches!(
        analyze_vertex_links(
            &g,
            &[&w, &w],
            VertexLinkLimits {
                max_wires: 0,
                ..cap
            }
        ),
        Err(VertexLinkError::LimitExceeded(VertexLinkResource::Wires))
    ));
    assert!(matches!(
        analyze_vertex_links(
            &g,
            &[&w, &w],
            VertexLinkLimits {
                max_uses: cap.max_uses - 1,
                ..cap
            }
        ),
        Err(VertexLinkError::LimitExceeded(VertexLinkResource::Uses))
    ));
    assert!(matches!(
        analyze_vertex_links(&g, &[&alien], cap),
        Err(VertexLinkError::DifferentGraph { wire: 0 })
    ));
    assert_eq!(w.uses(), paths[0].as_slice());
}

#[test]
fn empty_collection_and_empty_graph_are_inactive() {
    let g = IncidenceGraph::try_new(
        0,
        &[],
        GraphLimits {
            max_vertices: 0,
            max_edges: 0,
        },
    )
    .unwrap();
    let cap = VertexLinkLimits {
        max_vertices: 0,
        max_edge_ends: 0,
        max_wires: 0,
        max_uses: 0,
        max_parent_steps: 0,
    };
    let result = analyze_vertex_links(&g, &[], cap).unwrap();
    assert!(result.vertices().is_empty());
    assert_eq!(result.parent_steps(), 0);
    let (g, _) = build(&[vec![0, 1, 2]]);
    let result = analyze_vertex_links(
        &g,
        &[],
        VertexLinkLimits {
            max_vertices: g.vertex_count(),
            max_edge_ends: 2 * g.edges().len(),
            ..cap
        },
    )
    .unwrap();
    assert!(
        result
            .vertices()
            .iter()
            .all(|v| v.kind() == VertexLinkKind::Isolated)
    );
}

#[test]
fn distinct_parallel_graph_edges_are_distinct_link_nodes() {
    let edges = [
        Edge::new(0, 1),
        Edge::new(1, 2),
        Edge::new(2, 0),
        Edge::new(0, 1),
        Edge::new(1, 3),
        Edge::new(3, 0),
    ];
    let graph = IncidenceGraph::try_new(
        4,
        &edges,
        GraphLimits {
            max_vertices: 4,
            max_edges: 6,
        },
    )
    .unwrap();
    let f = |edge| EdgeUse::new(edge, Direction::Forward);
    let a = wire(&graph, &[f(0), f(1), f(2)]);
    let b = wire(&graph, &[f(3), f(4), f(5)]);
    let report = analyze_vertex_links(
        &graph,
        &[&a, &b],
        VertexLinkLimits {
            max_vertices: 4,
            max_edge_ends: 12,
            max_wires: 2,
            max_uses: 6,
            max_parent_steps: usize::MAX,
        },
    )
    .unwrap();
    let link = report.vertices()[0];
    assert_eq!(
        (
            link.active_ends(),
            link.components(),
            link.degree_one(),
            link.kind()
        ),
        (4, 2, 4, VertexLinkKind::Other)
    );
}
