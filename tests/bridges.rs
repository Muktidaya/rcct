// SPDX-License-Identifier: Apache-2.0
use rcct::planar::{
    Arrangement2, ArrangementLimits2, BridgeAnalysisError2, BridgeAnalysisLimits2,
    BridgeAnalysisResource2, Point2, Segment2, analyze_bridges2, build_arrangement2,
};
use std::collections::VecDeque;

fn segment(a: (i64, i64), b: (i64, i64)) -> Segment2 {
    Segment2::try_new(
        Point2::try_new(a.0 as f64, a.1 as f64).unwrap(),
        Point2::try_new(b.0 as f64, b.1 as f64).unwrap(),
    )
    .unwrap()
}
fn graph(sources: &[Segment2]) -> Arrangement2 {
    build_arrangement2(
        sources,
        ArrangementLimits2 {
            max_segments: 32,
            max_pairs: 496,
            max_cuts: 2_100,
            max_vertices: 600,
            max_edges: 1_000,
            max_edge_uses: 2_100,
            max_comparisons: 10_000_000,
        },
    )
    .unwrap()
}
fn limits() -> BridgeAnalysisLimits2 {
    BridgeAnalysisLimits2 {
        max_vertices: 600,
        max_edges: 1_000,
        max_adjacency_visits: 2_000,
        max_stack_depth: 600,
        max_components: 600,
    }
}

// Independent reference: remove one edge, then count connected components with
// breadth-first search. This uses no low-link or DFS implementation.
fn oracle(graph: &Arrangement2, removed: Option<usize>) -> (usize, Vec<usize>) {
    let n = graph.vertices().len();
    let mut components = vec![usize::MAX; n];
    let mut count = 0;
    for root in 0..n {
        if components[root] != usize::MAX {
            continue;
        }
        components[root] = count;
        let mut queue = VecDeque::from([root]);
        while let Some(v) = queue.pop_front() {
            for (index, edge) in graph.edges().iter().enumerate() {
                if Some(index) == removed {
                    continue;
                }
                let [a, b] = edge.vertices();
                let neighbor = if a == v {
                    Some(b)
                } else if b == v {
                    Some(a)
                } else {
                    None
                };
                if let Some(next) = neighbor
                    && components[next] == usize::MAX
                {
                    components[next] = count;
                    queue.push_back(next);
                }
            }
        }
        count += 1;
    }
    (count, components)
}
fn verify(graph: &Arrangement2) {
    let result = analyze_bridges2(graph, limits()).unwrap();
    let (count, components) = oracle(graph, None);
    assert_eq!(result.component_count(), count);
    for a in 0..components.len() {
        for b in 0..components.len() {
            assert_eq!(
                result.component_of_vertex(a) == result.component_of_vertex(b),
                components[a] == components[b]
            );
        }
    }
    for edge in 0..graph.edges().len() {
        let (removed_count, _) = oracle(graph, Some(edge));
        assert_eq!(
            result.is_bridge(edge),
            Some(removed_count > count),
            "edge {edge}"
        );
    }
    assert_eq!(result.component_of_vertex(graph.vertices().len()), None);
    assert_eq!(result.is_bridge(graph.edges().len()), None);
}
fn square(x: i64, y: i64) -> Vec<Segment2> {
    vec![
        segment((x, y), (x + 2, y)),
        segment((x + 2, y), (x + 2, y + 2)),
        segment((x + 2, y + 2), (x, y + 2)),
        segment((x, y + 2), (x, y)),
    ]
}

#[test]
fn deterministic_random_graphs_match_edge_removal_oracle() {
    let mut state = 0x5d7a_298b_u64;
    let mut rand = |n: usize| {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((state >> 32) as usize) % n
    };
    for _ in 0..160 {
        let count = 1 + rand(6);
        let mut sources = Vec::new();
        for i in 0..count {
            if i > 0 && rand(7) == 0 {
                let old: Segment2 = sources[rand(i)];
                sources.push(Segment2::try_new(old.end(), old.start()).unwrap());
                continue;
            }
            let a = (rand(9) as i64 - 4, rand(9) as i64 - 4);
            let mut b = (rand(9) as i64 - 4, rand(9) as i64 - 4);
            if a == b {
                b.0 += 1;
            }
            sources.push(segment(a, b));
        }
        verify(&graph(&sources));
    }
}

#[test]
fn cycles_spurs_crossings_and_coincident_sources() {
    let mut s = square(0, 0);
    s.push(segment((2, 0), (4, 0)));
    let g = graph(&s);
    let a = analyze_bridges2(&g, limits()).unwrap();
    assert_eq!(a.component_count(), 1);
    assert_eq!(
        (0..g.edges().len())
            .filter(|&e| a.is_bridge(e) == Some(true))
            .count(),
        1
    );
    verify(&g);
    let cross = graph(&[segment((-2, 0), (2, 0)), segment((0, -2), (0, 2))]);
    let a = analyze_bridges2(&cross, limits()).unwrap();
    assert!((0..cross.edges().len()).all(|e| a.is_bridge(e) == Some(true)));
    verify(&cross);
    let duplicate = graph(&[segment((0, 0), (2, 0)), segment((2, 0), (0, 0))]);
    let a = analyze_bridges2(&duplicate, limits()).unwrap();
    assert_eq!(
        (duplicate.edges().len(), duplicate.edge_uses().len()),
        (1, 2)
    );
    assert_eq!(a.is_bridge(0), Some(true));
    verify(&duplicate);
    let mut disconnected = square(0, 0);
    disconnected.extend(square(5, 0));
    let g = graph(&disconnected);
    let a = analyze_bridges2(&g, limits()).unwrap();
    assert_eq!(a.component_count(), 2);
    assert!((0..g.edges().len()).all(|e| a.is_bridge(e) == Some(false)));
    verify(&g);
}

#[test]
fn caps_and_iterative_depth_fail_without_mutating_graph() {
    let mut sources = Vec::new();
    for x in 0..20 {
        sources.push(segment((x, 0), (x + 1, 0)));
    }
    let graph = graph(&sources);
    let baseline = analyze_bridges2(&graph, limits()).unwrap();
    assert_eq!(
        (
            graph.vertices().len(),
            graph.edges().len(),
            baseline.component_count()
        ),
        (21, 20, 1)
    );
    for (resource, lim) in [
        (
            BridgeAnalysisResource2::Vertices,
            BridgeAnalysisLimits2 {
                max_vertices: 20,
                ..limits()
            },
        ),
        (
            BridgeAnalysisResource2::Edges,
            BridgeAnalysisLimits2 {
                max_edges: 19,
                ..limits()
            },
        ),
        (
            BridgeAnalysisResource2::AdjacencyVisits,
            BridgeAnalysisLimits2 {
                max_adjacency_visits: 39,
                ..limits()
            },
        ),
        (
            BridgeAnalysisResource2::StackDepth,
            BridgeAnalysisLimits2 {
                max_stack_depth: 10,
                ..limits()
            },
        ),
        (
            BridgeAnalysisResource2::Components,
            BridgeAnalysisLimits2 {
                max_components: 0,
                ..limits()
            },
        ),
    ] {
        assert!(
            matches!(analyze_bridges2(&graph,lim),Err(BridgeAnalysisError2::LimitExceeded(actual)) if actual==resource)
        );
        assert_eq!(baseline.component_count(), 1);
        assert!((0..20).all(|e| baseline.is_bridge(e) == Some(true)));
    }
    let exact = BridgeAnalysisLimits2 {
        max_vertices: 21,
        max_edges: 20,
        max_adjacency_visits: 40,
        max_stack_depth: 21,
        max_components: 1,
    };
    assert!(analyze_bridges2(&graph, exact).is_ok());
    let empty = build_arrangement2(
        &[],
        ArrangementLimits2 {
            max_segments: 0,
            max_pairs: 0,
            max_cuts: 0,
            max_vertices: 0,
            max_edges: 0,
            max_edge_uses: 0,
            max_comparisons: 0,
        },
    )
    .unwrap();
    let zero = BridgeAnalysisLimits2 {
        max_vertices: 0,
        max_edges: 0,
        max_adjacency_visits: 0,
        max_stack_depth: 0,
        max_components: 0,
    };
    assert_eq!(analyze_bridges2(&empty, zero).unwrap().component_count(), 0);
}
