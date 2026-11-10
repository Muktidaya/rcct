// SPDX-License-Identifier: Apache-2.0
use rcct::planar::{
    Arrangement2, ArrangementLimits2, EmbeddingError2, EmbeddingLimits2, EmbeddingResource2,
    Orientation2, Point2, Segment2, WalkOrientationError2, WalkOrientationLimits2,
    WalkOrientationResource2, build_arrangement2, embed_arrangement2, simple_walk_orientation2,
};
use std::cmp::Ordering;
use std::collections::BTreeSet;

fn raw(a: (f64, f64), b: (f64, f64)) -> Segment2 {
    Segment2::try_new(
        Point2::try_new(a.0, a.1).unwrap(),
        Point2::try_new(b.0, b.1).unwrap(),
    )
    .unwrap()
}
fn segment(a: (i64, i64), b: (i64, i64)) -> Segment2 {
    raw((a.0 as f64, a.1 as f64), (b.0 as f64, b.1 as f64))
}
fn graph(inputs: &[Segment2]) -> Arrangement2 {
    build_arrangement2(
        inputs,
        ArrangementLimits2 {
            max_segments: 12,
            max_pairs: 66,
            max_cuts: 300,
            max_vertices: 150,
            max_edges: 300,
            max_edge_uses: 300,
            max_comparisons: 500_000,
        },
    )
    .unwrap()
}
fn limits() -> EmbeddingLimits2 {
    EmbeddingLimits2 {
        max_vertices: 150,
        max_half_edges: 600,
        max_edge_uses: 300,
        max_comparisons: 500_000,
        max_boundary_walks: 600,
        max_walk_steps: 600,
    }
}
fn square(x: i64, y: i64, size: i64) -> Vec<Segment2> {
    vec![
        segment((x, y), (x + size, y)),
        segment((x + size, y), (x + size, y + size)),
        segment((x + size, y + size), (x, y + size)),
        segment((x, y + size), (x, y)),
    ]
}

// Independent small-integer oracle; no RCCT orientation or intersection code
// determines the expected radial ordering or successor links.
fn integer_direction(g: &Arrangement2, h: usize) -> (i128, i128) {
    let edge = h / 2;
    let use_ = g.edge_uses().iter().find(|u| u.edge() == edge).unwrap();
    let source = g.sources()[use_.source()];
    let mut dx = (source.end().x() as i64 - source.start().x() as i64) as i128;
    let mut dy = (source.end().y() as i64 - source.start().y() as i64) as i128;
    let along = use_.forward() == h.is_multiple_of(2);
    if !along {
        dx = -dx;
        dy = -dy;
    }
    (dx, dy)
}
fn compare_direction(a: (i128, i128), b: (i128, i128)) -> Ordering {
    let upper = |(x, y): (i128, i128)| y > 0 || (y == 0 && x > 0);
    match (upper(a), upper(b)) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        _ => match (a.0 * b.1 - a.1 * b.0).cmp(&0) {
            Ordering::Greater => Ordering::Less,
            Ordering::Less => Ordering::Greater,
            Ordering::Equal => Ordering::Equal,
        },
    }
}
fn check_embedding(g: &Arrangement2) {
    let embedding = embed_arrangement2(g, limits()).unwrap();
    let half = embedding.half_edges();
    assert_eq!(half.len(), g.edges().len() * 2);
    for vertex in 0..g.vertices().len() {
        let mut expected: Vec<_> = half
            .iter()
            .enumerate()
            .filter_map(|(i, h)| (h.origin() == vertex).then_some(i))
            .collect();
        expected
            .sort_by(|&a, &b| compare_direction(integer_direction(g, a), integer_direction(g, b)));
        for pair in expected.windows(2) {
            assert_ne!(
                compare_direction(integer_direction(g, pair[0]), integer_direction(g, pair[1])),
                Ordering::Equal
            );
        }
        assert_eq!(embedding.outgoing(vertex).unwrap(), expected);
        for (i, &outgoing) in expected.iter().enumerate() {
            let incoming = half[outgoing].twin();
            let clockwise = expected[(i + expected.len() - 1) % expected.len()];
            assert_eq!(half[incoming].next(), clockwise);
        }
    }
    assert!(embedding.outgoing(g.vertices().len()).is_none());
    for (i, h) in half.iter().enumerate() {
        assert_eq!(half[h.twin()].twin(), i);
        assert_eq!(half[h.twin()].origin(), h.destination());
        assert_eq!(half[h.next()].origin(), h.destination());
        assert_eq!(half[h.twin()].edge(), h.edge());
    }
    let mut seen = BTreeSet::new();
    for index in 0..embedding.boundary_walk_count() {
        let walk = embedding.boundary_walk(index).unwrap();
        assert!(!walk.is_empty());
        for i in 0..walk.len() {
            assert!(seen.insert(walk[i]));
            assert_eq!(half[walk[i]].next(), walk[(i + 1) % walk.len()]);
        }
    }
    assert_eq!(seen, (0..half.len()).collect());
    assert!(
        embedding
            .boundary_walk(embedding.boundary_walk_count())
            .is_none()
    );
}

#[test]
fn random_graphs_match_independent_integer_rotation_oracle() {
    let mut state = 0x8ac4_3715_u64;
    let mut rand = |n: usize| {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((state >> 32) as usize) % n
    };
    for _ in 0..400 {
        let count = 1 + rand(6);
        let mut inputs = Vec::new();
        for i in 0..count {
            if i > 0 && rand(7) == 0 {
                let old: Segment2 = inputs[rand(i)];
                inputs.push(Segment2::try_new(old.end(), old.start()).unwrap());
                continue;
            }
            let a = (rand(9) as i64 - 4, rand(9) as i64 - 4);
            let mut b = (rand(9) as i64 - 4, rand(9) as i64 - 4);
            if a == b {
                b.0 += 1;
            }
            inputs.push(segment(a, b));
        }
        check_embedding(&graph(&inputs));
    }
}

#[test]
fn square_diagonal_walks_and_winding_signs() {
    let mut inputs = square(0, 0, 2);
    inputs.push(segment((0, 0), (2, 2)));
    let graph = graph(&inputs);
    let embedding = embed_arrangement2(&graph, limits()).unwrap();
    assert_eq!(embedding.boundary_walk_count(), 3);
    let mut areas = Vec::new();
    for index in 0..3 {
        let walk = embedding.boundary_walk(index).unwrap();
        let mut area = 0_i128;
        for &h in walk {
            let he = embedding.half_edges()[h];
            let coord = |vertex: usize| {
                let p = graph.vertices()[vertex].point().point_enclosure();
                assert_eq!(p.x().lower(), p.x().upper());
                assert_eq!(p.y().lower(), p.y().upper());
                (p.x().lower() as i128, p.y().lower() as i128)
            };
            let a = coord(he.origin());
            let b = coord(he.destination());
            area += a.0 * b.1 - a.1 * b.0;
        }
        areas.push(area);
        let orientation = simple_walk_orientation2(
            &embedding,
            index,
            WalkOrientationLimits2 {
                max_steps: 10,
                max_comparisons: 100,
            },
        )
        .unwrap();
        assert_eq!(
            orientation,
            if area < 0 {
                Orientation2::Clockwise
            } else {
                Orientation2::Counterclockwise
            }
        );
    }
    areas.sort();
    assert_eq!(areas, [-8, 4, 4]);
    check_embedding(&graph);
}

#[test]
fn disconnected_walks_and_bridges_are_not_faces() {
    let mut inputs = square(0, 0, 2);
    inputs.extend(square(4, 0, 2));
    let disconnected = graph(&inputs);
    let embedding = embed_arrangement2(&disconnected, limits()).unwrap();
    assert_eq!(embedding.boundary_walk_count(), 4);
    check_embedding(&disconnected);
    let mut inputs = square(0, 0, 2);
    inputs.push(segment((2, 0), (3, 0)));
    let graph = graph(&inputs);
    let embedding = embed_arrangement2(&graph, limits()).unwrap();
    assert_eq!(embedding.boundary_walk_count(), 2);
    assert!((0..2).any(|w| matches!(
        simple_walk_orientation2(
            &embedding,
            w,
            WalkOrientationLimits2 {
                max_steps: 20,
                max_comparisons: 100
            }
        ),
        Err(WalkOrientationError2::NotSimple)
    )));
    let bridge = graph
        .edge_uses()
        .iter()
        .find(|u| u.source() == 4)
        .unwrap()
        .edge();
    let a = bridge * 2;
    let b = a + 1;
    assert!((0..embedding.boundary_walk_count()).any(|w| {
        let walk = embedding.boundary_walk(w).unwrap();
        walk.contains(&a) && walk.contains(&b)
    }));
    check_embedding(&graph);
}

#[test]
fn simple_walk_orientation_limits_and_extreme_triangle() {
    let square_graph = graph(&square(0, 0, 2));
    let embedding = embed_arrangement2(&square_graph, limits()).unwrap();
    let enough = WalkOrientationLimits2 {
        max_steps: 4,
        max_comparisons: 9,
    };
    assert!(simple_walk_orientation2(&embedding, 0, enough).is_ok());
    assert!(matches!(
        simple_walk_orientation2(
            &embedding,
            0,
            WalkOrientationLimits2 {
                max_steps: 3,
                ..enough
            }
        ),
        Err(WalkOrientationError2::LimitExceeded(
            WalkOrientationResource2::Steps
        ))
    ));
    assert!(matches!(
        simple_walk_orientation2(
            &embedding,
            0,
            WalkOrientationLimits2 {
                max_comparisons: 8,
                ..enough
            }
        ),
        Err(WalkOrientationError2::LimitExceeded(
            WalkOrientationResource2::Comparisons
        ))
    ));
    assert!(matches!(
        simple_walk_orientation2(&embedding, 2, enough),
        Err(WalkOrientationError2::UnknownWalk)
    ));
    assert!(simple_walk_orientation2(&embedding, 0, enough).is_ok());

    let big = f64::MAX;
    let tiny = f64::from_bits(1);
    let inputs = [
        raw((0., 0.), (big, 0.)),
        raw((big, 0.), (big, tiny)),
        raw((big, tiny), (0., 0.)),
    ];
    let extreme = graph(&inputs);
    let embedding = embed_arrangement2(&extreme, limits()).unwrap();
    let signs: BTreeSet<_> = (0..embedding.boundary_walk_count())
        .map(|w| {
            match simple_walk_orientation2(
                &embedding,
                w,
                WalkOrientationLimits2 {
                    max_steps: 10,
                    max_comparisons: 100,
                },
            )
            .unwrap()
            {
                Orientation2::Clockwise => -1,
                Orientation2::Counterclockwise => 1,
                Orientation2::Collinear => panic!("simple triangle cannot be collinear"),
            }
        })
        .collect();
    assert_eq!(signs, BTreeSet::from([-1, 1]));
}

#[test]
fn nested_components_remain_separate_boundary_walks() {
    let mut inputs = square(0, 0, 6);
    inputs.extend(square(2, 2, 2));
    let graph = graph(&inputs);
    let embedding = embed_arrangement2(&graph, limits()).unwrap();
    assert_eq!(embedding.boundary_walk_count(), 4);
    let mut clockwise = 0;
    let mut counterclockwise = 0;
    for w in 0..4 {
        match simple_walk_orientation2(
            &embedding,
            w,
            WalkOrientationLimits2 {
                max_steps: 10,
                max_comparisons: 100,
            },
        )
        .unwrap()
        {
            Orientation2::Clockwise => clockwise += 1,
            Orientation2::Counterclockwise => counterclockwise += 1,
            Orientation2::Collinear => panic!("simple square cannot be collinear"),
        }
    }
    assert_eq!((clockwise, counterclockwise), (2, 2));
}

#[test]
fn extreme_near_collinear_rays_have_exact_radial_order() {
    let big = f64::MAX;
    let tiny = f64::from_bits(1);
    let ends = [
        (big, 0.),
        (big, tiny),
        (tiny, big),
        (0., big),
        (-big, tiny),
        (-big, 0.),
        (0., -big),
        (big, -tiny),
    ];
    let inputs: Vec<_> = ends.iter().map(|&end| raw((0., 0.), end)).collect();
    let graph = graph(&inputs);
    let embedding = embed_arrangement2(&graph, limits()).unwrap();
    let center = graph
        .vertices()
        .iter()
        .position(|vertex| {
            let e = vertex.point().point_enclosure();
            e.x().lower() == 0. && e.x().upper() == 0. && e.y().lower() == 0. && e.y().upper() == 0.
        })
        .unwrap();
    let sources: Vec<_> = embedding
        .outgoing(center)
        .unwrap()
        .iter()
        .map(|&h| {
            let edge = embedding.half_edges()[h].edge();
            graph
                .edge_uses()
                .iter()
                .find(|use_| use_.edge() == edge)
                .unwrap()
                .source()
        })
        .collect();
    assert_eq!(sources, vec![0, 1, 2, 3, 4, 5, 6, 7]);
}

#[test]
fn limit_failures_leave_graph_and_prior_embedding_intact() {
    let empty = graph(&[]);
    let graph = graph(&square(0, 0, 2));
    let original = embed_arrangement2(&graph, limits()).unwrap();
    assert_eq!(
        (original.half_edges().len(), original.boundary_walk_count()),
        (8, 2)
    );
    for (expected, lim) in [
        (
            EmbeddingResource2::Vertices,
            EmbeddingLimits2 {
                max_vertices: 3,
                ..limits()
            },
        ),
        (
            EmbeddingResource2::HalfEdges,
            EmbeddingLimits2 {
                max_half_edges: 7,
                ..limits()
            },
        ),
        (
            EmbeddingResource2::EdgeUses,
            EmbeddingLimits2 {
                max_edge_uses: 3,
                ..limits()
            },
        ),
        (
            EmbeddingResource2::WalkSteps,
            EmbeddingLimits2 {
                max_walk_steps: 7,
                ..limits()
            },
        ),
        (
            EmbeddingResource2::Comparisons,
            EmbeddingLimits2 {
                max_comparisons: 0,
                ..limits()
            },
        ),
        (
            EmbeddingResource2::BoundaryWalks,
            EmbeddingLimits2 {
                max_boundary_walks: 1,
                ..limits()
            },
        ),
    ] {
        assert!(
            matches!(embed_arrangement2(&graph,lim),Err(EmbeddingError2::LimitExceeded(actual)) if actual==expected)
        );
        assert_eq!(
            (original.half_edges().len(), original.boundary_walk_count()),
            (8, 2)
        );
        assert_eq!((graph.vertices().len(), graph.edges().len()), (4, 4));
    }
    let exact = EmbeddingLimits2 {
        max_vertices: 4,
        max_half_edges: 8,
        max_edge_uses: 4,
        max_comparisons: 10_000,
        max_boundary_walks: 2,
        max_walk_steps: 8,
    };
    assert!(embed_arrangement2(&graph, exact).is_ok());
    let zero = EmbeddingLimits2 {
        max_vertices: 0,
        max_half_edges: 0,
        max_edge_uses: 0,
        max_comparisons: 0,
        max_boundary_walks: 0,
        max_walk_steps: 0,
    };
    let embedding = embed_arrangement2(&empty, zero).unwrap();
    assert_eq!(
        (
            embedding.half_edges().len(),
            embedding.boundary_walk_count()
        ),
        (0, 0)
    );
}
